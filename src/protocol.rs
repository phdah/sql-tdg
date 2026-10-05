//! SQL Semantic Protocol integration and protocol-driven test-data generation.

use std::collections::{BTreeMap, BTreeSet};

use rand_chacha::ChaCha8Rng;
use rand_core::{Rng, SeedableRng};
use std::error::Error;
use std::fmt;

use sql_semantic_protocol::{
    AnalysisBundle, ColumnExpression, ComparisonOperator, ComposedSemantics, ConfiguredSqlInput,
    DataType, DatasetRef, Expression, JoinKind, Predicate, ProtocolStatement, QueryStatement,
    RelationCatalog, RelationResolution, RelationSchema, ResolvedComposedSemantics, SqlInput,
    TransformationLayer, ValueDomain, analyze_configured_inputs_with_catalog, dialect_from_name,
};

use crate::generator::{ColumnPlan, GenerationDomain, Generator, GeneratorError};
use crate::protocol_value::{
    ProtocolValue, build_array, candidates, is_supported, rejected_candidates,
};
use crate::solver::SolverError;
use crate::table::{Table, TableError};

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

/// Arrow-backed generated source tables keyed by canonical relation identity.
pub struct GeneratedData {
    tables: BTreeMap<String, Table>,
    row_counts: GenerationRowCounts,
}

