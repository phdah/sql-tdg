//! Deterministic physical-source witnesses for protocol CASE branch domains.

use std::collections::{BTreeMap, BTreeSet};

use sql_semantic_protocol::{
    AnalysisBundle, CaseSourceDomains, ComparisonAssumption, DataType, Expression, RelationSchema,
    ResolvedComposedSemantics, ValueDomain,
};

use crate::protocol::ProtocolGenerationError;
use crate::protocol_value::{ProtocolValue, candidates, value_satisfies_domain};

type ColumnKey = (String, String);

struct BranchTarget<'a> {
    location: String,
    domains: &'a CaseSourceDomains,
}

/// Coverage status of one protocol CASE branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaseCoverageStatus {
    /// A generated matching row was assigned an exact branch witness.
    Covered,
    /// Protocol control flow or composed query filters exclude the branch.
    Unreachable,
    /// Protocol cannot derive exact physical source conditions for this branch.
    Unknown,
    /// No compatible matching row was available to witness the branch.
    InsufficientRows,
}

impl CaseCoverageStatus {
    /// Return the stable name for CLI reporting.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Covered => "covered",
            Self::Unreachable => "unreachable",
            Self::Unknown => "unknown",
            Self::InsufficientRows => "insufficient_rows",
        }
    }
}

/// An explicit branch-coverage result, including branches that cannot be exercised.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseCoverageFinding {
    location: String,
    status: CaseCoverageStatus,
    detail: String,
}

impl CaseCoverageFinding {
    fn new(location: &str, status: CaseCoverageStatus, detail: impl Into<String>) -> Self {
        Self {
            location: location.to_owned(),
            status,
            detail: detail.into(),
        }
    }

