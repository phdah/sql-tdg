//! SQL Semantic Protocol integration and protocol-driven test-data generation.

use std::collections::{BTreeMap, BTreeSet};

use rand_chacha::ChaCha8Rng;
use rand_core::{Rng, SeedableRng};
use std::error::Error;
use std::fmt;

use sql_semantic_protocol::{
    AnalysisBundle, ColumnExpression, ComparisonAssumption, ComparisonOperator, ComposedSemantics,
    ConfiguredSqlInput, DataType, DatasetRef, Expression, JoinKind, Predicate, ProtocolStatement,
    QueryStatement, RelationCatalog, RelationResolution, RelationSchema, ResolvedComposedSemantics,
    SqlInput, TransformationLayer, ValueDomain, analyze_configured_inputs_with_catalog,
    dialect_from_name,
};

use crate::case_coverage::{CaseCoverageFinding, cover_case_branches};
use crate::constraint_generation::{self, ColumnDomains};
use crate::generator::{ColumnPlan, GenerationDomain, Generator, GeneratorError};
use crate::protocol_value::{
    ProtocolValue, build_array, candidates, is_supported, rejected_candidates,
    value_satisfies_domain,
};
use crate::solver::SolverError;
use crate::table::{Table, TableError};
use crate::test_case::{BoundaryKind, GenerationBoundary};

/// Explicit selection of one terminal protocol outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutcomeSelector {
    /// Select a named terminal relation.
    Relation(String),
    /// Select an anonymous terminal outcome by its layer identifier.
    AnonymousLayer(String),
}

/// Matching and deliberately rejected rows generated for each relation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenerationRowCounts {
    matching: usize,
    rejected: usize,
}

impl GenerationRowCounts {
    /// Creates explicit matching and rejected row counts.
    pub fn new(matching: usize, rejected: usize) -> Result<Self, ProtocolGenerationError> {
        matching.checked_add(rejected).ok_or_else(|| {
            ProtocolGenerationError::InvalidRowConfiguration {
                message: "matching and rejected row counts overflow usize".to_owned(),
            }
        })?;
        Ok(Self { matching, rejected })
    }

    /// Creates an all-matching row configuration.
    pub const fn matching_only(rows: usize) -> Self {
        Self {
            matching: rows,
            rejected: 0,
        }
    }

    /// Splits a total row count using a rejected-row ratio in the inclusive range 0.0..=1.0.
    ///
    /// The rejected count is rounded to the nearest whole row and the remainder is matching.
    pub fn from_rejected_ratio(
        total_rows: usize,
        rejected_ratio: f64,
    ) -> Result<Self, ProtocolGenerationError> {
        if !rejected_ratio.is_finite() || !(0.0..=1.0).contains(&rejected_ratio) {
            return Err(ProtocolGenerationError::InvalidRowConfiguration {
                message: format!(
                    "rejected row ratio must be finite and between 0.0 and 1.0, got {rejected_ratio}"
                ),
            });
        }

        let total = u64::try_from(total_rows).map_err(|_| {
            ProtocolGenerationError::InvalidRowConfiguration {
                message: "total row count cannot be represented as u64".to_owned(),
            }
        })?;
        if total > (1_u64 << 53) {
            return Err(ProtocolGenerationError::InvalidRowConfiguration {
                message:
                    "ratio-based row counts are limited to integers exactly representable by f64"
                        .to_owned(),
            });
        }

        let rejected = ((total as f64) * rejected_ratio).round() as usize;
        Self::new(total_rows - rejected, rejected)
    }

    /// Returns the number of rows sampled inside every supplied scalar domain.
    pub const fn matching(self) -> usize {
        self.matching
    }

    /// Returns the number of rows with one deliberately violated scalar domain.
    pub const fn rejected(self) -> usize {
        self.rejected
    }

    /// Returns the total number of generated rows.
    pub const fn total(self) -> usize {
        self.matching + self.rejected
    }
}

/// Generation parameters for SQL analysis at an explicit boundary.
#[derive(Debug, Clone, Copy)]
pub struct SqlGenerationSettings<'a> {
    row_counts: GenerationRowCounts,
    seed: u64,
    assumptions: &'a [ComparisonAssumption],
}

impl<'a> SqlGenerationSettings<'a> {
    /// Construct explicit row counts, seed, and caller-attested comparison settings.
    pub const fn new(
        row_counts: GenerationRowCounts,
        seed: u64,
        assumptions: &'a [ComparisonAssumption],
    ) -> Self {
        Self {
            row_counts,
            seed,
            assumptions,
        }
    }
}

/// Arrow-backed generated source tables keyed by canonical relation identity.
pub struct GeneratedData {
    tables: BTreeMap<String, Table>,
    row_counts: GenerationRowCounts,
    case_coverage: Vec<CaseCoverageFinding>,
    unhonored_constraints: Vec<String>,
}

impl fmt::Debug for GeneratedData {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("GeneratedData")
            .field("relations", &self.tables.keys().collect::<Vec<_>>())
            .field("row_counts", &self.row_counts)
            .field("case_coverage", &self.case_coverage)
            .field("unhonored_constraints", &self.unhonored_constraints)
            .finish()
    }
}

impl GeneratedData {
    /// Return all generated source tables in deterministic relation order.
    pub fn tables(&self) -> &BTreeMap<String, Table> {
        &self.tables
    }

    /// Return one generated source table.
    pub fn table(&self, relation: &str) -> Option<&Table> {
        self.tables.get(relation)
    }

    /// Return the matching and deliberately rejected row counts for each generated relation.
    pub const fn row_counts(&self) -> GenerationRowCounts {
        self.row_counts
    }

    /// Coverage findings for output CASE expressions, including un-coverable branches.
    pub fn case_coverage(&self) -> &[CaseCoverageFinding] {
        &self.case_coverage
    }

    /// Relations whose declared constraints were not enforced because they were not generated.
    pub fn unhonored_constraints(&self) -> &[String] {
        &self.unhonored_constraints
    }
}

/// Errors returned while consuming protocol semantics or generating source data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolGenerationError {
    /// The requested SQL dialect is not supported by SQL Semantic Protocol.
    UnsupportedDialect {
        /// Dialect name supplied by the caller.
        dialect: String,
    },
    /// SQL Semantic Protocol could not analyze the SQL input.
    Analysis {
        /// Upstream analysis error.
        message: String,
    },
    /// Source schema metadata is invalid.
    InvalidSchemaMetadata {
        /// Upstream schema metadata error.
        message: String,
    },
    /// The analyzed bundle has no terminal outcome.
    NoTerminalOutcome,
    /// More than one terminal outcome exists and the caller did not select one.
    AmbiguousTerminalOutcome {
        /// Stable terminal outcome descriptions.
        candidates: Vec<String>,
    },
    /// Several terminal outcomes generated together have no shared dataset that satisfies all of them.
    ConflictingOutcomes {
        /// Stable descriptions of the outcomes whose conditions conflict.
        outcomes: Vec<String>,
        /// Explanation naming the conflicting source columns.
        message: String,
    },
    /// One terminal outcome cannot be generated while generating for all terminal outcomes.
    TerminalOutcome {
        /// Stable description of the failing outcome.
        outcome: String,
        /// Reason the outcome cannot be generated.
        source: Box<ProtocolGenerationError>,
    },
    /// The requested terminal outcome does not exist.
    UnknownTerminalOutcome {
        /// Requested outcome description.
        selector: String,
    },
    /// The selected terminal outcome has no corresponding transformation layer.
    MissingOutcomeLayer {
        /// Layer or relation identifying the outcome.
        outcome: String,
    },
    /// Semantic composition for the selected outcome is unresolved.
    UnresolvedComposition {
        /// Layer whose composition is unresolved.
        layer_id: String,
        /// Upstream reason and diagnostics.
        reason: String,
    },
    /// Resolved composition still contains a diagnostic that prevents safe generation.
    UnsupportedSemantics {
        /// Layer containing the diagnostic.
        layer_id: String,
        /// Stable diagnostic code.
        code: String,
        /// Diagnostic explanation.
        message: String,
    },
    /// Residual conditions that prevent guaranteed row matching.
    ResidualConditions {
        layer_id: String,
        conditions: Vec<String>,
    },
    /// Conditional comparisons that need explicit caller declarations.
    MissingComparisonAssumptions {
        layer_id: String,
        conditions: Vec<String>,
    },
    /// A CASE branch could not be covered with exact protocol-derived witness values.
    CaseCoverage {
        /// Layer, output column, and branch selecting the witness.
        location: String,
        /// Explanation, including unknown or unreachable branch semantics.
        message: String,
    },
    /// A physical source relation does not have declared schema metadata.
    MissingSourceSchema {
        /// Canonical source relation identity.
        relation: String,
    },
    /// An explicitly selected intermediate relation has no declared output schema metadata.
    MissingBoundarySchema {
        /// Exact protocol relation identity selected for materialization.
        relation: String,
    },
    /// A requested generation boundary cannot be materialized safely.
    InvalidGenerationBoundary {
        /// Selected terminal layer.
        layer_id: String,
        /// Explanation of the invalid or unsupported boundary.
        message: String,
    },
    /// A constrained protocol column is absent from declared source schema metadata.
    MissingSchemaColumn {
        /// Canonical source relation identity.
        relation: String,
        /// Constrained source column.
        column: String,
    },
    /// A protocol column domain cannot be mapped to exactly one source column.
    AmbiguousColumnDomain {
        /// Column name from the protocol domain.
        column: String,
    },
    /// A protocol domain is explicitly unknown.
    UnknownDomain {
        /// Source relation.
        relation: String,
        /// Source column.
        column: String,
        /// Upstream reason.
        reason: String,
    },
    /// No values satisfy the protocol domain.
    EmptyDomain {
        /// Source relation.
        relation: String,
        /// Source column.
        column: String,
    },
    /// A protocol literal cannot be represented by the declared source type.
    InvalidLiteral {
        /// Source relation.
        relation: String,
        /// Source column.
        column: String,
        /// Explanation of the incompatibility.
        message: String,
    },
    /// The declared source datatype cannot be generated without weakening its semantics.
    UnsupportedSourceType {
        /// Source relation.
        relation: String,
        /// Source column.
        column: String,
        /// Canonical protocol datatype.
        data_type: String,
    },
    /// Requested matching/rejected row counts or ratio are invalid.
    InvalidRowConfiguration {
        /// Explanation of the invalid configuration.
        message: String,
    },
    /// Rejected rows were requested but a relation has no safely complementable scalar domain.
    NoRejectableColumn {
        /// Canonical source relation identity.
        relation: String,
    },
    /// A protocol relationship cannot be coordinated safely by the generator.
    UnsupportedRelationship {
        /// Layer containing the relationship.
        layer_id: String,
        /// Explanation of the unsupported relationship shape.
        message: String,
    },
    /// Protocol relationship domains have no common matching value.
    UnsatisfiableRelationship {
        /// Stable relationship description.
        relationship: String,
    },
    /// Rejected rows were requested but no relationship can be broken in isolation.
    NoBreakableRelationship {
        /// Selected terminal layer.
        layer_id: String,
    },
    /// The protocol domain shape is not supported for the declared source type.
    UnsupportedDomain {
        /// Source relation.
        relation: String,
        /// Source column.
        column: String,
        /// Explanation of the unsupported mapping.
        message: String,
    },
    /// Protocol relation constraints cannot be honored or contain unsupported metadata.
    RelationConstraint {
        /// Canonical relation identity, or <bundle> for unattributed metadata.
        relation: String,
        /// Constraint diagnostic or unsatisfiable combination.
        message: String,
    },
    /// The typed solver rejected a protocol-derived domain.
    Solver {
        /// Source relation.
        relation: String,
        /// Source column.
        column: String,
        /// Solver error.
        source: SolverError,
    },
    /// Table construction failed.
    Table {
        /// Source relation.
        relation: String,
        /// Table error.
        source: TableError,
    },
    /// Deterministic value generation failed.
    Generator {
        /// Source relation.
        relation: String,
        /// Generator error.
        source: GeneratorError,
    },
}

