//! SQL Semantic Protocol integration and protocol-driven test-data generation.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use sql_semantic_protocol::{
    AnalysisBundle, ComposedSemantics, ConfiguredSqlInput, DataType, DatasetRef, RelationCatalog,
    RelationSchema, SqlInput, ValueDomain, analyze_configured_inputs_with_catalog,
    dialect_from_name,
};

use crate::generator::{ColumnPlan, GenerationDomain, Generator, GeneratorError};
use crate::protocol_value::{build_array, candidates, is_supported};
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

/// Arrow-backed generated source tables keyed by canonical relation identity.
pub struct GeneratedData {
    tables: BTreeMap<String, Table>,
}

impl fmt::Debug for GeneratedData {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("GeneratedData")
            .field("relations", &self.tables.keys().collect::<Vec<_>>())
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

/// Analyze SQL through SQL Semantic Protocol and generate source tables for its terminal outcome.
pub fn generate_from_sql(
    sql: &str,
    dialect_name: &str,
    source_schemas: &[RelationSchema],
    rows: usize,
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

    generate_from_bundle(&bundle, None, rows, seed)
}

/// Generate source tables from one explicitly resolved terminal outcome in a protocol bundle.
pub fn generate_from_bundle(
    bundle: &AnalysisBundle,
    selector: Option<&OutcomeSelector>,
    rows: usize,
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

    let mut schemas = BTreeMap::new();
    for schema in bundle.source_schemas() {
        schemas.insert(schema.relation(), schema);
    }

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

    let mut tables = BTreeMap::new();
    for relation in semantics.dependencies() {
        let schema = schemas.get(relation.as_str()).copied().ok_or_else(|| {
            ProtocolGenerationError::MissingSourceSchema {
                relation: relation.clone(),
            }
        })?;

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
            plans.push(ColumnPlan::new(schema_column.name(), generation_domain));
        }

        let generated = Generator::new()
            .generate_protocol_values(rows, seed, &plans)
            .map_err(|source| ProtocolGenerationError::Generator {
                relation: relation.clone(),
                source,
            })?;

        let arrays = schema
            .columns()
            .iter()
            .zip(generated)
            .map(|(schema_column, values)| {
                build_array(schema_column.data_type(), &values).map_err(|message| {
                    ProtocolGenerationError::UnsupportedDomain {
                        relation: relation.clone(),
                        column: schema_column.name().to_owned(),
                        message,
                    }
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        let table =
            Table::from_protocol_arrays(schema.columns(), rows, arrays).map_err(|source| {
                ProtocolGenerationError::Table {
                    relation: relation.clone(),
                    source,
                }
            })?;
        tables.insert(relation.clone(), table);
    }

    Ok(GeneratedData { tables })
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
