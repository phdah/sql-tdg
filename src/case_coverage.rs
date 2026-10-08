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

/// Exercise every CASE branch represented by exact physical source domains.
///
/// CASE reachability is provided entirely by SQL Semantic Protocol. Witness values are checked
/// against every selected outcome's composed domains before they replace matching source rows.
pub(crate) fn cover_case_branches(
    bundle: &AnalysisBundle,
    roots: &[String],
    outcomes: &[&ResolvedComposedSemantics],
    schemas: &BTreeMap<String, &RelationSchema>,
    generated: &mut BTreeMap<String, Vec<Vec<ProtocolValue>>>,
    matching_rows: usize,
    relationship_keys: &BTreeSet<ColumnKey>,
) -> Result<(), ProtocolGenerationError> {
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

    if targets.len() > matching_rows {
        return Err(ProtocolGenerationError::CaseCoverage {
            location: "matching rows".to_owned(),
            message: format!(
                "{} CASE branches require at least {} matching rows, got {matching_rows}",
                targets.len(),
                targets.len()
            ),
        });
    }

    for (row, target) in targets.iter().enumerate() {
        let alternatives = match target.domains {
            CaseSourceDomains::Reachable { alternatives } => alternatives,
            CaseSourceDomains::Unreachable => {
                return Err(ProtocolGenerationError::CaseCoverage {
                    location: target.location.clone(),
                    message: "branch is unreachable by CASE control flow".to_owned(),
                });
            }
            CaseSourceDomains::Unknown(reason) => {
                return Err(ProtocolGenerationError::CaseCoverage {
                    location: target.location.clone(),
                    message: format!("branch is not coverable: {}", reason.reason()),
                });
            }
            _ => {
                return Err(ProtocolGenerationError::CaseCoverage {
                    location: target.location.clone(),
                    message: "unsupported future CASE domain variant".to_owned(),
                });
            }
        };
        let mut witness = None;
        for alternative in alternatives {
            let mut by_column = BTreeMap::<ColumnKey, Vec<&ValueDomain>>::new();
            for column_domain in alternative.column_domains() {
                let Some(relation) = column_domain.column().relation() else {
                    return Err(ProtocolGenerationError::CaseCoverage {
                        location: target.location.clone(),
                        message: format!(
                            "CASE column {} is not resolved to a physical relation",
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
            let mut possible = true;
            for (key, mut domains) in by_column {
                let (relation, column) = (&key.0, &key.1);
                let schema =
                    schemas
                        .get(relation)
                        .ok_or_else(|| ProtocolGenerationError::CaseCoverage {
                            location: target.location.clone(),
                            message: format!("CASE source relation {relation} has no schema"),
                        })?;
                let source_column = schema
                    .columns()
                    .iter()
                    .find(|item| item.name() == column)
                    .ok_or_else(|| ProtocolGenerationError::CaseCoverage {
                        location: target.location.clone(),
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

                require_case_assumptions(&target.location, source_column.data_type(), outcomes)?;
                let mut pool = Vec::new();
                for domain in &domains {
                    for candidate in
                        candidates(source_column.data_type(), Some(domain)).map_err(|message| {
                            ProtocolGenerationError::CaseCoverage {
                                location: target.location.clone(),
                                message: format!("cannot sample {relation}.{column}: {message}"),
                            }
                        })?
                    {
                        if !pool.contains(&candidate) {
                            pool.push(candidate);
                        }
                    }
                }
                let mut valid = None;
                for candidate in pool {
                    let mut satisfies = true;
                    for domain in &domains {
                        if !value_satisfies_domain(source_column.data_type(), &candidate, domain)
                            .map_err(|message| ProtocolGenerationError::CaseCoverage {
                                location: target.location.clone(),
                                message: format!("cannot check {relation}.{column}: {message}"),
                            })?
                        {
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
                    possible = false;
                    break;
                };
                chosen.insert(key, value);
            }
            if possible {
                witness = Some(chosen);
                break;
            }
        }
        let Some(witness) = witness else {
            return Err(ProtocolGenerationError::CaseCoverage {
                location: target.location.clone(),
                message: "branch is unreachable under the composed query column domains".to_owned(),
            });
        };
        for ((relation, column), value) in witness {
            let schema =
                schemas
                    .get(&relation)
                    .ok_or_else(|| ProtocolGenerationError::CaseCoverage {
                        location: target.location.clone(),
                        message: format!("missing source schema for {relation}"),
                    })?;
            let index = schema
                .columns()
                .iter()
                .position(|item| item.name() == column)
                .ok_or_else(|| ProtocolGenerationError::CaseCoverage {
                    location: target.location.clone(),
                    message: format!("missing source column {relation}.{column}"),
                })?;
            let slot = generated
                .get_mut(&relation)
                .and_then(|columns| columns.get_mut(index))
                .and_then(|values| values.get_mut(row))
                .ok_or_else(|| ProtocolGenerationError::CaseCoverage {
                    location: target.location.clone(),
                    message: format!("no matching row {row} for {relation}.{column}"),
                })?;
            if relationship_keys.contains(&(relation.clone(), column.clone())) && *slot != value {
                return Err(ProtocolGenerationError::CaseCoverage {
                    location: target.location.clone(),
                    message: format!(
                        "CASE witness for {relation}.{column} conflicts with the coordinated join key"
                    ),
                });
            }
            *slot = value;
        }
    }
    Ok(())
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