impl fmt::Display for ProtocolGenerationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedDialect { dialect } => {
                write!(formatter, "unsupported SQL dialect {dialect:?}")
            }
            Self::Analysis { message } => write!(formatter, "protocol analysis failed: {message}"),
            Self::InvalidSchemaMetadata { message } => {
                write!(
                    formatter,
                    "invalid protocol source schema metadata: {message}"
                )
            }
            Self::NoTerminalOutcome => {
                formatter.write_str("protocol bundle has no terminal outcome")
            }
            Self::AmbiguousTerminalOutcome { candidates } => write!(
                formatter,
                "protocol bundle has multiple terminal outcomes; select one explicitly: {}",
                candidates.join(", ")
            ),
            Self::ConflictingOutcomes { outcomes, message } => write!(
                formatter,
                "terminal outcomes {} cannot share one generated dataset: {message}",
                outcomes.join(", ")
            ),
            Self::TerminalOutcome { outcome, source } => write!(
                formatter,
                "terminal outcome {outcome} cannot be generated: {source}"
            ),
            Self::UnknownTerminalOutcome { selector } => {
                write!(formatter, "unknown terminal outcome {selector}")
            }
            Self::MissingOutcomeLayer { outcome } => {
                write!(
                    formatter,
                    "terminal outcome {outcome} has no transformation layer"
                )
            }
            Self::UnresolvedComposition { layer_id, reason } => write!(
                formatter,
                "terminal layer {layer_id} has unresolved composed semantics: {reason}"
            ),
            Self::UnsupportedSemantics {
                layer_id,
                code,
                message,
            } => write!(
                formatter,
                "terminal layer {layer_id} contains unsupported semantics {code}: {message}"
            ),
            Self::ResidualConditions {
                layer_id,
                conditions,
            } => write!(
                formatter,
                "layer {layer_id} has residual protocol conditions: {}",
                conditions.join("; ")
            ),
            Self::MissingComparisonAssumptions {
                layer_id,
                conditions,
            } => write!(
                formatter,
                "layer {layer_id} requires declared comparison assumptions: {}",
                conditions.join("; ")
            ),
            Self::CaseCoverage { location, message } => {
                write!(
                    formatter,
                    "CASE branch {location} cannot be covered: {message}"
                )
            }
            Self::MissingSourceSchema { relation } => {
                write!(
                    formatter,
                    "missing source schema metadata for relation {relation:?}"
                )
            }
            Self::MissingBoundarySchema { relation } => write!(
                formatter,
                "missing output schema metadata for intermediate relation {relation:?}"
            ),
            Self::InvalidGenerationBoundary { layer_id, message } => write!(
                formatter,
                "cannot materialize generation boundary for layer {layer_id}: {message}"
            ),
            Self::MissingSchemaColumn { relation, column } => write!(
                formatter,
                "protocol constrains {relation}.{column} but the source schema does not declare it"
            ),
            Self::AmbiguousColumnDomain { column } => {
                write!(
                    formatter,
                    "column domain {column:?} cannot be mapped unambiguously"
                )
            }
            Self::UnknownDomain {
                relation,
                column,
                reason,
            } => write!(
                formatter,
                "protocol domain for {relation}.{column} is unknown: {reason}"
            ),
            Self::EmptyDomain { relation, column } => {
                write!(
                    formatter,
                    "protocol domain for {relation}.{column} is empty"
                )
            }
            Self::InvalidLiteral {
                relation,
                column,
                message,
            } => write!(
                formatter,
                "invalid protocol literal for {relation}.{column}: {message}"
            ),
            Self::UnsupportedSourceType {
                relation,
                column,
                data_type,
            } => write!(
                formatter,
                "unsupported source datatype {data_type} for {relation}.{column}"
            ),
            Self::InvalidRowConfiguration { message } => {
                write!(formatter, "invalid row configuration: {message}")
            }
            Self::NoRejectableColumn { relation } => write!(
                formatter,
                "relation {relation:?} has no constrained scalar column with a safe complement domain"
            ),
            Self::UnsupportedRelationship { layer_id, message } => write!(
                formatter,
                "layer {layer_id} contains a relationship that cannot be generated safely: {message}"
            ),
            Self::UnsatisfiableRelationship { relationship } => write!(
                formatter,
                "relationship {relationship} has no common non-null value within its protocol domains"
            ),
            Self::NoBreakableRelationship { layer_id } => write!(
                formatter,
                "layer {layer_id} has no relationship that can be broken without violating another supplied relationship or scalar domain"
            ),
            Self::UnsupportedDomain {
                relation,
                column,
                message,
            } => write!(
                formatter,
                "unsupported protocol domain for {relation}.{column}: {message}"
            ),
            Self::RelationConstraint { relation, message } => write!(
                formatter, "relation constraint on {relation}: {message}"
            ),
            Self::Solver {
                relation, column, ..
            } => write!(
                formatter,
                "could not prepare domain for {relation}.{column}"
            ),
            Self::Table { relation, .. } => {
                write!(
                    formatter,
                    "could not construct table for relation {relation:?}"
                )
            }
            Self::Generator { relation, .. } => {
                write!(
                    formatter,
                    "could not generate table for relation {relation:?}"
                )
            }
        }
    }
}

impl Error for ProtocolGenerationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Solver { source, .. } => Some(source),
            Self::Table { source, .. } => Some(source),
            Self::Generator { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// Analyze SQL through SQL Semantic Protocol and generate matching physical source rows.
pub fn generate_from_sql(
    sql: &str,
    dialect_name: &str,
    source_schemas: &[RelationSchema],
    rows: usize,
    seed: u64,
) -> Result<GeneratedData, ProtocolGenerationError> {
    generate_classified_from_sql(
        sql,
        dialect_name,
        source_schemas,
        GenerationRowCounts::matching_only(rows),
        seed,
    )
}

/// Analyze SQL through SQL Semantic Protocol and generate classified physical source rows.
pub fn generate_classified_from_sql(
    sql: &str,
    dialect_name: &str,
    source_schemas: &[RelationSchema],
    row_counts: GenerationRowCounts,
    seed: u64,
) -> Result<GeneratedData, ProtocolGenerationError> {
    let boundary = GenerationBoundary::physical_sources();
    generate_classified_from_sql_at_boundary(
        sql,
        dialect_name,
        source_schemas,
        None,
        &boundary,
        row_counts,
        seed,
    )
}

/// Analyze SQL with caller-attested comparison settings.
pub fn generate_classified_from_sql_with_assumptions(
    sql: &str,
    dialect_name: &str,
    source_schemas: &[RelationSchema],
    row_counts: GenerationRowCounts,
    seed: u64,
    assumptions: &[ComparisonAssumption],
) -> Result<GeneratedData, ProtocolGenerationError> {
    generate_classified_from_sql_at_boundary_with_assumptions(
        sql,
        dialect_name,
        source_schemas,
        None,
        &GenerationBoundary::physical_sources(),
        SqlGenerationSettings::new(row_counts, seed, assumptions),
    )
}

/// Analyze SQL and generate matching rows at an explicit protocol graph boundary.
pub fn generate_from_sql_at_boundary(
    sql: &str,
    dialect_name: &str,
    relation_schemas: &[RelationSchema],
    selector: Option<&OutcomeSelector>,
    boundary: &GenerationBoundary,
    rows: usize,
    seed: u64,
) -> Result<GeneratedData, ProtocolGenerationError> {
    generate_classified_from_sql_at_boundary(
        sql,
        dialect_name,
        relation_schemas,
        selector,
        boundary,
        GenerationRowCounts::matching_only(rows),
        seed,
    )
}

/// Analyze SQL and generate classified rows at an explicit protocol graph boundary.
pub fn generate_classified_from_sql_at_boundary(
    sql: &str,
    dialect_name: &str,
    relation_schemas: &[RelationSchema],
    selector: Option<&OutcomeSelector>,
    boundary: &GenerationBoundary,
    row_counts: GenerationRowCounts,
    seed: u64,
) -> Result<GeneratedData, ProtocolGenerationError> {
    generate_classified_from_sql_at_boundary_with_assumptions(
        sql,
        dialect_name,
        relation_schemas,
        selector,
        boundary,
        SqlGenerationSettings::new(row_counts, seed, &[]),
    )
}

/// Generate classified rows at a boundary with explicit comparison assumptions.
pub fn generate_classified_from_sql_at_boundary_with_assumptions(
    sql: &str,
    dialect_name: &str,
    relation_schemas: &[RelationSchema],
    selector: Option<&OutcomeSelector>,
    boundary: &GenerationBoundary,
    settings: SqlGenerationSettings<'_>,
) -> Result<GeneratedData, ProtocolGenerationError> {
    let dialect = dialect_from_name(dialect_name).ok_or_else(|| {
        ProtocolGenerationError::UnsupportedDialect {
            dialect: dialect_name.to_owned(),
        }
    })?;
    let input = SqlInput::inline(sql);
    let configured = [ConfiguredSqlInput::new(
        "input-0001",
        &input,
        dialect_name,
        dialect.as_ref(),
    )];
    let catalog = RelationCatalog::from_schemas(relation_schemas).map_err(|error| {
        ProtocolGenerationError::InvalidSchemaMetadata {
            message: error.to_string(),
        }
    })?;
    let mut bundle =
        analyze_configured_inputs_with_catalog(&configured, &catalog).map_err(|error| {
            ProtocolGenerationError::Analysis {
                message: error.to_string(),
            }
        })?;

    bundle.declare_comparison_assumptions(settings.assumptions);
    generate_classified_from_bundle_at_boundary(
        &bundle,
        selector,
        boundary,
        settings.row_counts,
        settings.seed,
    )
}

/// Generate matching physical source rows from one explicitly resolved terminal outcome.
pub fn generate_from_bundle(
    bundle: &AnalysisBundle,
    selector: Option<&OutcomeSelector>,
    rows: usize,
    seed: u64,
) -> Result<GeneratedData, ProtocolGenerationError> {
    generate_classified_from_bundle(
        bundle,
        selector,
        GenerationRowCounts::matching_only(rows),
        seed,
    )
}

/// Generate classified physical source rows from one explicitly resolved terminal outcome.
pub fn generate_classified_from_bundle(
    bundle: &AnalysisBundle,
    selector: Option<&OutcomeSelector>,
    row_counts: GenerationRowCounts,
    seed: u64,
) -> Result<GeneratedData, ProtocolGenerationError> {
    let boundary = GenerationBoundary::physical_sources();
    generate_classified_from_bundle_at_boundary(bundle, selector, &boundary, row_counts, seed)
}

/// Generate matching rows at an explicit boundary of one selected terminal outcome.
pub fn generate_from_bundle_at_boundary(
    bundle: &AnalysisBundle,
    selector: Option<&OutcomeSelector>,
    boundary: &GenerationBoundary,
    rows: usize,
    seed: u64,
) -> Result<GeneratedData, ProtocolGenerationError> {
    generate_classified_from_bundle_at_boundary(
        bundle,
        selector,
        boundary,
        GenerationRowCounts::matching_only(rows),
        seed,
    )
}

/// Generate classified rows at an explicit boundary of the selected terminal outcome.
///
/// Without a selector, a bundle with several terminal outcomes generates one shared set of
/// physical source tables whose rows satisfy every outcome at once. Intermediate boundaries
/// always require a single terminal outcome.
pub fn generate_classified_from_bundle_at_boundary(
    bundle: &AnalysisBundle,
    selector: Option<&OutcomeSelector>,
    boundary: &GenerationBoundary,
    row_counts: GenerationRowCounts,
    seed: u64,
) -> Result<GeneratedData, ProtocolGenerationError> {
    if selector.is_none() && boundary.kind() == BoundaryKind::PhysicalSources {
        let finals = terminal_outcomes(bundle);
        if finals.len() > 1 {
            return generate_all_outcomes_data(bundle, &finals, row_counts, seed);
        }
    }

    let (layer_id, outcome_description) = select_terminal_layer(bundle, selector)?;
    let layer = bundle
        .layers()
        .iter()
        .find(|layer| layer.id() == layer_id)
        .ok_or(ProtocolGenerationError::MissingOutcomeLayer {
            outcome: outcome_description,
        })?;

    let semantics = resolved_layer_semantics(layer)?;
    require_exact_conditions(layer.id(), semantics)?;

    if let Some(diagnostic) = semantics.diagnostics().first() {
        return Err(ProtocolGenerationError::UnsupportedSemantics {
            layer_id: layer.id().to_owned(),
            code: diagnostic.code().to_owned(),
            message: diagnostic.message().to_owned(),
        });
    }

    let schemas = bundle
        .source_schemas()
        .iter()
        .map(|schema| (schema.relation().to_owned(), schema))
        .collect::<BTreeMap<_, _>>();

    match boundary.kind() {
        BoundaryKind::PhysicalSources => {
            validate_composed_domains(semantics, &schemas)?;
            let relationships = collect_equality_relationships(layer.id(), semantics, &schemas)?;

            if relationships.is_empty() {
                generate_scalar_data(bundle, layer.id(), semantics, &schemas, row_counts, seed)
            } else {
                generate_relational_data(
                    bundle,
                    layer.id(),
                    semantics,
                    &schemas,
                    &relationships,
                    row_counts,
                    seed,
                )
            }
        }
        BoundaryKind::IntermediateRelations => {
            generate_intermediate_boundary_data(bundle, layer, &schemas, boundary, row_counts, seed)
        }
    }
}

fn stable_reason<T: fmt::Debug>(value: T) -> String {
    let mut result = String::new();
    for (index, ch) in format!("{value:?}").chars().enumerate() {
        if index > 0 && ch.is_ascii_uppercase() {
            result.push('_');
        }
        result.push(ch.to_ascii_lowercase());
    }
    result
}

fn require_exact_conditions(
    layer_id: &str,
    semantics: &ResolvedComposedSemantics,
) -> Result<(), ProtocolGenerationError> {
    let exactness = semantics.condition_exactness();
    let residuals = exactness
        .residual_conditions()
        .iter()
        .map(|condition| {
            format!(
                "reason={} clause={} layer={} scope={} condition={}",
                stable_reason(condition.reason()),
                stable_reason(condition.clause()),
                condition.origin_layer_id().unwrap_or(layer_id),
                condition.origin_scope().unwrap_or("query"),
                condition.identity(),
            )
        })
        .collect::<Vec<_>>();
    if !residuals.is_empty() {
        return Err(ProtocolGenerationError::ResidualConditions {
            layer_id: layer_id.to_owned(),
            conditions: residuals,
        });
    }
    let missing = exactness
        .required_assumptions()
        .iter()
        .filter(|condition| {
            !exactness
                .declared_assumptions()
                .contains(&condition.assumption())
        })
        .map(|condition| {
            format!(
                "assumption={} clause={} layer={} scope={} condition={}",
                condition.assumption().as_str(),
                stable_reason(condition.clause()),
                condition.origin_layer_id().unwrap_or(layer_id),
                condition.origin_scope().unwrap_or("query"),
                condition.identity(),
            )
        })
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(ProtocolGenerationError::MissingComparisonAssumptions {
            layer_id: layer_id.to_owned(),
            conditions: missing,
        });
    }
    Ok(())
}