impl fmt::Debug for GeneratedData {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("GeneratedData")
            .field("relations", &self.tables.keys().collect::<Vec<_>>())
            .field("row_counts", &self.row_counts)
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
    /// A physical source relation does not have declared schema metadata.
    MissingSourceSchema {
        /// Canonical source relation identity.
        relation: String,
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
            Self::MissingSourceSchema { relation } => {
                write!(
                    formatter,
                    "missing source schema metadata for relation {relation:?}"
                )
            }
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

/// Analyze SQL through SQL Semantic Protocol and generate matching source rows.
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

/// Analyze SQL through SQL Semantic Protocol and generate classified source rows.
pub fn generate_classified_from_sql(
    sql: &str,
    dialect_name: &str,
    source_schemas: &[RelationSchema],
    row_counts: GenerationRowCounts,
    seed: u64,
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
    let catalog = RelationCatalog::from_schemas(source_schemas).map_err(|error| {
        ProtocolGenerationError::InvalidSchemaMetadata {
            message: error.to_string(),
        }
    })?;
    let bundle =
        analyze_configured_inputs_with_catalog(&configured, &catalog).map_err(|error| {
            ProtocolGenerationError::Analysis {
                message: error.to_string(),
            }
        })?;

    generate_classified_from_bundle(&bundle, None, row_counts, seed)
}

/// Generate matching source rows from one explicitly resolved terminal outcome in a protocol bundle.
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

/// Generate classified source rows from one explicitly resolved terminal outcome in a protocol bundle.
pub fn generate_classified_from_bundle(
    bundle: &AnalysisBundle,
    selector: Option<&OutcomeSelector>,
    row_counts: GenerationRowCounts,
    seed: u64,
) -> Result<GeneratedData, ProtocolGenerationError> {
    let (layer_id, outcome_description) = select_terminal_layer(bundle, selector)?;
    let layer = bundle
        .layers()
        .iter()
        .find(|layer| layer.id() == layer_id)
        .ok_or(ProtocolGenerationError::MissingOutcomeLayer {
            outcome: outcome_description,
        })?;

    let semantics = match layer.composed_semantics() {
        ComposedSemantics::Resolved(semantics) => semantics,
        ComposedSemantics::Unresolved(unresolved) => {
            return Err(ProtocolGenerationError::UnresolvedComposition {
                layer_id: layer.id().to_owned(),
                reason: format!("{:?}", unresolved.reason()),
            });
        }
        _ => {
            return Err(ProtocolGenerationError::UnresolvedComposition {
                layer_id: layer.id().to_owned(),
                reason: "unsupported composed semantics variant".to_owned(),
            });
        }
    };

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

    validate_composed_domains(semantics, &schemas)?;
    let relationships = collect_equality_relationships(bundle, layer.id(), semantics, &schemas)?;

    if relationships.is_empty() {
        generate_scalar_data(semantics, &schemas, row_counts, seed)
    } else {
        generate_relational_data(
            layer.id(),
            semantics,
            &schemas,
            &relationships,
            row_counts,
            seed,
        )
    }
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
    semantics: &ResolvedComposedSemantics,
    schemas: &BTreeMap<String, &RelationSchema>,
    row_counts: GenerationRowCounts,
    seed: u64,
) -> Result<GeneratedData, ProtocolGenerationError> {
    let mut tables = BTreeMap::new();

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

        let table = build_protocol_table(relation, schema, row_counts.total(), generated)?;
        tables.insert(relation.clone(), table);
    }

    Ok(GeneratedData { tables, row_counts })
}

fn generate_relational_data(
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

    Ok(GeneratedData { tables, row_counts })
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
    bundle: &AnalysisBundle,
    selected_layer_id: &str,
    semantics: &ResolvedComposedSemantics,
    schemas: &BTreeMap<String, &RelationSchema>,
) -> Result<Vec<EqualityRelationship>, ProtocolGenerationError> {
    let ancestor_ids = ancestor_layer_ids(bundle, selected_layer_id);
    let mut relationships = BTreeSet::new();

    for layer in bundle
        .layers()
        .iter()
        .filter(|layer| ancestor_ids.contains(layer.id()))
    {
        let query = query_for_layer(bundle, layer)?;
        reject_unsupported_relational_predicates(layer.id(), query)?;
        for join in query.joins() {
            if join.kind() == JoinKind::Cross && join.condition().is_none() {
                continue;
            }
            if join.kind() != JoinKind::Inner {
                return Err(ProtocolGenerationError::UnsupportedRelationship {
                    layer_id: layer.id().to_owned(),
                    message: format!(
                        "join kind {:?} does not define the required matching/non-matching witness contract",
                        join.kind()
                    ),
                });
            }

            let condition = join.condition().ok_or_else(|| {
                ProtocolGenerationError::UnsupportedRelationship {
                    layer_id: layer.id().to_owned(),
                    message: "inner join has no resolved condition".to_owned(),
                }
            })?;
            let mut equalities = Vec::new();
            collect_column_equalities(layer.id(), condition, &mut equalities)?;

            for (left_expression, right_expression) in equalities {
                let left = resolve_relationship_column(bundle, layer, query, &left_expression)?;
                let right = resolve_relationship_column(bundle, layer, query, &right_expression)?;

                if left == right {
                    return Err(ProtocolGenerationError::UnsupportedRelationship {
                        layer_id: layer.id().to_owned(),
                        message: format!(
                            "relationship {} = {} resolves to the same physical source column",
                            left.describe(),
                            right.describe()
                        ),
                    });
                }
                if !semantics.dependencies().contains(&left.relation)
                    || !semantics.dependencies().contains(&right.relation)
                {
                    return Err(ProtocolGenerationError::UnsupportedRelationship {
                        layer_id: layer.id().to_owned(),
                        message:
                            "resolved relationship is outside the selected outcome dependencies"
                                .to_owned(),
                    });
                }

                validate_relationship_types(layer.id(), schemas, &left, &right)?;
                relationships.insert(EqualityRelationship::new(left, right));
            }
        }
    }

    Ok(relationships.into_iter().collect())
}

fn ancestor_layer_ids(bundle: &AnalysisBundle, selected_layer_id: &str) -> BTreeSet<String> {
    let mut ancestors = BTreeSet::new();
    let mut pending = vec![selected_layer_id.to_owned()];

    while let Some(layer_id) = pending.pop() {
        if !ancestors.insert(layer_id.clone()) {
            continue;
        }

        let mut producers = bundle
            .graph()
            .edges()
            .iter()
            .filter(|edge| {
                edge.consumer_layer_id() == layer_id
                    && edge.resolution() == RelationResolution::Resolved
            })
            .flat_map(|edge| edge.producer_layer_ids().iter().cloned())
            .collect::<Vec<_>>();
        producers.sort();
        producers.reverse();
        pending.extend(producers);
    }

    ancestors
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

fn resolve_relationship_column(
    bundle: &AnalysisBundle,
    layer: &TransformationLayer,
    query: &QueryStatement,
    column: &ColumnExpression,
) -> Result<RelationshipColumn, ProtocolGenerationError> {
    let source_relation = source_relation_for_column(layer.id(), query, column)?;
    let edge = bundle
        .graph()
        .edges()
        .iter()
        .find(|edge| edge.consumer_layer_id() == layer.id() && edge.relation() == source_relation)
        .ok_or_else(|| ProtocolGenerationError::UnsupportedRelationship {
            layer_id: layer.id().to_owned(),
            message: format!(
                "source {}.{} has no protocol dependency edge",
                source_relation,
                column.name()
            ),
        })?;

    match edge.resolution() {
        RelationResolution::External => Ok(RelationshipColumn::new(edge.relation(), column.name())),
        RelationResolution::Resolved => {
            let [producer_id] = edge.producer_layer_ids() else {
                return Err(ProtocolGenerationError::UnsupportedRelationship {
                    layer_id: layer.id().to_owned(),
                    message: format!(
                        "source relation {source_relation} does not resolve to exactly one producer"
                    ),
                });
            };
            let producer = bundle
                .layers()
                .iter()
                .find(|candidate| candidate.id() == producer_id)
                .ok_or_else(|| ProtocolGenerationError::UnsupportedRelationship {
                    layer_id: layer.id().to_owned(),
                    message: format!("producer layer {producer_id} is missing"),
                })?;
            let producer_semantics = match producer.composed_semantics() {
                ComposedSemantics::Resolved(semantics) => semantics,
                _ => {
                    return Err(ProtocolGenerationError::UnsupportedRelationship {
                        layer_id: layer.id().to_owned(),
                        message: format!(
                            "producer layer {producer_id} does not have resolved composed semantics"
                        ),
                    });
                }
            };
            let matches = producer_semantics
                .output()
                .columns()
                .iter()
                .filter(|output| output.name() == column.name())
                .collect::<Vec<_>>();
            let [output] = matches.as_slice() else {
                return Err(ProtocolGenerationError::UnsupportedRelationship {
                    layer_id: layer.id().to_owned(),
                    message: format!(
                        "producer relation {source_relation} does not expose exactly one column named {}",
                        column.name()
                    ),
                });
            };
            if !matches!(output.expression(), Expression::Column(_)) {
                return Err(ProtocolGenerationError::UnsupportedRelationship {
                    layer_id: layer.id().to_owned(),
                    message: format!(
                        "join key {source_relation}.{} is produced by a non-identity expression",
                        column.name()
                    ),
                });
            }
            let [lineage] = output.lineage() else {
                return Err(ProtocolGenerationError::UnsupportedRelationship {
                    layer_id: layer.id().to_owned(),
                    message: format!(
                        "join key {source_relation}.{} does not resolve to one physical source column",
                        column.name()
                    ),
                });
            };
            Ok(RelationshipColumn::new(
                lineage.relation(),
                lineage.column(),
            ))
        }
        resolution => Err(ProtocolGenerationError::UnsupportedRelationship {
            layer_id: layer.id().to_owned(),
            message: format!(
                "source relation {source_relation} has unsupported resolution {resolution:?}"
            ),
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
    let mut visited = BTreeSet::new();
    let mut selected = BTreeMap::new();

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

        let Some(first) = component.first() else {
            continue;
        };
        let mut common = candidates_by_column.get(first).cloned().ok_or_else(|| {
            ProtocolGenerationError::UnsatisfiableRelationship {
                relationship: first.describe(),
            }
        })?;
        common.retain(|value| {
            component.iter().skip(1).all(|column| {
                candidates_by_column
                    .get(column)
                    .is_some_and(|values| values.contains(value))
            })
        });

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

fn select_terminal_layer(
    bundle: &AnalysisBundle,
    selector: Option<&OutcomeSelector>,
) -> Result<(String, String), ProtocolGenerationError> {
    let finals = bundle
        .graph()
        .components()
        .iter()
        .flat_map(|component| component.final_outcomes())
        .collect::<Vec<_>>();

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
    Ok(GenerationDomain::Values(values))
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