    /// Layer, output column, and CASE arm.
    pub fn location(&self) -> &str {
        &self.location
    }
    /// Whether the branch was witnessed, excluded, or could not be exercised.
    pub fn status(&self) -> CaseCoverageStatus {
        self.status
    }
    /// Reason or matching-row position of the coverage result.
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

/// Exercise CASE branch witnesses where feasible without weakening source membership.
///
/// Unreachable/unknown branches and insufficient row budgets are reported explicitly rather than
/// silently presented as covered. A single row may witness compatible branches across different
/// CASE expressions, including copies propagated through upstream layers.
pub(crate) fn cover_case_branches(
    bundle: &AnalysisBundle,
    roots: &[String],
    outcomes: &[&ResolvedComposedSemantics],
    schemas: &BTreeMap<String, &RelationSchema>,
    generated: &mut BTreeMap<String, Vec<Vec<ProtocolValue>>>,
    matching_rows: usize,
    relationship_keys: &BTreeSet<ColumnKey>,
) -> Result<Vec<CaseCoverageFinding>, ProtocolGenerationError> {
    let mut targets = Vec::new();
    let mut visited = BTreeSet::new();
    let mut pending = roots.to_vec();
    while let Some(id) = pending.pop() {
        if !visited.insert(id.clone()) {
            continue;
        }
        let layer = bundle
            .layers()
            .iter()
            .find(|layer| layer.id() == id)
            .ok_or_else(|| ProtocolGenerationError::CaseCoverage {
                location: id.clone(),
                message: "referenced transformation layer is missing".to_owned(),
            })?;
        if let sql_semantic_protocol::ComposedSemantics::Resolved(semantics) =
            layer.composed_semantics()
        {
            for output in semantics.output().columns() {
                collect_cases(
                    output.expression(),
                    &format!("{}:{}", layer.id(), output.name()),
                    &mut targets,
                );
            }
        }
        for edge in bundle
            .graph()
            .edges()
            .iter()
            .filter(|edge| edge.consumer_layer_id() == id)
        {
            pending.extend(edge.producer_layer_ids().iter().cloned());
        }
    }
    targets.sort_by(|left, right| left.location.cmp(&right.location));
    let mut assigned = vec![BTreeMap::<ColumnKey, ProtocolValue>::new(); matching_rows];
    let mut findings = Vec::with_capacity(targets.len());

    for target in targets {
        let alternatives = match target.domains {
            CaseSourceDomains::Reachable { alternatives } => alternatives,
            CaseSourceDomains::Unreachable => {
                findings.push(CaseCoverageFinding::new(
                    &target.location,
                    CaseCoverageStatus::Unreachable,
                    "excluded by earlier CASE branches",
                ));
                continue;
            }
            CaseSourceDomains::Unknown(reason) => {
                findings.push(CaseCoverageFinding::new(
                    &target.location,
                    CaseCoverageStatus::Unknown,
                    reason.reason(),
                ));
                continue;
            }
            _ => {
                return Err(ProtocolGenerationError::CaseCoverage {
                    location: target.location,
                    message: "unsupported future CASE domain variant".to_owned(),
                });
            }
        };
        let mut feasible = false;
        let mut covered_row = None;
        for alternative in alternatives {
            let Some(witness) = branch_witness(
                alternative.column_domains(),
                &target.location,
                outcomes,
                schemas,
            )?
            else {
                continue;
            };
            feasible = true;
            for (row, row_assignments) in assigned.iter_mut().enumerate() {
                let compatible = witness.iter().all(|(key, value)| {
                    row_assignments
                        .get(key)
                        .is_none_or(|existing| existing == value)
                        && (!relationship_keys.contains(key)
                            || generated
                                .get(&key.0)
                                .and_then(|columns| {
                                    schemas.get(&key.0).and_then(|schema| {
                                        schema
                                            .columns()
                                            .iter()
                                            .position(|column| column.name() == key.1)
                                            .and_then(|index| columns.get(index))
                                    })
                                })
                                .and_then(|values| values.get(row))
                                .is_some_and(|existing| existing == value))
                });
                if !compatible {
                    continue;
                }
                for (key, value) in &witness {
                    let schema = schemas.get(&key.0).ok_or_else(|| {
                        ProtocolGenerationError::CaseCoverage {
                            location: target.location.clone(),
                            message: format!("missing source schema for {}", key.0),
                        }
                    })?;
                    let index = schema
                        .columns()
                        .iter()
                        .position(|column| column.name() == key.1)
                        .ok_or_else(|| ProtocolGenerationError::CaseCoverage {
                            location: target.location.clone(),
                            message: format!("missing source column {}.{}", key.0, key.1),
                        })?;
                    let slot = generated
                        .get_mut(&key.0)
                        .and_then(|columns| columns.get_mut(index))
                        .and_then(|values| values.get_mut(row))
                        .ok_or_else(|| ProtocolGenerationError::CaseCoverage {
                            location: target.location.clone(),
                            message: format!("no matching row {row} for {}.{}", key.0, key.1),
                        })?;
                    *slot = value.clone();
                    row_assignments.insert(key.clone(), value.clone());
                }
                covered_row = Some(row);
                break;
            }
            if covered_row.is_some() {
                break;
            }
        }
        let finding = if let Some(row) = covered_row {
            CaseCoverageFinding::new(
                &target.location,
                CaseCoverageStatus::Covered,
                format!("row {row}"),
            )
        } else if feasible {
            CaseCoverageFinding::new(
                &target.location,
                CaseCoverageStatus::InsufficientRows,
                format!("no compatible witness among {matching_rows} matching rows"),
            )
        } else {
            CaseCoverageFinding::new(
                &target.location,
                CaseCoverageStatus::Unreachable,
                "no CASE alternative intersects the composed query domains",
            )
        };
        findings.push(finding);
    }
    Ok(findings)
}

fn branch_witness(
    branch_domains: &[sql_semantic_protocol::ColumnDomain],
    location: &str,
    outcomes: &[&ResolvedComposedSemantics],
    schemas: &BTreeMap<String, &RelationSchema>,
) -> Result<Option<BTreeMap<ColumnKey, ProtocolValue>>, ProtocolGenerationError> {
    let mut by_column = BTreeMap::<ColumnKey, Vec<&ValueDomain>>::new();
    for column_domain in branch_domains {
        let Some(relation) = column_domain.column().relation() else {
            return Err(ProtocolGenerationError::CaseCoverage {
                location: location.to_owned(),
                message: format!(
                    "CASE column {} is not a physical source",
                    column_domain.column().name()
                ),
            });
        };
        by_column
            .entry((
                relation.to_owned(),
                column_domain.column().name().to_owned(),
            ))
            .or_default()
            .push(column_domain.domain());
    }
    let mut chosen = BTreeMap::new();
    for (key, mut domains) in by_column {
        let (relation, column) = (&key.0, &key.1);
        let schema =
            schemas
                .get(relation)
                .ok_or_else(|| ProtocolGenerationError::CaseCoverage {
                    location: location.to_owned(),
                    message: format!("CASE source relation {relation} has no schema"),
                })?;
        let source_column = schema
            .columns()
            .iter()
            .find(|item| item.name() == column)
            .ok_or_else(|| ProtocolGenerationError::CaseCoverage {
                location: location.to_owned(),
                message: format!("CASE source column {relation}.{column} has no schema"),
            })?;
        for outcome in outcomes {
            domains.extend(
                outcome
                    .column_domains()
                    .iter()
                    .filter(|domain| {
                        domain.column().relation() == Some(relation.as_str())
                            && domain.column().name() == column
                    })
                    .map(|domain| domain.domain()),
            );
        }
        require_case_assumptions(location, source_column.data_type(), outcomes)?;
        let mut pool = Vec::new();
        for domain in &domains {
            let sampled = match candidates(source_column.data_type(), Some(domain)) {
                Ok(values) => values,
                Err(message) if message == "NULL cannot be generated for a non-nullable datatype" => {
                    return Ok(None);
                }
                Err(message) => {
                    return Err(ProtocolGenerationError::CaseCoverage {
                        location: location.to_owned(),
                        message: format!("cannot sample {relation}.{column}: {message}"),
                    });
                }
            };
            for candidate in sampled {
                if !pool.contains(&candidate) {
                    pool.push(candidate);
                }
            }
        }
        let mut valid = None;
        for candidate in pool {
            let mut satisfies = true;
            for domain in &domains {
                if !value_satisfies_domain(source_column.data_type(), &candidate, domain).map_err(
                    |message| ProtocolGenerationError::CaseCoverage {
                        location: location.to_owned(),
                        message: format!("cannot check {relation}.{column}: {message}"),
                    },
                )? {
                    satisfies = false;
                    break;
                }
            }
            if satisfies {
                valid = Some(candidate);
                break;
            }
        }
        let Some(value) = valid else {
            return Ok(None);
        };
        chosen.insert(key, value);
    }
    Ok(Some(chosen))
}

fn require_case_assumptions(
    location: &str,
    data_type: &DataType,
    outcomes: &[&ResolvedComposedSemantics],
) -> Result<(), ProtocolGenerationError> {
    let mut required = Vec::new();
    let mut data_type = data_type;
    while let DataType::Nullable(inner) = data_type {
        data_type = inner;
    }
    match data_type {
        DataType::String { fixed, .. } => {
            required.push(ComparisonAssumption::BinaryCollation);
            if *fixed {
                required.push(ComparisonAssumption::NoCharPadding);
            }
        }
        DataType::FloatingPoint { .. } => {
            required.extend([
                ComparisonAssumption::NoNan,
                ComparisonAssumption::SignedZeroEquivalent,
            ]);
        }
        DataType::Timestamp { .. } => required.push(ComparisonAssumption::SessionTimeZone),
        _ => {}
    };
    for assumption in required {
        if !outcomes.iter().all(|outcome| {
            outcome
                .condition_exactness()
                .declared_assumptions()
                .contains(&assumption)
        }) {
            return Err(ProtocolGenerationError::CaseCoverage {
                location: location.to_owned(),
                message: format!(
                    "CASE comparison requires declared assumption {}",
                    assumption.as_str()
                ),
            });
        }
    }
    Ok(())
}

fn collect_cases<'a>(
    expression: &'a Expression,
    location: &str,
    targets: &mut Vec<BranchTarget<'a>>,
) {
    match expression {
        Expression::Case(case) => {
            for (index, branch) in case.branches().iter().enumerate() {
                let name = format!("{location}:WHEN {}", index + 1);
                targets.push(BranchTarget {
                    location: name.clone(),
                    domains: branch.source_domains(),
                });
                collect_cases(branch.result(), &name, targets);
            }
            targets.push(BranchTarget {
                location: format!("{location}:ELSE"),
                domains: case.else_source_domains(),
            });
            if let Some(result) = case.else_result() {
                collect_cases(result, location, targets);
            }
        }
        Expression::Binary(binary) => {
            collect_cases(binary.left(), location, targets);
            collect_cases(binary.right(), location, targets);
        }
        Expression::Unary(unary) => collect_cases(unary.operand(), location, targets),
        Expression::Function(function) => {
            for argument in function.arguments() {
                collect_cases(argument, location, targets);
            }
        }
        _ => {}
    }
}