fn resolved_layer_semantics(
    layer: &TransformationLayer,
) -> Result<&ResolvedComposedSemantics, ProtocolGenerationError> {
    match layer.composed_semantics() {
        ComposedSemantics::Resolved(semantics) => Ok(semantics),
        ComposedSemantics::Unresolved(unresolved) => {
            Err(ProtocolGenerationError::UnresolvedComposition {
                layer_id: layer.id().to_owned(),
                reason: format!("{:?}", unresolved.reason()),
            })
        }
        _ => Err(ProtocolGenerationError::UnresolvedComposition {
            layer_id: layer.id().to_owned(),
            reason: "unsupported composed semantics variant".to_owned(),
        }),
    }
}

fn generate_intermediate_boundary_data(
    bundle: &AnalysisBundle,
    target_layer: &TransformationLayer,
    schemas: &BTreeMap<String, &RelationSchema>,
    boundary: &GenerationBoundary,
    row_counts: GenerationRowCounts,
    seed: u64,
) -> Result<GeneratedData, ProtocolGenerationError> {
    let target_query = query_for_layer(bundle, target_layer)?;
    let selected = boundary
        .relations()
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let direct_edges = bundle
        .graph()
        .edges()
        .iter()
        .filter(|edge| edge.consumer_layer_id() == target_layer.id())
        .collect::<Vec<_>>();

    if direct_edges.is_empty() {
        return Err(ProtocolGenerationError::InvalidGenerationBoundary {
            layer_id: target_layer.id().to_owned(),
            message: "selected target has no direct relation boundary to materialize".to_owned(),
        });
    }

    let mut direct_relations = BTreeSet::new();
    let mut producers = BTreeMap::new();
    for edge in direct_edges {
        direct_relations.insert(edge.relation().to_owned());
        if edge.resolution() != RelationResolution::Resolved {
            return Err(ProtocolGenerationError::InvalidGenerationBoundary {
                layer_id: target_layer.id().to_owned(),
                message: format!(
                    "direct dependency {:?} has resolution {:?}; intermediate boundaries require a fully defined in-bundle producer",
                    edge.relation(),
                    edge.resolution()
                ),
            });
        }
        let [producer_id] = edge.producer_layer_ids() else {
            return Err(ProtocolGenerationError::InvalidGenerationBoundary {
                layer_id: target_layer.id().to_owned(),
                message: format!(
                    "direct dependency {:?} does not resolve to exactly one producer",
                    edge.relation()
                ),
            });
        };
        let producer = bundle
            .layers()
            .iter()
            .find(|layer| layer.id() == producer_id)
            .ok_or_else(|| ProtocolGenerationError::InvalidGenerationBoundary {
                layer_id: target_layer.id().to_owned(),
                message: format!("producer layer {producer_id:?} is missing"),
            })?;
        let Some(write_kind) = producer.write_kind() else {
            return Err(ProtocolGenerationError::InvalidGenerationBoundary {
                layer_id: target_layer.id().to_owned(),
                message: format!(
                    "producer layer {:?} does not expose a complete relation-write contract",
                    producer.id()
                ),
            });
        };
        if !write_kind.fully_defines_relation() {
            return Err(ProtocolGenerationError::InvalidGenerationBoundary {
                layer_id: target_layer.id().to_owned(),
                message: format!(
                    "relation {:?} is produced by partial write kind {write_kind:?}",
                    edge.relation()
                ),
            });
        }
        producers.insert(edge.relation().to_owned(), producer);
    }

    if selected != direct_relations {
        let missing = direct_relations
            .difference(&selected)
            .cloned()
            .collect::<Vec<_>>();
        let unexpected = selected
            .difference(&direct_relations)
            .cloned()
            .collect::<Vec<_>>();
        return Err(ProtocolGenerationError::InvalidGenerationBoundary {
            layer_id: target_layer.id().to_owned(),
            message: format!(
                "intermediate boundary must exactly cover direct target inputs; missing={missing:?}, unexpected={unexpected:?}"
            ),
        });
    }

    let relationships =
        collect_boundary_relationships(target_layer.id(), target_query, &selected, schemas)?;
    let include_rejected_domains = relationships.is_empty() && row_counts.rejected() > 0;
    let mut plans_by_relation = BTreeMap::new();
    let mut candidates_by_column = BTreeMap::new();

    for relation in boundary.relations() {
        let schema = schemas.get(relation).copied().ok_or_else(|| {
            ProtocolGenerationError::MissingBoundarySchema {
                relation: relation.clone(),
            }
        })?;
        let producer = producers.get(relation).copied().ok_or_else(|| {
            ProtocolGenerationError::InvalidGenerationBoundary {
                layer_id: target_layer.id().to_owned(),
                message: format!("boundary relation {relation:?} has no resolved producer"),
            }
        })?;
        let producer_semantics = resolved_layer_semantics(producer)?;
        require_exact_conditions(producer.id(), producer_semantics)?;
        if let Some(diagnostic) = producer_semantics.diagnostics().first() {
            return Err(ProtocolGenerationError::UnsupportedSemantics {
                layer_id: producer.id().to_owned(),
                code: diagnostic.code().to_owned(),
                message: diagnostic.message().to_owned(),
            });
        }

        validate_intermediate_schema(target_layer.id(), relation, schema, producer_semantics)?;
        let plans = prepare_intermediate_relation_plans(
            relation,
            schema,
            producer_semantics,
            target_query,
            include_rejected_domains,
            &mut candidates_by_column,
        )?;
        plans_by_relation.insert(relation.clone(), plans);
    }

    if relationships.is_empty() {
        generate_prepared_scalar_data(
            boundary.relations(),
            schemas,
            &plans_by_relation,
            row_counts,
            seed,
        )
    } else {
        let relationship_plan = PreparedRelationships {
            candidates_by_column: &candidates_by_column,
            relationships: &relationships,
        };
        generate_prepared_relational_data(
            target_layer.id(),
            boundary.relations(),
            schemas,
            &plans_by_relation,
            &relationship_plan,
            row_counts,
            seed,
        )
    }
}

fn validate_intermediate_schema(
    target_layer_id: &str,
    relation: &str,
    schema: &RelationSchema,
    producer_semantics: &ResolvedComposedSemantics,
) -> Result<(), ProtocolGenerationError> {
    let declared = schema
        .columns()
        .iter()
        .map(|column| column.name())
        .collect::<Vec<_>>();
    let produced = producer_semantics
        .output()
        .columns()
        .iter()
        .map(|column| column.name())
        .collect::<Vec<_>>();

    if declared != produced {
        return Err(ProtocolGenerationError::InvalidGenerationBoundary {
            layer_id: target_layer_id.to_owned(),
            message: format!(
                "declared output schema for {relation:?} does not match producer output columns: declared={declared:?}, produced={produced:?}"
            ),
        });
    }

    Ok(())
}

fn prepare_intermediate_relation_plans(
    relation: &str,
    schema: &RelationSchema,
    producer_semantics: &ResolvedComposedSemantics,
    target_query: &QueryStatement,
    include_rejected_domains: bool,
    candidates_by_column: &mut BTreeMap<RelationshipColumn, Vec<ProtocolValue>>,
) -> Result<Vec<ColumnPlan>, ProtocolGenerationError> {
    let mut plans = Vec::with_capacity(schema.columns().len());

    for schema_column in schema.columns() {
        is_supported(schema_column.data_type()).map_err(|_| {
            unsupported_source_type(relation, schema_column.name(), schema_column.data_type())
        })?;

        let produced = producer_semantics
            .output()
            .columns()
            .iter()
            .filter(|column| column.name() == schema_column.name())
            .collect::<Vec<_>>();
        let [produced] = produced.as_slice() else {
            return Err(ProtocolGenerationError::InvalidGenerationBoundary {
                layer_id: relation.to_owned(),
                message: format!(
                    "producer output does not contain exactly one column named {:?}",
                    schema_column.name()
                ),
            });
        };

        let downstream = find_column_domain(
            target_query.column_domains(),
            relation,
            schema_column.name(),
        )?;
        let matching_values = intersect_boundary_candidates(
            relation,
            schema_column.name(),
            schema_column.data_type(),
            produced.domain(),
            downstream,
        )?;
        let rejected_values = if include_rejected_domains {
            boundary_rejected_candidates(
                relation,
                schema_column.name(),
                schema_column.data_type(),
                produced.domain(),
                downstream,
            )?
        } else {
            Vec::new()
        };

        candidates_by_column.insert(
            RelationshipColumn::new(relation, schema_column.name()),
            matching_values.clone(),
        );
        let unconstrained = matches!(produced.domain(), ValueDomain::Unbounded)
            && matches!(downstream, None | Some(ValueDomain::Unbounded));
        let matching_domain = if unconstrained {
            GenerationDomain::Unconstrained {
                data_type: schema_column.data_type().clone(),
            }
        } else {
            GenerationDomain::Values(matching_values)
        };
        plans.push(ColumnPlan::new(
            schema_column.name(),
            matching_domain,
            (!rejected_values.is_empty()).then_some(GenerationDomain::Values(rejected_values)),
        ));
    }

    Ok(plans)
}

fn intersect_boundary_candidates(
    relation: &str,
    column: &str,
    data_type: &DataType,
    upstream: &ValueDomain,
    downstream: Option<&ValueDomain>,
) -> Result<Vec<ProtocolValue>, ProtocolGenerationError> {
    validate_boundary_domain(relation, column, upstream)?;
    if let Some(domain) = downstream {
        validate_boundary_domain(relation, column, domain)?;
    }

    let mut pool = candidates(data_type, Some(upstream)).map_err(|message| {
        ProtocolGenerationError::UnsupportedDomain {
            relation: relation.to_owned(),
            column: column.to_owned(),
            message,
        }
    })?;
    if let Some(domain) = downstream {
        for candidate in candidates(data_type, Some(domain)).map_err(|message| {
            ProtocolGenerationError::UnsupportedDomain {
                relation: relation.to_owned(),
                column: column.to_owned(),
                message,
            }
        })? {
            if !pool.contains(&candidate) {
                pool.push(candidate);
            }
        }
    }

    let mut matching = Vec::new();
    for candidate in pool {
        let upstream_match =
            value_satisfies_domain(data_type, &candidate, upstream).map_err(|message| {
                ProtocolGenerationError::UnsupportedDomain {
                    relation: relation.to_owned(),
                    column: column.to_owned(),
                    message,
                }
            })?;
        let downstream_match = match downstream {
            Some(domain) => {
                value_satisfies_domain(data_type, &candidate, domain).map_err(|message| {
                    ProtocolGenerationError::UnsupportedDomain {
                        relation: relation.to_owned(),
                        column: column.to_owned(),
                        message,
                    }
                })?
            }
            None => true,
        };
        if upstream_match && downstream_match && !matching.contains(&candidate) {
            matching.push(candidate);
        }
    }

    if matching.is_empty() {
        return Err(ProtocolGenerationError::EmptyDomain {
            relation: relation.to_owned(),
            column: column.to_owned(),
        });
    }

    Ok(matching)
}

fn boundary_rejected_candidates(
    relation: &str,
    column: &str,
    data_type: &DataType,
    upstream: &ValueDomain,
    downstream: Option<&ValueDomain>,
) -> Result<Vec<ProtocolValue>, ProtocolGenerationError> {
    let Some(downstream) = downstream else {
        return Ok(Vec::new());
    };
    validate_boundary_domain(relation, column, downstream)?;

    let values = rejected_candidates(data_type, downstream).map_err(|message| {
        ProtocolGenerationError::UnsupportedDomain {
            relation: relation.to_owned(),
            column: column.to_owned(),
            message: format!("cannot derive boundary rejected values: {message}"),
        }
    })?;
    let mut result = Vec::new();
    for value in values {
        if value_satisfies_domain(data_type, &value, upstream).map_err(|message| {
            ProtocolGenerationError::UnsupportedDomain {
                relation: relation.to_owned(),
                column: column.to_owned(),
                message,
            }
        })? {
            result.push(value);
        }
    }
    Ok(result)
}

fn validate_boundary_domain(
    relation: &str,
    column: &str,
    domain: &ValueDomain,
) -> Result<(), ProtocolGenerationError> {
    match domain {
        ValueDomain::Empty => Err(ProtocolGenerationError::EmptyDomain {
            relation: relation.to_owned(),
            column: column.to_owned(),
        }),
        ValueDomain::Unknown(unknown) => Err(ProtocolGenerationError::UnknownDomain {
            relation: relation.to_owned(),
            column: column.to_owned(),
            reason: unknown.reason().to_owned(),
        }),
        _ => Ok(()),
    }
}

fn collect_boundary_relationships(
    layer_id: &str,
    query: &QueryStatement,
    selected: &BTreeSet<String>,
    schemas: &BTreeMap<String, &RelationSchema>,
) -> Result<Vec<EqualityRelationship>, ProtocolGenerationError> {
    reject_unsupported_relational_predicates(layer_id, query)?;
    let mut relationships = BTreeSet::new();

    for join in query.joins() {
        if join.kind() == JoinKind::Cross && join.condition().is_none() {
            continue;
        }
        if join.kind() != JoinKind::Inner {
            return Err(ProtocolGenerationError::UnsupportedRelationship {
                layer_id: layer_id.to_owned(),
                message: format!(
                    "join kind {:?} does not define the required matching/non-matching witness contract",
                    join.kind()
                ),
            });
        }

        let condition =
            join.condition()
                .ok_or_else(|| ProtocolGenerationError::UnsupportedRelationship {
                    layer_id: layer_id.to_owned(),
                    message: "inner join has no resolved condition".to_owned(),
                })?;
        let mut equalities = Vec::new();
        collect_column_equalities(layer_id, condition, &mut equalities)?;

        for (left_expression, right_expression) in equalities {
            let left_relation = source_relation_for_column(layer_id, query, &left_expression)?;
            let right_relation = source_relation_for_column(layer_id, query, &right_expression)?;
            if !selected.contains(left_relation) || !selected.contains(right_relation) {
                return Err(ProtocolGenerationError::InvalidGenerationBoundary {
                    layer_id: layer_id.to_owned(),
                    message: "join relationship crosses outside the selected intermediate boundary"
                        .to_owned(),
                });
            }

            let left = RelationshipColumn::new(left_relation, left_expression.name());
            let right = RelationshipColumn::new(right_relation, right_expression.name());
            if left == right {
                return Err(ProtocolGenerationError::UnsupportedRelationship {
                    layer_id: layer_id.to_owned(),
                    message: format!(
                        "relationship {} = {} resolves to the same boundary column",
                        left.describe(),
                        right.describe()
                    ),
                });
            }
            validate_relationship_types(layer_id, schemas, &left, &right)?;
            relationships.insert(EqualityRelationship::new(left, right));
        }
    }

    Ok(relationships.into_iter().collect())
}

fn generate_prepared_scalar_data(
    relations: &[String],
    schemas: &BTreeMap<String, &RelationSchema>,
    plans_by_relation: &BTreeMap<String, Vec<ColumnPlan>>,
    row_counts: GenerationRowCounts,
    seed: u64,
) -> Result<GeneratedData, ProtocolGenerationError> {
    let mut tables = BTreeMap::new();

    for relation in relations {
        let schema = source_schema(schemas, relation).map_err(|_| {
            ProtocolGenerationError::MissingBoundarySchema {
                relation: relation.clone(),
            }
        })?;
        let plans = plans_by_relation.get(relation).ok_or_else(|| {
            ProtocolGenerationError::InvalidGenerationBoundary {
                layer_id: relation.clone(),
                message: "prepared boundary plan is missing".to_owned(),
            }
        })?;
        if row_counts.rejected() > 0 && !plans.iter().any(ColumnPlan::is_rejectable) {
            return Err(ProtocolGenerationError::NoRejectableColumn {
                relation: relation.clone(),
            });
        }

        let generated = Generator::new()
            .generate_classified_protocol_values(
                row_counts.matching(),
                row_counts.rejected(),
                seed,
                plans,
            )
            .map_err(|source| ProtocolGenerationError::Generator {
                relation: relation.clone(),
                source,
            })?;
        let table = build_protocol_table(relation, schema, row_counts.total(), generated)?;
        tables.insert(relation.clone(), table);
    }

    Ok(GeneratedData {
        tables,
        row_counts,
        case_coverage: Vec::new(),
    })
}

struct PreparedRelationships<'a> {
    candidates_by_column: &'a BTreeMap<RelationshipColumn, Vec<ProtocolValue>>,
    relationships: &'a [EqualityRelationship],
}

fn generate_prepared_relational_data(
    layer_id: &str,
    relations: &[String],
    schemas: &BTreeMap<String, &RelationSchema>,
    plans_by_relation: &BTreeMap<String, Vec<ColumnPlan>>,
    relationship_plan: &PreparedRelationships<'_>,
    row_counts: GenerationRowCounts,
    seed: u64,
) -> Result<GeneratedData, ProtocolGenerationError> {
    let mut generated_by_relation = BTreeMap::new();

    for relation in relations {
        let plans = plans_by_relation.get(relation).ok_or_else(|| {
            ProtocolGenerationError::InvalidGenerationBoundary {
                layer_id: layer_id.to_owned(),
                message: format!("prepared plan for relation {relation:?} is missing"),
            }
        })?;
        let generated = Generator::new()
            .generate_classified_protocol_values(row_counts.total(), 0, seed, plans)
            .map_err(|source| ProtocolGenerationError::Generator {
                relation: relation.clone(),
                source,
            })?;
        generated_by_relation.insert(relation.clone(), generated);
    }

    let adjacency = relationship_adjacency(relationship_plan.relationships);
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let matching_values =
        choose_component_values(relationship_plan.candidates_by_column, &adjacency, &mut rng)?;

    for (column, value) in &matching_values {
        for row in 0..row_counts.total() {
            set_generated_value(
                &mut generated_by_relation,
                schemas,
                column,
                row,
                value.clone(),
            )?;
        }
    }

    if row_counts.rejected() > 0 {
        let witnesses = relationship_witnesses(
            relationship_plan.relationships,
            relationship_plan.candidates_by_column,
            &adjacency,
            &matching_values,
            &mut rng,
        )?;
        if witnesses.is_empty() {
            return Err(ProtocolGenerationError::NoBreakableRelationship {
                layer_id: layer_id.to_owned(),
            });
        }

        for offset in 0..row_counts.rejected() {
            let witness_index =
                sample_relationship_index(&mut rng, witnesses.len(), "relationship witness")?;
            let witness = witnesses.get(witness_index).ok_or_else(|| {
                ProtocolGenerationError::InvalidRowConfiguration {
                    message: "selected relationship witness is missing".to_owned(),
                }
            })?;
            set_generated_value(
                &mut generated_by_relation,
                schemas,
                &witness.column,
                row_counts.matching() + offset,
                witness.value.clone(),
            )?;
        }
    }

    let mut tables = BTreeMap::new();
    for relation in relations {
        let schema = schemas.get(relation).copied().ok_or_else(|| {
            ProtocolGenerationError::MissingBoundarySchema {
                relation: relation.clone(),
            }
        })?;
        let generated = generated_by_relation.remove(relation).ok_or_else(|| {
            ProtocolGenerationError::InvalidGenerationBoundary {
                layer_id: layer_id.to_owned(),
                message: format!("generated relation {relation:?} is missing"),
            }
        })?;
        let table = build_protocol_table(relation, schema, row_counts.total(), generated)?;
        tables.insert(relation.clone(), table);
    }

    Ok(GeneratedData {
        tables,
        row_counts,
        case_coverage: Vec::new(),
    })
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct RelationshipColumn {
    relation: String,
    column: String,
}

impl RelationshipColumn {
    fn new(relation: impl Into<String>, column: impl Into<String>) -> Self {
        Self {
            relation: relation.into(),
            column: column.into(),
        }
    }

    fn describe(&self) -> String {
        format!("{}.{}", self.relation, self.column)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct EqualityRelationship {
    left: RelationshipColumn,
    right: RelationshipColumn,
}

impl EqualityRelationship {
    fn new(left: RelationshipColumn, right: RelationshipColumn) -> Self {
        if left <= right {
            Self { left, right }
        } else {
            Self {
                left: right,
                right: left,
            }
        }
    }

    fn describe(&self) -> String {
        format!("{} = {}", self.left.describe(), self.right.describe())
    }
}

#[derive(Debug, Clone)]
struct RelationshipWitness {
    column: RelationshipColumn,
    value: ProtocolValue,
}

fn validate_composed_domains(
    semantics: &ResolvedComposedSemantics,
    schemas: &BTreeMap<String, &RelationSchema>,
) -> Result<(), ProtocolGenerationError> {
    for domain in semantics.column_domains() {
        let Some(relation) = domain.column().relation() else {
            return Err(ProtocolGenerationError::UnsupportedDomain {
                relation: "<unresolved>".to_owned(),
                column: domain.column().name().to_owned(),
                message: "composed source-column domain has no physical relation".to_owned(),
            });
        };
        if !semantics
            .dependencies()
            .iter()
            .any(|dependency| dependency == relation)
        {
            return Err(ProtocolGenerationError::UnsupportedDomain {
                relation: relation.to_owned(),
                column: domain.column().name().to_owned(),
                message: "domain relation is not a composed physical dependency".to_owned(),
            });
        }
        let schema = schemas.get(relation).copied().ok_or_else(|| {
            ProtocolGenerationError::MissingSourceSchema {
                relation: relation.to_owned(),
            }
        })?;
        if !schema
            .columns()
            .iter()
            .any(|column| column.name() == domain.column().name())
        {
            return Err(ProtocolGenerationError::MissingSchemaColumn {
                relation: relation.to_owned(),
                column: domain.column().name().to_owned(),
            });
        }
    }

    Ok(())
}

fn generate_scalar_data(
    bundle: &AnalysisBundle,
    layer_id: &str,
    semantics: &ResolvedComposedSemantics,
    schemas: &BTreeMap<String, &RelationSchema>,
    row_counts: GenerationRowCounts,
    seed: u64,
) -> Result<GeneratedData, ProtocolGenerationError> {
    let mut generated_by_relation = BTreeMap::new();

    for relation in semantics.dependencies() {
        let schema = source_schema(schemas, relation)?;
        let plans = prepare_relation_plans(semantics, relation, schema, row_counts.rejected() > 0)?;

        if row_counts.rejected() > 0 && !plans.iter().any(ColumnPlan::is_rejectable) {
            return Err(ProtocolGenerationError::NoRejectableColumn {
                relation: relation.clone(),
            });
        }

        let generated = Generator::new()
            .generate_classified_protocol_values(
                row_counts.matching(),
                row_counts.rejected(),
                seed,
                &plans,
            )
            .map_err(|source| ProtocolGenerationError::Generator {
                relation: relation.clone(),
                source,
            })?;

        generated_by_relation.insert(relation.clone(), generated);
    }

    let case_coverage = cover_case_branches(
        bundle,
        &[layer_id.to_owned()],
        &[semantics],
        schemas,
        &mut generated_by_relation,
        row_counts.matching(),
        &BTreeSet::new(),
    )?;
    let mut tables = BTreeMap::new();
    for relation in semantics.dependencies() {
        let schema = source_schema(schemas, relation)?;
        let generated = generated_by_relation.remove(relation).ok_or_else(|| {
            ProtocolGenerationError::MissingSourceSchema {
                relation: relation.clone(),
            }
        })?;
        tables.insert(
            relation.clone(),
            build_protocol_table(relation, schema, row_counts.total(), generated)?,
        );
    }
    Ok(GeneratedData {
        tables,
        row_counts,
        case_coverage,
    })
}

fn generate_relational_data(
    bundle: &AnalysisBundle,
    layer_id: &str,
    semantics: &ResolvedComposedSemantics,
    schemas: &BTreeMap<String, &RelationSchema>,
    relationships: &[EqualityRelationship],
    row_counts: GenerationRowCounts,
    seed: u64,
) -> Result<GeneratedData, ProtocolGenerationError> {
    let mut generated_by_relation = BTreeMap::new();

    for relation in semantics.dependencies() {
        let schema = source_schema(schemas, relation)?;
        let plans = prepare_relation_plans(semantics, relation, schema, false)?;
        let generated = Generator::new()
            .generate_classified_protocol_values(row_counts.total(), 0, seed, &plans)
            .map_err(|source| ProtocolGenerationError::Generator {
                relation: relation.clone(),
                source,
            })?;
        generated_by_relation.insert(relation.clone(), generated);
    }

    let candidates_by_column = relationship_candidates(semantics, schemas, relationships)?;
    let adjacency = relationship_adjacency(relationships);
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let matching_values = choose_component_values(&candidates_by_column, &adjacency, &mut rng)?;

    for (column, value) in &matching_values {
        for row in 0..row_counts.total() {
            set_generated_value(
                &mut generated_by_relation,
                schemas,
                column,
                row,
                value.clone(),
            )?;
        }
    }

    if row_counts.rejected() > 0 {
        let witnesses = relationship_witnesses(
            relationships,
            &candidates_by_column,
            &adjacency,
            &matching_values,
            &mut rng,
        )?;
        if witnesses.is_empty() {
            return Err(ProtocolGenerationError::NoBreakableRelationship {
                layer_id: layer_id.to_owned(),
            });
        }

        for offset in 0..row_counts.rejected() {
            let witness_index =
                sample_relationship_index(&mut rng, witnesses.len(), "relationship witness")?;
            let witness = witnesses.get(witness_index).ok_or_else(|| {
                ProtocolGenerationError::InvalidRowConfiguration {
                    message: "selected relationship witness is missing".to_owned(),
                }
            })?;
            set_generated_value(
                &mut generated_by_relation,
                schemas,
                &witness.column,
                row_counts.matching() + offset,
                witness.value.clone(),
            )?;
        }
    }

    let keys = relationships
        .iter()
        .flat_map(|join| [&join.left, &join.right])
        .map(|column| (column.relation.clone(), column.column.clone()))
        .collect::<BTreeSet<_>>();
    let case_coverage = cover_case_branches(
        bundle,
        &[layer_id.to_owned()],
        &[semantics],
        schemas,
        &mut generated_by_relation,
        row_counts.matching(),
        &keys,
    )?;
    let mut tables = BTreeMap::new();
    for relation in semantics.dependencies() {
        let schema = source_schema(schemas, relation)?;
        let generated = generated_by_relation.remove(relation).ok_or_else(|| {
            ProtocolGenerationError::MissingSourceSchema {
                relation: relation.clone(),
            }
        })?;
        let table = build_protocol_table(relation, schema, row_counts.total(), generated)?;
        tables.insert(relation.clone(), table);
    }

    Ok(GeneratedData {
        tables,
        row_counts,
        case_coverage,
    })
}

/// One terminal outcome participating in shared all-outcomes generation.
struct OutcomeSemantics<'a> {
    layer_id: String,
    description: String,
    semantics: &'a ResolvedComposedSemantics,
}

/// Generate one set of physical source tables whose rows satisfy every terminal outcome.
///
/// Each source column takes the intersection of the domains every outcome places on it, and
/// relationship keys take a value allowed by every outcome. Conflicts are explicit errors.
fn generate_all_outcomes_data(
    bundle: &AnalysisBundle,
    finals: &[&DatasetRef],
    row_counts: GenerationRowCounts,
    seed: u64,
) -> Result<GeneratedData, ProtocolGenerationError> {
    if row_counts.rejected() > 0 {
        return Err(ProtocolGenerationError::InvalidRowConfiguration {
            message: "rejected rows are not defined when generating for all terminal outcomes; select one terminal outcome".to_owned(),
        });
    }

    let schemas = bundle
        .source_schemas()
        .iter()
        .map(|schema| (schema.relation().to_owned(), schema))
        .collect::<BTreeMap<_, _>>();

    let mut outcomes = Vec::with_capacity(finals.len());
    let mut relationships = BTreeSet::new();
    let mut outcomes_by_column = BTreeMap::<RelationshipColumn, BTreeSet<String>>::new();
    for outcome in finals {
        let (layer_id, description) = outcome_layer(bundle, outcome)?;
        let (semantics, outcome_relationships) =
            prepare_outcome(bundle, &layer_id, &description, &schemas).map_err(|source| {
                ProtocolGenerationError::TerminalOutcome {
                    outcome: description.clone(),
                    source: Box::new(source),
                }
            })?;

        for relationship in outcome_relationships {
            for column in [&relationship.left, &relationship.right] {
                outcomes_by_column
                    .entry(column.clone())
                    .or_default()
                    .insert(description.clone());
            }
            relationships.insert(relationship);
        }
        outcomes.push(OutcomeSemantics {
            layer_id,
            description,
            semantics,
        });
    }
    outcomes.sort_by(|left, right| left.description.cmp(&right.description));

    let relations = outcomes
        .iter()
        .flat_map(|outcome| outcome.semantics.dependencies().iter().cloned())
        .collect::<BTreeSet<_>>();
    let relationships = relationships.into_iter().collect::<Vec<_>>();
    let relationship_columns = relationships
        .iter()
        .flat_map(|relationship| [&relationship.left, &relationship.right])
        .cloned()
        .collect::<BTreeSet<_>>();

    let mut generated_by_relation = BTreeMap::new();
    let mut candidates_by_column = BTreeMap::new();
    let mut domains_by_column = BTreeMap::new();
    for relation in &relations {
        let schema = source_schema(&schemas, relation)?;
        let mut plans = Vec::with_capacity(schema.columns().len());
        for schema_column in schema.columns() {
            let column = RelationshipColumn::new(relation.as_str(), schema_column.name());
            let constraints = outcome_column_domains(&outcomes, relation, schema_column.name())?;
            for (description, _) in &constraints {
                outcomes_by_column
                    .entry(column.clone())
                    .or_default()
                    .insert((*description).to_owned());
            }
            let (plan, values) = shared_column_plan(relation, schema_column, &constraints)?;
            if relationship_columns.contains(&column) {
                let values = values
                    .into_iter()
                    .filter(|value| !matches!(value, ProtocolValue::Null))
                    .collect::<Vec<_>>();
                if values.is_empty() {
                    return Err(ProtocolGenerationError::UnsatisfiableRelationship {
                        relationship: column.describe(),
                    });
                }
                domains_by_column.insert(
                    column.clone(),
                    RelationshipDomains {
                        data_type: schema_column.data_type(),
                        domains: constraints.iter().map(|(_, domain)| *domain).collect(),
                    },
                );
                candidates_by_column.insert(column, values);
            }
            plans.push(plan);
        }

        let generated = Generator::new()
            .generate_classified_protocol_values(row_counts.matching(), 0, seed, &plans)
            .map_err(|source| ProtocolGenerationError::Generator {
                relation: relation.clone(),
                source,
            })?;
        generated_by_relation.insert(relation.clone(), generated);
    }

    if !relationships.is_empty() {
        let adjacency = relationship_adjacency(&relationships);
        let mut shared_candidates = BTreeMap::new();
        for component in relationship_components(&candidates_by_column, &adjacency) {
            let common =
                shared_component_values(&candidates_by_column, &domains_by_column, &component)?;
            if common.is_empty() {
                let involved = component
                    .iter()
                    .filter_map(|column| outcomes_by_column.get(column))
                    .flatten()
                    .cloned()
                    .collect::<BTreeSet<_>>();
                return Err(ProtocolGenerationError::ConflictingOutcomes {
                    outcomes: involved.into_iter().collect(),
                    message: format!(
                        "relationship {} has no common non-null value",
                        component
                            .iter()
                            .map(RelationshipColumn::describe)
                            .collect::<Vec<_>>()
                            .join(" = ")
                    ),
                });
            }
            for column in component {
                shared_candidates.insert(column, common.clone());
            }
        }

        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        let matching_values = choose_component_values(&shared_candidates, &adjacency, &mut rng)?;
        for (column, value) in &matching_values {
            for row in 0..row_counts.matching() {
                set_generated_value(
                    &mut generated_by_relation,
                    &schemas,
                    column,
                    row,
                    value.clone(),
                )?;
            }
        }
    }

    let roots = outcomes
        .iter()
        .map(|outcome| outcome.layer_id.clone())
        .collect::<Vec<_>>();
    let selected = outcomes
        .iter()
        .map(|outcome| outcome.semantics)
        .collect::<Vec<_>>();
    let keys = relationship_columns
        .iter()
        .map(|column| (column.relation.clone(), column.column.clone()))
        .collect::<BTreeSet<_>>();
    let case_coverage = cover_case_branches(
        bundle,
        &roots,
        &selected,
        &schemas,
        &mut generated_by_relation,
        row_counts.matching(),
        &keys,
    )?;
    let mut tables = BTreeMap::new();
    for relation in &relations {
        let schema = source_schema(&schemas, relation)?;
        let generated = generated_by_relation.remove(relation).ok_or_else(|| {
            ProtocolGenerationError::MissingSourceSchema {
                relation: relation.clone(),
            }
        })?;
        let table = build_protocol_table(relation, schema, row_counts.matching(), generated)?;
        tables.insert(relation.clone(), table);
    }

    Ok(GeneratedData {
        tables,
        row_counts,
        case_coverage,
    })
}

/// Validate one terminal outcome for shared generation and collect its relationships.
fn prepare_outcome<'a>(
    bundle: &'a AnalysisBundle,
    layer_id: &str,
    description: &str,
    schemas: &BTreeMap<String, &RelationSchema>,
) -> Result<(&'a ResolvedComposedSemantics, Vec<EqualityRelationship>), ProtocolGenerationError> {
    let layer = bundle
        .layers()
        .iter()
        .find(|layer| layer.id() == layer_id)
        .ok_or_else(|| ProtocolGenerationError::MissingOutcomeLayer {
            outcome: description.to_owned(),
        })?;
    let semantics = resolved_layer_semantics(layer)?;
    require_exact_conditions(layer.id(), semantics)?;
    if let Some(diagnostic) = semantics.diagnostics().first() {
        return Err(ProtocolGenerationError::UnsupportedSemantics {
            layer_id: layer.id().to_owned(),
            code: diagnostic.code().to_owned(),
            message: diagnostic.message().to_owned(),
        });
    }
    validate_composed_domains(semantics, schemas)?;
    let relationships = collect_equality_relationships(layer.id(), semantics, schemas)?;
    Ok((semantics, relationships))
}

/// Datatype and outcome domains of one relationship key column.
struct RelationshipDomains<'a> {
    data_type: &'a DataType,
    domains: Vec<&'a ValueDomain>,
}

/// Non-null values allowed by every outcome domain on every column of a relationship component.
///
/// Candidates are pooled across all component columns because overlapping domains on
/// different columns can share a value that neither column's own candidates contain.
fn shared_component_values(
    candidates_by_column: &BTreeMap<RelationshipColumn, Vec<ProtocolValue>>,
    domains_by_column: &BTreeMap<RelationshipColumn, RelationshipDomains<'_>>,
    component: &[RelationshipColumn],
) -> Result<Vec<ProtocolValue>, ProtocolGenerationError> {
    let mut pool = Vec::new();
    for column in component {
        for value in candidates_by_column.get(column).into_iter().flatten() {
            if !pool.contains(value) {
                pool.push(value.clone());
            }
        }
    }

    let mut common = Vec::new();
    for value in pool {
        let mut satisfies_all = true;
        for column in component {
            let Some(constraints) = domains_by_column.get(column) else {
                continue;
            };
            for domain in &constraints.domains {
                let satisfies = value_satisfies_domain(constraints.data_type, &value, domain)
                    .map_err(|message| ProtocolGenerationError::UnsupportedDomain {
                        relation: column.relation.clone(),
                        column: column.column.clone(),
                        message,
                    })?;
                if !satisfies {
                    satisfies_all = false;
                }
            }
        }
        if satisfies_all {
            common.push(value);
        }
    }
    Ok(common)
}

/// Domains that terminal outcomes place on one source column, keyed by outcome description.
fn outcome_column_domains<'a>(
    outcomes: &'a [OutcomeSemantics<'a>],
    relation: &str,
    column: &str,
) -> Result<Vec<(&'a str, &'a ValueDomain)>, ProtocolGenerationError> {
    let mut constraints = Vec::new();
    for outcome in outcomes {
        if let Some(domain) =
            find_column_domain(outcome.semantics.column_domains(), relation, column)?
        {
            constraints.push((outcome.description.as_str(), domain));
        }
    }
    Ok(constraints)
}

/// Build the generation plan and candidate values for a column shared by several outcomes.
///
/// A column constrained by at most one outcome keeps that outcome's exact domain. A column
/// constrained by several outcomes samples from candidate values that satisfy all of them.
fn shared_column_plan(
    relation: &str,
    schema_column: &sql_semantic_protocol::SchemaColumn,
    constraints: &[(&str, &ValueDomain)],
) -> Result<(ColumnPlan, Vec<ProtocolValue>), ProtocolGenerationError> {
    let column = schema_column.name();
    let data_type = schema_column.data_type();
    is_supported(data_type).map_err(|_| unsupported_source_type(relation, column, data_type))?;
    let unsupported = |message: String| ProtocolGenerationError::UnsupportedDomain {
        relation: relation.to_owned(),
        column: column.to_owned(),
        message,
    };

    let all_unbounded = constraints
        .iter()
        .all(|(_, domain)| matches!(domain, ValueDomain::Unbounded));
    if constraints.len() <= 1 || all_unbounded {
        let domain = constraints.first().map(|(_, domain)| *domain);
        let plan = ColumnPlan::new(
            column,
            map_domain(relation, column, data_type, domain)?,
            None,
        );
        let values = candidates(data_type, domain).map_err(unsupported)?;
        return Ok((plan, values));
    }

    let mut pool = Vec::new();
    for (_, domain) in constraints {
        validate_boundary_domain(relation, column, domain)?;
        for candidate in candidates(data_type, Some(domain)).map_err(unsupported)? {
            if !pool.contains(&candidate) {
                pool.push(candidate);
            }
        }
    }

    let mut values = Vec::new();
    for candidate in pool {
        let mut satisfies_all = true;
        for (_, domain) in constraints {
            if !value_satisfies_domain(data_type, &candidate, domain).map_err(unsupported)? {
                satisfies_all = false;
                break;
            }
        }
        if satisfies_all {
            values.push(candidate);
        }
    }

    if values.is_empty() {
        return Err(ProtocolGenerationError::ConflictingOutcomes {
            outcomes: constraints
                .iter()
                .map(|(description, _)| (*description).to_owned())
                .collect(),
            message: format!("no value of {relation}.{column} satisfies every outcome's domain"),
        });
    }

    let plan = ColumnPlan::new(column, GenerationDomain::Values(values.clone()), None);
    Ok((plan, values))
}

fn prepare_relation_plans(
    semantics: &ResolvedComposedSemantics,
    relation: &str,
    schema: &RelationSchema,
    include_rejected_domains: bool,
) -> Result<Vec<ColumnPlan>, ProtocolGenerationError> {
    let mut plans = Vec::with_capacity(schema.columns().len());

    for schema_column in schema.columns() {
        is_supported(schema_column.data_type()).map_err(|_| {
            unsupported_source_type(relation, schema_column.name(), schema_column.data_type())
        })?;

        let domain =
            find_column_domain(semantics.column_domains(), relation, schema_column.name())?;
        let generation_domain = map_domain(
            relation,
            schema_column.name(),
            schema_column.data_type(),
            domain,
        )?;
        let rejected_domain = if include_rejected_domains {
            map_rejected_domain(
                relation,
                schema_column.name(),
                schema_column.data_type(),
                domain,
            )?
        } else {
            None
        };
        plans.push(ColumnPlan::new(
            schema_column.name(),
            generation_domain,
            rejected_domain,
        ));
    }

    Ok(plans)
}

fn build_protocol_table(
    relation: &str,
    schema: &RelationSchema,
    rows: usize,
    generated: Vec<Vec<ProtocolValue>>,
) -> Result<Table, ProtocolGenerationError> {
    let arrays = schema
        .columns()
        .iter()
        .zip(generated)
        .map(|(schema_column, values)| {
            build_array(schema_column.data_type(), &values).map_err(|message| {
                ProtocolGenerationError::UnsupportedDomain {
                    relation: relation.to_owned(),
                    column: schema_column.name().to_owned(),
                    message,
                }
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    Table::from_protocol_arrays(schema.columns(), rows, arrays).map_err(|source| {
        ProtocolGenerationError::Table {
            relation: relation.to_owned(),
            source,
        }
    })
}

fn source_schema<'a>(
    schemas: &'a BTreeMap<String, &'a RelationSchema>,
    relation: &str,
) -> Result<&'a RelationSchema, ProtocolGenerationError> {
    schemas
        .get(relation)
        .copied()
        .ok_or_else(|| ProtocolGenerationError::MissingSourceSchema {
            relation: relation.to_owned(),
        })
}

fn collect_equality_relationships(
    selected_layer_id: &str,
    semantics: &ResolvedComposedSemantics,
    schemas: &BTreeMap<String, &RelationSchema>,
) -> Result<Vec<EqualityRelationship>, ProtocolGenerationError> {
    let mut relationships = BTreeSet::new();
    let mut instances = BTreeMap::<(String, String), BTreeSet<String>>::new();
    for equality in semantics.join_equalities() {
        let layer_id = equality.origin_layer_id();
        if equality.join_kind() != JoinKind::Inner {
            return Err(ProtocolGenerationError::UnsupportedRelationship {
                layer_id: layer_id.to_owned(),
                message: format!(
                    "join kind {:?} is not an inner equality",
                    equality.join_kind()
                ),
            });
        }
        for endpoint in [equality.left(), equality.right()] {
            if !semantics
                .dependencies()
                .iter()
                .any(|relation| relation == endpoint.relation())
            {
                return Err(ProtocolGenerationError::UnsupportedRelationship {
                    layer_id: layer_id.to_owned(),
                    message: format!(
                        "equality endpoint {}.{} is not a physical dependency of {selected_layer_id}",
                        endpoint.relation(),
                        endpoint.column()
                    ),
                });
            }
            let seen = instances
                .entry((layer_id.to_owned(), endpoint.relation().to_owned()))
                .or_default();
            seen.insert(endpoint.relation_instance().to_owned());
            if seen.len() > 1 {
                return Err(ProtocolGenerationError::UnsupportedRelationship {
                    layer_id: layer_id.to_owned(),
                    message: format!(
                        "repeated instances of relation {} cannot share generated source keys",
                        endpoint.relation()
                    ),
                });
            }
        }
        let left = RelationshipColumn::new(equality.left().relation(), equality.left().column());
        let right = RelationshipColumn::new(equality.right().relation(), equality.right().column());
        if left == right {
            return Err(ProtocolGenerationError::UnsupportedRelationship {
                layer_id: layer_id.to_owned(),
                message: format!(
                    "equality {} references one physical source column twice",
                    left.describe()
                ),
            });
        }
        validate_relationship_types(layer_id, schemas, &left, &right)?;
        relationships.insert(EqualityRelationship::new(left, right));
    }
    Ok(relationships.into_iter().collect())
}

fn query_for_layer<'a>(
    bundle: &'a AnalysisBundle,
    layer: &TransformationLayer,
) -> Result<&'a QueryStatement, ProtocolGenerationError> {
    let input = bundle
        .inputs()
        .iter()
        .find(|input| input.id() == layer.input_id())
        .ok_or_else(|| ProtocolGenerationError::UnsupportedRelationship {
            layer_id: layer.id().to_owned(),
            message: "layer input is missing from the analysis bundle".to_owned(),
        })?;
    let statement = input
        .statements()
        .get(layer.statement_index())
        .ok_or_else(|| ProtocolGenerationError::UnsupportedRelationship {
            layer_id: layer.id().to_owned(),
            message: "layer statement is missing from its analyzed input".to_owned(),
        })?;

    match statement {
        ProtocolStatement::Query(query) => Ok(query),
        _ => Err(ProtocolGenerationError::UnsupportedRelationship {
            layer_id: layer.id().to_owned(),
            message: "relationship layer does not contain query semantics".to_owned(),
        }),
    }
}

fn reject_unsupported_relational_predicates(
    layer_id: &str,
    query: &QueryStatement,
) -> Result<(), ProtocolGenerationError> {
    for predicate in [
        query.predicates().where_predicate(),
        query.predicates().having_predicate(),
        query.predicates().qualify_predicate(),
    ]
    .into_iter()
    .flatten()
    {
        reject_unsupported_relational_predicate(layer_id, predicate)?;
    }

    Ok(())
}

fn reject_unsupported_relational_predicate(
    layer_id: &str,
    predicate: &Predicate,
) -> Result<(), ProtocolGenerationError> {
    match predicate {
        Predicate::Exists(_) => Err(ProtocolGenerationError::UnsupportedRelationship {
            layer_id: layer_id.to_owned(),
            message: "EXISTS/NOT EXISTS relationship semantics are not yet supported by relational witness generation"
                .to_owned(),
        }),
        Predicate::InSubquery(_) => Err(ProtocolGenerationError::UnsupportedRelationship {
            layer_id: layer_id.to_owned(),
            message: "IN/NOT IN subquery relationship semantics are not yet supported by relational witness generation"
                .to_owned(),
        }),
        Predicate::And(logical) | Predicate::Or(logical) => {
            for operand in logical.operands() {
                reject_unsupported_relational_predicate(layer_id, operand)?;
            }
            Ok(())
        }
        Predicate::Not(not) => reject_unsupported_relational_predicate(layer_id, not.operand()),
        _ => Ok(()),
    }
}

fn collect_column_equalities(
    layer_id: &str,
    predicate: &Predicate,
    equalities: &mut Vec<(ColumnExpression, ColumnExpression)>,
) -> Result<(), ProtocolGenerationError> {
    match predicate {
        Predicate::Comparison(comparison) if comparison.operator() == ComparisonOperator::Eq => {
            match (comparison.left(), comparison.right()) {
                (Expression::Column(left), Expression::Column(right)) => {
                    equalities.push((left.clone(), right.clone()));
                    Ok(())
                }
                _ => Err(ProtocolGenerationError::UnsupportedRelationship {
                    layer_id: layer_id.to_owned(),
                    message: "equality join must compare two protocol column expressions"
                        .to_owned(),
                }),
            }
        }
        Predicate::And(logical) => {
            for operand in logical.operands() {
                collect_column_equalities(layer_id, operand, equalities)?;
            }
            Ok(())
        }
        _ => Err(ProtocolGenerationError::UnsupportedRelationship {
            layer_id: layer_id.to_owned(),
            message: "join condition must be one or more column equalities combined with AND"
                .to_owned(),
        }),
    }
}

fn source_relation_for_column<'a>(
    layer_id: &str,
    query: &'a QueryStatement,
    column: &ColumnExpression,
) -> Result<&'a str, ProtocolGenerationError> {
    if let Some(qualifier) = column.relation() {
        let matches = query
            .sources()
            .iter()
            .filter(|source| source.alias() == Some(qualifier) || source.name() == qualifier)
            .collect::<Vec<_>>();
        return match matches.as_slice() {
            [source] => Ok(source.name()),
            [] => Err(ProtocolGenerationError::UnsupportedRelationship {
                layer_id: layer_id.to_owned(),
                message: format!(
                    "join column qualifier {qualifier:?} does not resolve to a protocol source"
                ),
            }),
            _ => Err(ProtocolGenerationError::UnsupportedRelationship {
                layer_id: layer_id.to_owned(),
                message: format!(
                    "join column qualifier {qualifier:?} resolves to multiple protocol sources"
                ),
            }),
        };
    }

    match query.sources() {
        [source] => Ok(source.name()),
        _ => Err(ProtocolGenerationError::UnsupportedRelationship {
            layer_id: layer_id.to_owned(),
            message: format!(
                "unqualified join column {:?} cannot be mapped to one protocol source",
                column.name()
            ),
        }),
    }
}

fn validate_relationship_types(
    layer_id: &str,
    schemas: &BTreeMap<String, &RelationSchema>,
    left: &RelationshipColumn,
    right: &RelationshipColumn,
) -> Result<(), ProtocolGenerationError> {
    let left_type = relationship_data_type(schemas, left)?;
    let right_type = relationship_data_type(schemas, right)?;
    if left_type != right_type {
        return Err(ProtocolGenerationError::UnsupportedRelationship {
            layer_id: layer_id.to_owned(),
            message: format!(
                "relationship {} = {} compares incompatible canonical datatypes {} and {}",
                left.describe(),
                right.describe(),
                protocol_type_name(left_type),
                protocol_type_name(right_type)
            ),
        });
    }

    Ok(())
}

fn relationship_data_type<'a>(
    schemas: &'a BTreeMap<String, &'a RelationSchema>,
    column: &RelationshipColumn,
) -> Result<&'a DataType, ProtocolGenerationError> {
    let schema = source_schema(schemas, &column.relation)?;
    schema
        .columns()
        .iter()
        .find(|schema_column| schema_column.name() == column.column)
        .map(|schema_column| schema_column.data_type())
        .ok_or_else(|| ProtocolGenerationError::MissingSchemaColumn {
            relation: column.relation.clone(),
            column: column.column.clone(),
        })
}

fn relationship_candidates(
    semantics: &ResolvedComposedSemantics,
    schemas: &BTreeMap<String, &RelationSchema>,
    relationships: &[EqualityRelationship],
) -> Result<BTreeMap<RelationshipColumn, Vec<ProtocolValue>>, ProtocolGenerationError> {
    let columns = relationships
        .iter()
        .flat_map(|relationship| [&relationship.left, &relationship.right])
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut result = BTreeMap::new();

    for column in columns {
        let data_type = relationship_data_type(schemas, &column)?;
        let domain =
            find_column_domain(semantics.column_domains(), &column.relation, &column.column)?;
        let values = candidates(data_type, domain)
            .map_err(|message| ProtocolGenerationError::UnsupportedDomain {
                relation: column.relation.clone(),
                column: column.column.clone(),
                message,
            })?
            .into_iter()
            .filter(|value| !matches!(value, ProtocolValue::Null))
            .collect::<Vec<_>>();
        if values.is_empty() {
            return Err(ProtocolGenerationError::UnsatisfiableRelationship {
                relationship: column.describe(),
            });
        }
        result.insert(column, values);
    }

    Ok(result)
}

fn relationship_adjacency(
    relationships: &[EqualityRelationship],
) -> BTreeMap<RelationshipColumn, BTreeSet<RelationshipColumn>> {
    let mut adjacency = BTreeMap::<RelationshipColumn, BTreeSet<RelationshipColumn>>::new();

    for relationship in relationships {
        adjacency
            .entry(relationship.left.clone())
            .or_default()
            .insert(relationship.right.clone());
        adjacency
            .entry(relationship.right.clone())
            .or_default()
            .insert(relationship.left.clone());
    }

    adjacency
}

fn choose_component_values<R: Rng + ?Sized>(
    candidates_by_column: &BTreeMap<RelationshipColumn, Vec<ProtocolValue>>,
    adjacency: &BTreeMap<RelationshipColumn, BTreeSet<RelationshipColumn>>,
    rng: &mut R,
) -> Result<BTreeMap<RelationshipColumn, ProtocolValue>, ProtocolGenerationError> {
    let mut selected = BTreeMap::new();

    for component in relationship_components(candidates_by_column, adjacency) {
        let common = component_common_values(candidates_by_column, &component);

        if common.is_empty() {
            return Err(ProtocolGenerationError::UnsatisfiableRelationship {
                relationship: component
                    .iter()
                    .map(RelationshipColumn::describe)
                    .collect::<Vec<_>>()
                    .join(" = "),
            });
        }

        let index = sample_relationship_index(rng, common.len(), "relationship component")?;
        let value = common.get(index).cloned().ok_or_else(|| {
            ProtocolGenerationError::InvalidRowConfiguration {
                message: "selected relationship value is missing".to_owned(),
            }
        })?;
        for column in component {
            selected.insert(column, value.clone());
        }
    }

    Ok(selected)
}

/// Group relationship columns into sorted connected components in deterministic order.
fn relationship_components(
    candidates_by_column: &BTreeMap<RelationshipColumn, Vec<ProtocolValue>>,
    adjacency: &BTreeMap<RelationshipColumn, BTreeSet<RelationshipColumn>>,
) -> Vec<Vec<RelationshipColumn>> {
    let mut visited = BTreeSet::new();
    let mut components = Vec::new();

    for start in candidates_by_column.keys() {
        if visited.contains(start) {
            continue;
        }

        let mut component = Vec::new();
        let mut pending = vec![start.clone()];
        while let Some(column) = pending.pop() {
            if !visited.insert(column.clone()) {
                continue;
            }
            component.push(column.clone());
            if let Some(neighbors) = adjacency.get(&column) {
                pending.extend(neighbors.iter().rev().cloned());
            }
        }
        component.sort();
        components.push(component);
    }

    components
}

/// Values allowed for every column of one relationship component, in the first column's order.
fn component_common_values(
    candidates_by_column: &BTreeMap<RelationshipColumn, Vec<ProtocolValue>>,
    component: &[RelationshipColumn],
) -> Vec<ProtocolValue> {
    let Some((first, rest)) = component.split_first() else {
        return Vec::new();
    };
    let mut common = candidates_by_column.get(first).cloned().unwrap_or_default();
    common.retain(|value| {
        rest.iter().all(|column| {
            candidates_by_column
                .get(column)
                .is_some_and(|values| values.contains(value))
        })
    });
    common
}

fn relationship_witnesses<R: Rng + ?Sized>(
    relationships: &[EqualityRelationship],
    candidates_by_column: &BTreeMap<RelationshipColumn, Vec<ProtocolValue>>,
    adjacency: &BTreeMap<RelationshipColumn, BTreeSet<RelationshipColumn>>,
    matching_values: &BTreeMap<RelationshipColumn, ProtocolValue>,
    rng: &mut R,
) -> Result<Vec<RelationshipWitness>, ProtocolGenerationError> {
    let mut witnesses = Vec::new();

    for relationship in relationships {
        let left_degree = adjacency.get(&relationship.left).map_or(0, BTreeSet::len);
        let right_degree = adjacency.get(&relationship.right).map_or(0, BTreeSet::len);

        let candidate_column = if left_degree == 1 {
            Some(&relationship.left)
        } else if right_degree == 1 {
            Some(&relationship.right)
        } else {
            None
        };

        let Some(column) = candidate_column else {
            continue;
        };
        let Some(matching_value) = matching_values.get(column) else {
            continue;
        };
        let alternates = candidates_by_column
            .get(column)
            .map(|values| {
                values
                    .iter()
                    .filter(|value| *value != matching_value)
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if alternates.is_empty() {
            continue;
        }

        let index = sample_relationship_index(rng, alternates.len(), "relationship break")?;
        let value = alternates.get(index).cloned().ok_or_else(|| {
            ProtocolGenerationError::UnsatisfiableRelationship {
                relationship: relationship.describe(),
            }
        })?;
        witnesses.push(RelationshipWitness {
            column: column.clone(),
            value,
        });
    }

    Ok(witnesses)
}

fn set_generated_value(
    generated_by_relation: &mut BTreeMap<String, Vec<Vec<ProtocolValue>>>,
    schemas: &BTreeMap<String, &RelationSchema>,
    column: &RelationshipColumn,
    row: usize,
    value: ProtocolValue,
) -> Result<(), ProtocolGenerationError> {
    let schema = source_schema(schemas, &column.relation)?;
    let column_index = schema
        .columns()
        .iter()
        .position(|schema_column| schema_column.name() == column.column)
        .ok_or_else(|| ProtocolGenerationError::MissingSchemaColumn {
            relation: column.relation.clone(),
            column: column.column.clone(),
        })?;
    let columns = generated_by_relation
        .get_mut(&column.relation)
        .ok_or_else(|| ProtocolGenerationError::MissingSourceSchema {
            relation: column.relation.clone(),
        })?;
    let values = columns.get_mut(column_index).ok_or_else(|| {
        ProtocolGenerationError::MissingSchemaColumn {
            relation: column.relation.clone(),
            column: column.column.clone(),
        }
    })?;
    let slot =
        values
            .get_mut(row)
            .ok_or_else(|| ProtocolGenerationError::InvalidRowConfiguration {
                message: format!(
                    "generated row {row} is missing for relationship column {}",
                    column.describe()
                ),
            })?;
    *slot = value;
    Ok(())
}

fn sample_relationship_index<R: Rng + ?Sized>(
    rng: &mut R,
    value_count: usize,
    purpose: &str,
) -> Result<usize, ProtocolGenerationError> {
    if value_count == 0 {
        return Err(ProtocolGenerationError::InvalidRowConfiguration {
            message: format!("{purpose} has no candidate values"),
        });
    }

    let upper = u64::try_from(value_count).map_err(|_| {
        ProtocolGenerationError::InvalidRowConfiguration {
            message: format!("{purpose} candidate count cannot be represented as u64"),
        }
    })?;
    let zone = u64::MAX - (u64::MAX % upper);

    loop {
        let candidate = rng.next_u64();
        if candidate < zone {
            let index = candidate % upper;
            return usize::try_from(index).map_err(|_| {
                ProtocolGenerationError::InvalidRowConfiguration {
                    message: format!("{purpose} index cannot be represented as usize"),
                }
            });
        }
    }
}

fn terminal_outcomes(bundle: &AnalysisBundle) -> Vec<&DatasetRef> {
    bundle
        .graph()
        .components()
        .iter()
        .flat_map(|component| component.final_outcomes())
        .collect()
}

fn select_terminal_layer(
    bundle: &AnalysisBundle,
    selector: Option<&OutcomeSelector>,
) -> Result<(String, String), ProtocolGenerationError> {
    let finals = terminal_outcomes(bundle);

    let selected = match selector {
        None => match finals.as_slice() {
            [] => return Err(ProtocolGenerationError::NoTerminalOutcome),
            [only] => *only,
            _ => {
                return Err(ProtocolGenerationError::AmbiguousTerminalOutcome {
                    candidates: finals
                        .iter()
                        .map(|outcome| describe_outcome(outcome))
                        .collect(),
                });
            }
        },
        Some(selector) => finals
            .iter()
            .copied()
            .find(|outcome| selector_matches(selector, outcome))
            .ok_or_else(|| ProtocolGenerationError::UnknownTerminalOutcome {
                selector: describe_selector(selector),
            })?,
    };

    outcome_layer(bundle, selected)
}

/// Resolve a terminal outcome to its producing layer identifier and stable description.
fn outcome_layer(
    bundle: &AnalysisBundle,
    selected: &DatasetRef,
) -> Result<(String, String), ProtocolGenerationError> {
    match selected {
        DatasetRef::Anonymous { layer_id } => Ok((layer_id.clone(), describe_outcome(selected))),
        DatasetRef::Relation { name } => {
            let layer = bundle
                .layers()
                .iter()
                .find(|layer| {
                    layer
                        .produces()
                        .iter()
                        .any(|produced| produced.relation_name() == Some(name.as_str()))
                })
                .ok_or_else(|| ProtocolGenerationError::MissingOutcomeLayer {
                    outcome: describe_outcome(selected),
                })?;
            Ok((layer.id().to_owned(), describe_outcome(selected)))
        }
        _ => Err(ProtocolGenerationError::UnknownTerminalOutcome {
            selector: describe_outcome(selected),
        }),
    }
}

fn selector_matches(selector: &OutcomeSelector, outcome: &DatasetRef) -> bool {
    match (selector, outcome) {
        (OutcomeSelector::Relation(expected), DatasetRef::Relation { name }) => expected == name,
        (OutcomeSelector::AnonymousLayer(expected), DatasetRef::Anonymous { layer_id }) => {
            expected == layer_id
        }
        _ => false,
    }
}

fn describe_selector(selector: &OutcomeSelector) -> String {
    match selector {
        OutcomeSelector::Relation(name) => format!("relation:{name}"),
        OutcomeSelector::AnonymousLayer(layer_id) => format!("anonymous:{layer_id}"),
    }
}

fn describe_outcome(outcome: &DatasetRef) -> String {
    match outcome {
        DatasetRef::Relation { name } => format!("relation:{name}"),
        DatasetRef::Anonymous { layer_id } => format!("anonymous:{layer_id}"),
        _ => "unsupported".to_owned(),
    }
}

fn find_column_domain<'a>(
    domains: &'a [sql_semantic_protocol::ColumnDomain],
    relation: &str,
    column: &str,
) -> Result<Option<&'a ValueDomain>, ProtocolGenerationError> {
    let matches = domains
        .iter()
        .filter(|domain| {
            domain.column().name() == column && domain.column().relation() == Some(relation)
        })
        .collect::<Vec<_>>();

    match matches.as_slice() {
        [] => Ok(None),
        [domain] => Ok(Some(domain.domain())),
        _ => Err(ProtocolGenerationError::AmbiguousColumnDomain {
            column: column.to_owned(),
        }),
    }
}

fn map_domain(
    relation: &str,
    column: &str,
    data_type: &DataType,
    domain: Option<&ValueDomain>,
) -> Result<GenerationDomain, ProtocolGenerationError> {
    match domain {
        Some(ValueDomain::Empty) => Err(ProtocolGenerationError::EmptyDomain {
            relation: relation.to_owned(),
            column: column.to_owned(),
        }),
        Some(ValueDomain::Unknown(unknown)) => Err(ProtocolGenerationError::UnknownDomain {
            relation: relation.to_owned(),
            column: column.to_owned(),
            reason: unknown.reason().to_owned(),
        }),
        Some(domain) => generic_generation_domain(relation, column, data_type, Some(domain)),
        None => generic_generation_domain(relation, column, data_type, None),
    }
}

fn map_rejected_domain(
    relation: &str,
    column: &str,
    data_type: &DataType,
    domain: Option<&ValueDomain>,
) -> Result<Option<GenerationDomain>, ProtocolGenerationError> {
    let Some(domain) = domain else {
        return Ok(None);
    };

    match domain {
        ValueDomain::Empty => {
            return Err(ProtocolGenerationError::EmptyDomain {
                relation: relation.to_owned(),
                column: column.to_owned(),
            });
        }
        ValueDomain::Unknown(unknown) => {
            return Err(ProtocolGenerationError::UnknownDomain {
                relation: relation.to_owned(),
                column: column.to_owned(),
                reason: unknown.reason().to_owned(),
            });
        }
        _ => {}
    }

    let values = rejected_candidates(data_type, domain).map_err(|message| {
        ProtocolGenerationError::UnsupportedDomain {
            relation: relation.to_owned(),
            column: column.to_owned(),
            message: format!("cannot derive rejected values: {message}"),
        }
    })?;

    if values.is_empty() {
        Ok(None)
    } else if let ValueDomain::Ranges(ranges) = domain {
        Ok(Some(GenerationDomain::RejectedRange {
            data_type: data_type.clone(),
            ranges: ranges.ranges().to_vec(),
        }))
    } else {
        Ok(Some(GenerationDomain::Values(values)))
    }
}

fn generic_generation_domain(
    relation: &str,
    column: &str,
    data_type: &DataType,
    domain: Option<&ValueDomain>,
) -> Result<GenerationDomain, ProtocolGenerationError> {
    let values = candidates(data_type, domain).map_err(|message| {
        ProtocolGenerationError::UnsupportedDomain {
            relation: relation.to_owned(),
            column: column.to_owned(),
            message,
        }
    })?;
    if values.is_empty() {
        return Err(ProtocolGenerationError::EmptyDomain {
            relation: relation.to_owned(),
            column: column.to_owned(),
        });
    }

    match domain {
        Some(ValueDomain::Ranges(ranges)) => Ok(GenerationDomain::Range {
            data_type: data_type.clone(),
            ranges: ranges.ranges().to_vec(),
        }),
        None | Some(ValueDomain::Unbounded) => Ok(GenerationDomain::Unconstrained {
            data_type: data_type.clone(),
        }),
        Some(_) => Ok(GenerationDomain::Values(values)),
    }
}

fn unsupported_source_type(
    relation: &str,
    column: &str,
    data_type: &DataType,
) -> ProtocolGenerationError {
    ProtocolGenerationError::UnsupportedSourceType {
        relation: relation.to_owned(),
        column: column.to_owned(),
        data_type: protocol_type_name(data_type),
    }
}

fn protocol_type_name(data_type: &DataType) -> String {
    match data_type {
        DataType::Custom { name, modifiers } if modifiers.is_empty() => format!("custom:{name}"),
        DataType::Custom { name, modifiers } => {
            format!("custom:{name}({})", modifiers.join(","))
        }
        _ => data_type.kind().to_owned(),
    }
}
