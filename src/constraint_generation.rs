//! Protocol-owned relation constraints applied to generated physical source values.

use std::collections::{BTreeMap, BTreeSet};

use rand_chacha::ChaCha8Rng;
use rand_core::{Rng, SeedableRng};
use sql_semantic_protocol::{
    AnalysisBundle, ConstraintValue, DataType, RelationConstraint, RelationSchema, ValueDomain,
};

use crate::protocol::ProtocolGenerationError;
use crate::protocol_value::{
    ProtocolValue, candidates, sample_range_value, sample_unconstrained_value,
    value_satisfies_domain,
};

pub(crate) type ColumnDomains = BTreeMap<(String, String), Vec<ValueDomain>>;
type ValuesByRelation = BTreeMap<String, Vec<Vec<ProtocolValue>>>;

fn failure(relation: &str, message: impl Into<String>) -> ProtocolGenerationError {
    ProtocolGenerationError::RelationConstraint {
        relation: relation.to_owned(),
        message: message.into(),
    }
}

fn choose_index<R: Rng + ?Sized>(rng: &mut R, length: usize) -> Result<usize, String> {
    let upper = u64::try_from(length).map_err(|_| "value set too large")?;
    if upper == 0 {
        return Err("value set is empty".to_owned());
    }
    usize::try_from(rng.next_u64() % upper).map_err(|_| "sample index overflow".to_owned())
}

fn typed_value(data_type: &DataType, value: &ConstraintValue) -> Result<ProtocolValue, String> {
    if let DataType::Nullable(inner) = data_type {
        if matches!(value, ConstraintValue::Null) {
            return Ok(ProtocolValue::Null);
        }
        return typed_value(inner, value);
    }
    let result = match (data_type, value) {
        (_, ConstraintValue::Null) => {
            return Err("NULL is not a non-null accepted value".to_owned());
        }
        (DataType::Boolean, ConstraintValue::Boolean(value)) => ProtocolValue::Boolean(*value),
        (DataType::SignedInteger { bits }, value) => {
            let numeric = match value {
                ConstraintValue::Integer(value) => *value,
                ConstraintValue::UnsignedInteger(value) => {
                    i64::try_from(*value).map_err(|_| "signed integer overflow")?
                }
                ConstraintValue::String(value) => value.parse::<i64>()
                    .map_err(|_| "integer accepted value is not numeric")?,
                _ => return Err("expected signed integer accepted value".to_owned()),
            };
            match bits.unwrap_or(64) {
                0..=8 => ProtocolValue::Int8(i8::try_from(numeric).map_err(|_| "INT8 overflow")?),
                9..=16 => ProtocolValue::Int16(i16::try_from(numeric).map_err(|_| "INT16 overflow")?),
                17..=32 => ProtocolValue::Int32(i32::try_from(numeric).map_err(|_| "INT32 overflow")?),
                33..=64 => ProtocolValue::Int64(numeric),
                _ => return Err("unsupported signed integer width".to_owned()),
            }
        }
        (DataType::UnsignedInteger { bits }, value) => {
            let numeric = match value {
                ConstraintValue::UnsignedInteger(value) => *value,
                ConstraintValue::Integer(value) => {
                    u64::try_from(*value).map_err(|_| "negative unsigned integer")?
                }
                ConstraintValue::String(value) => value.parse::<u64>()
                    .map_err(|_| "unsigned accepted value is not numeric")?,
                _ => return Err("expected unsigned integer accepted value".to_owned()),
            };
            match bits.unwrap_or(64) {
                0..=8 => ProtocolValue::UInt8(u8::try_from(numeric).map_err(|_| "UINT8 overflow")?),
                9..=16 => ProtocolValue::UInt16(u16::try_from(numeric).map_err(|_| "UINT16 overflow")?),
                17..=32 => ProtocolValue::UInt32(u32::try_from(numeric).map_err(|_| "UINT32 overflow")?),
                33..=64 => ProtocolValue::UInt64(numeric),
                _ => return Err("unsupported unsigned integer width".to_owned()),
            }
        }
        (DataType::String { .. } | DataType::Enum { .. } | DataType::Set { .. }, ConstraintValue::String(value)) => {
            ProtocolValue::String(value.clone())
        }
        (DataType::FloatingPoint { bits }, value) => {
            let text = match value {
                ConstraintValue::Number(value) | ConstraintValue::String(value) => value.clone(),
                ConstraintValue::Integer(value) => value.to_string(),
                ConstraintValue::UnsignedInteger(value) => value.to_string(),
                _ => return Err("expected floating-point accepted value".to_owned()),
            };
            if bits.unwrap_or(64) <= 32 {
                ProtocolValue::Float32(text.parse::<f32>().map_err(|_| "invalid FLOAT32")?.to_bits())
            } else {
                ProtocolValue::Float64(text.parse::<f64>().map_err(|_| "invalid FLOAT64")?.to_bits())
            }
        }
        (DataType::Decimal { precision, scale }, value) => {
            let text = match value {
                ConstraintValue::Number(value) | ConstraintValue::String(value) => value.clone(),
                ConstraintValue::Integer(value) => value.to_string(),
                ConstraintValue::UnsignedInteger(value) => value.to_string(),
                _ => return Err("expected decimal accepted value".to_owned()),
            };
            ProtocolValue::Decimal128(crate::protocol_value::parse_decimal_scaled(
                &text,
                precision.unwrap_or(38),
                scale.unwrap_or(0),
            )?)
        }
        _ => return Err(format!("unsupported accepted-value type: {data_type:?}, {value:?}")),
    };
    if !value_satisfies_domain(data_type, &result, &ValueDomain::Unbounded)? {
        return Err("accepted value is outside its declared column datatype".to_owned());
    }
    Ok(result)
}

fn schema_column<'a>(
    schemas: &'a BTreeMap<String, &RelationSchema>,
    relation: &str,
    column: &str,
) -> Result<(usize, &'a DataType), ProtocolGenerationError> {
    let schema = schemas.get(relation).ok_or_else(|| failure(relation, "missing schema"))?;
    schema.columns().iter().enumerate()
        .find(|(_, candidate)| candidate.name() == column)
        .map(|(index, column)| (index, column.data_type()))
        .ok_or_else(|| failure(relation, format!("constraint references missing column {column}")))
}

fn field<'a>(
    data: &'a ValuesByRelation,
    schemas: &BTreeMap<String, &RelationSchema>,
    relation: &str,
    column: &str,
    row: usize,
) -> Result<&'a ProtocolValue, ProtocolGenerationError> {
    let (index, _) = schema_column(schemas, relation, column)?;
    data.get(relation).and_then(|columns| columns.get(index))
        .and_then(|values| values.get(row))
        .ok_or_else(|| failure(relation, format!("{column}: missing row {row}")))
}

fn replace(
    data: &mut ValuesByRelation,
    schemas: &BTreeMap<String, &RelationSchema>,
    relation: &str,
    column: &str,
    row: usize,
    value: ProtocolValue,
) -> Result<(), ProtocolGenerationError> {
    let (index, _) = schema_column(schemas, relation, column)?;
    let slot = data.get_mut(relation).and_then(|columns| columns.get_mut(index))
        .and_then(|values| values.get_mut(row))
        .ok_or_else(|| failure(relation, format!("{column}: missing row {row}")))?;
    *slot = value;
    Ok(())
}

fn tuple(
    data: &ValuesByRelation,
    schemas: &BTreeMap<String, &RelationSchema>,
    relation: &str,
    columns: &[String],
    row: usize,
) -> Result<Vec<ProtocolValue>, ProtocolGenerationError> {
    columns.iter().map(|column| field(data, schemas, relation, column, row).cloned()).collect()
}

fn permitted(
    domains: &ColumnDomains,
    relation: &str,
    column: &str,
    data_type: &DataType,
    value: &ProtocolValue,
    accepted: Option<&Vec<ProtocolValue>>,
    require_nonnull: bool,
) -> Result<bool, ProtocolGenerationError> {
    if require_nonnull && matches!(value, ProtocolValue::Null) {
        return Ok(false);
    }
    if let Some(accepted) = accepted {
        if !matches!(value, ProtocolValue::Null) && !accepted.contains(value) {
            return Ok(false);
        }
    }
    if !value_satisfies_domain(data_type, value, &ValueDomain::Unbounded)
        .map_err(|reason| failure(relation, format!("{column}: {reason}")))?
    {
        return Ok(false);
    }
    if let Some(restrictions) = domains.get(&(relation.to_owned(), column.to_owned())) {
        for domain in restrictions {
            if !value_satisfies_domain(data_type, value, domain)
                .map_err(|reason| failure(relation, format!("{column}: {reason}")))?
            {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

fn sample_valid<R: Rng + ?Sized>(
    rng: &mut R,
    domains: &ColumnDomains,
    relation: &str,
    column: &str,
    data_type: &DataType,
    accepted: Option<&Vec<ProtocolValue>>,
    require_nonnull: bool,
) -> Result<ProtocolValue, ProtocolGenerationError> {
    let restriction = domains.get(&(relation.to_owned(), column.to_owned()))
        .and_then(|items| items.first());
    for _ in 0..1024 {
        let value = if let Some(accepted) = accepted {
            let index = choose_index(rng, accepted.len())
                .map_err(|reason| failure(relation, format!("{column}: {reason}")))?;
            accepted.get(index).cloned()
                .ok_or_else(|| failure(relation, "missing accepted value"))?
        } else if let Some(domain) = restriction {
            match domain {
                ValueDomain::Ranges(ranges) => sample_range_value(data_type, ranges.ranges(), rng)
                    .map_err(|reason| failure(relation, reason))?,
                _ => {
                    let options = candidates(data_type, Some(domain))
                        .map_err(|reason| failure(relation, reason))?;
                    let index = choose_index(rng, options.len())
                        .map_err(|reason| failure(relation, reason))?;
                    options.get(index).cloned()
                        .ok_or_else(|| failure(relation, "missing domain candidate"))?
                }
            }
        } else {
            sample_unconstrained_value(data_type, rng)
                .map_err(|reason| failure(relation, reason))?
        };
        if permitted(domains, relation, column, data_type, &value, accepted, require_nonnull)? {
            return Ok(value);
        }
    }
    Err(failure(relation, format!("{column}: no sampled value satisfies the query domains and relation constraints")))
}


/// Enforce constraints on physical relation values before Arrow arrays are built.
///
/// Unsupported declarations, missing parents, and unsatisfiable constraints fail closed.
/// Metadata for non-generated relations is surfaced as not honored by the caller.
pub(crate) fn enforce(
    bundle: &AnalysisBundle,
    schemas: &BTreeMap<String, &RelationSchema>,
    data: &mut ValuesByRelation,
    domains: &ColumnDomains,
    matching_rows: usize,
    seed: u64,
) -> Result<Vec<String>, ProtocolGenerationError> {
    if let Some(diagnostic) = bundle.constraint_diagnostics().first() {
        return Err(failure("<bundle>", format!("{}: {}", diagnostic.code(), diagnostic.message())));
    }

    let mut unhonored = Vec::new();
    let mut accepted = BTreeMap::<String, BTreeMap<String, Vec<ProtocolValue>>>::new();
    let mut nonnull = BTreeMap::<String, BTreeSet<String>>::new();
    for constraints in bundle.relation_constraints() {
        let relation = constraints.relation();
        if !data.contains_key(relation) {
            if !constraints.constraints().is_empty() || !constraints.diagnostics().is_empty() {
                unhonored.push(relation.to_owned());
            }
            continue;
        }
        if let Some(diagnostic) = constraints.diagnostics().first() {
            return Err(failure(relation, format!("{}: {}", diagnostic.code(), diagnostic.message())));
        }
        let mut relation_accepted = BTreeMap::new();
        let mut relation_nonnull = BTreeSet::new();
        for constraint in constraints.constraints() {
            match constraint {
                RelationConstraint::PrimaryKey(key) => {
                    relation_nonnull.extend(key.columns().iter().cloned());
                }
                RelationConstraint::NotNull(column) => {
                    relation_nonnull.insert(column.column().to_owned());
                }
                RelationConstraint::AcceptedValues(column) => {
                    let (_, datatype) = schema_column(schemas, relation, column.column())?;
                    let values = column.values().iter()
                        .filter(|value| !matches!(value, ConstraintValue::Null))
                        .map(|value| typed_value(datatype, value)
                            .map_err(|reason| failure(relation, format!("{}: {reason}", column.column()))))
                        .collect::<Result<Vec<_>, _>>()?;
                    relation_accepted.insert(column.column().to_owned(), values);
                }
                RelationConstraint::UniqueKey(_) | RelationConstraint::ForeignKey(_) => {}
                _ => return Err(failure(relation, "unsupported future RelationConstraint variant")),
            }
        }
        accepted.insert(relation.to_owned(), relation_accepted);
        nonnull.insert(relation.to_owned(), relation_nonnull);
    }

    if data.values().any(|columns| columns.first().is_some_and(|values| values.len() != matching_rows))
        && bundle.relation_constraints().iter().any(|set| data.contains_key(set.relation()) && !set.constraints().is_empty())
    {
        return Err(failure("<bundle>", "relation constraints with deliberately rejected rows are not supported"));
    }

    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    // First ensure all constrained scalar columns are typed and within query domains.
    for (relation, relation_accepted) in &accepted {
        let schema = schemas.get(relation).ok_or_else(|| failure(relation, "missing schema"))?;
        let required = nonnull.get(relation).ok_or_else(|| failure(relation, "missing non-null metadata"))?;
        for column in schema.columns() {
            if !required.contains(column.name()) && !relation_accepted.contains_key(column.name()) {
                continue;
            }
            for row in 0..matching_rows {
                if !permitted(
                    domains, relation, column.name(), column.data_type(),
                    field(data, schemas, relation, column.name(), row)?,
                    relation_accepted.get(column.name()), required.contains(column.name()),
                )? {
                    let replacement = sample_valid(
                        &mut rng, domains, relation, column.name(), column.data_type(),
                        relation_accepted.get(column.name()), required.contains(column.name()),
                    )?;
                    replace(data, schemas, relation, column.name(), row, replacement)?;
                }
            }
        }
    }

    // Key tuples can be composite. SQL unique constraints permit repeated NULL tuples,
    // while primary keys require every component to be non-NULL.
    for set in bundle.relation_constraints() {
        let relation = set.relation();
        if !data.contains_key(relation) { continue; }
        for constraint in set.constraints() {
            let (columns, primary) = match constraint {
                RelationConstraint::PrimaryKey(key) => (key.columns(), true),
                RelationConstraint::UniqueKey(key) => (key.columns(), false),
                _ => continue,
            };
            let mut seen = Vec::<Vec<ProtocolValue>>::new();
            for row in 0..matching_rows {
                let mut key = tuple(data, schemas, relation, columns, row)?;
                if key.iter().any(|value| matches!(value, ProtocolValue::Null)) && !primary {
                    continue;
                }
                if seen.contains(&key) {
                    let mut resolved = false;
                    for attempt in 0..1024 {
                        let column = &columns[attempt % columns.len()];
                        let (_, datatype) = schema_column(schemas, relation, column)?;
                        let candidate = sample_valid(
                            &mut rng, domains, relation, column, datatype,
                            accepted.get(relation).and_then(|fields| fields.get(column)),
                            nonnull.get(relation).is_some_and(|cols| cols.contains(column)),
                        )?;
                        let previous = field(data, schemas, relation, column, row)?.clone();
                        replace(data, schemas, relation, column, row, candidate)?;
                        key = tuple(data, schemas, relation, columns, row)?;
                        if !seen.contains(&key) {
                            resolved = true;
                            break;
                        }
                        replace(data, schemas, relation, column, row, previous)?;
                    }
                    if !resolved {
                        return Err(failure(relation, format!("key {columns:?} cannot be unique for {matching_rows} rows")));
                    }
                }
                seen.push(key);
            }
        }
    }

    // Assign each foreign-key tuple from an actual generated parent row.
    for set in bundle.relation_constraints() {
        let relation = set.relation();
        if !data.contains_key(relation) { continue; }
        for constraint in set.constraints() {
            let RelationConstraint::ForeignKey(key) = constraint else { continue };
            let target = key.referenced_relation();
            let parent_rows = data.get(target)
                .and_then(|columns| columns.first())
                .map(Vec::len)
                .ok_or_else(|| failure(relation, format!("foreign key references non-generated relation {target}")))?;

            for row in 0..matching_rows {
                let child = tuple(data, schemas, relation, key.columns(), row)?;
                if child.iter().any(|value| matches!(value, ProtocolValue::Null)) { continue; }
                let mut existing = false;
                for parent_row in 0..parent_rows {
                    if tuple(data, schemas, target, key.referenced_columns(), parent_row)? == child {
                        existing = true;
                        break;
                    }
                }
                if existing { continue; }
                let mut selected = None;
                for offset in 0..parent_rows {
                    let parent_row = (row + offset) % parent_rows;
                    let parent = tuple(data, schemas, target, key.referenced_columns(), parent_row)?;
                    let mut valid = true;
                    for (column, value) in key.columns().iter().zip(&parent) {
                        let (_, datatype) = schema_column(schemas, relation, column)?;
                        if !permitted(
                            domains, relation, column, datatype, value,
                            accepted.get(relation).and_then(|fields| fields.get(column)),
                            nonnull.get(relation).is_some_and(|cols| cols.contains(column)),
                        )? {
                            valid = false;
                            break;
                        }
                    }
                    if valid {
                        selected = Some(parent);
                        break;
                    }
                }
                let parent = selected.ok_or_else(|| failure(
                    relation, format!("foreign key {:?}: no valid referenced tuple satisfies the query domains", key.columns()),
                ))?;
                for (column, value) in key.columns().iter().zip(parent) {
                    replace(data, schemas, relation, column, row, value)?;
                }
            }
        }
    }

    validate(bundle, schemas, data, domains, matching_rows)?;
    Ok(unhonored)
}

/// Check the final values after other witness-generation passes have run.
pub(crate) fn validate(
    bundle: &AnalysisBundle,
    schemas: &BTreeMap<String, &RelationSchema>,
    data: &ValuesByRelation,
    domains: &ColumnDomains,
    rows: usize,
) -> Result<(), ProtocolGenerationError> {
    for set in bundle.relation_constraints() {
        let relation = set.relation();
        if !data.contains_key(relation) { continue; }
        let mut primary_columns = BTreeSet::new();
        for constraint in set.constraints() {
            if let RelationConstraint::PrimaryKey(key) = constraint {
                primary_columns.extend(key.columns().iter().cloned());
            }
        }
        for constraint in set.constraints() {
            match constraint {
                RelationConstraint::NotNull(column) => {
                    for row in 0..rows {
                        if matches!(field(data, schemas, relation, column.column(), row)?, ProtocolValue::Null) {
                            return Err(failure(relation, format!("not_null violated: {}", column.column())));
                        }
                    }
                }
                RelationConstraint::AcceptedValues(column) => {
                    let (_, datatype) = schema_column(schemas, relation, column.column())?;
                    let allowed = column.values().iter()
                        .filter(|value| !matches!(value, ConstraintValue::Null))
                        .map(|value| typed_value(datatype, value)
                            .map_err(|reason| failure(relation, reason)))
                        .collect::<Result<Vec<_>, _>>()?;
                    for row in 0..rows {
                        if !permitted(
                            domains, relation, column.column(), datatype,
                            field(data, schemas, relation, column.column(), row)?,
                            Some(&allowed), primary_columns.contains(column.column()),
                        )? {
                            return Err(failure(relation, format!("accepted_values violated: {}", column.column())));
                        }
                    }
                }
                RelationConstraint::PrimaryKey(key) | RelationConstraint::UniqueKey(key) => {
                    let primary = matches!(constraint, RelationConstraint::PrimaryKey(_));
                    let mut seen = Vec::new();
                    for row in 0..rows {
                        let key_value = tuple(data, schemas, relation, key.columns(), row)?;
                        if key_value.iter().any(|value| matches!(value, ProtocolValue::Null)) {
                            if primary {
                                return Err(failure(relation, format!("primary key {:?} contains NULL", key.columns())));
                            }
                            continue;
                        }
                        if seen.contains(&key_value) {
                            return Err(failure(relation, format!("key {:?} contains duplicate values", key.columns())));
                        }
                        seen.push(key_value);
                    }
                }
                RelationConstraint::ForeignKey(key) => {
                    let parent_rows = data.get(key.referenced_relation())
                        .and_then(|columns| columns.first()).map(Vec::len)
                        .ok_or_else(|| failure(relation, "foreign key parent is not generated"))?;
                    for row in 0..rows {
                        let child = tuple(data, schemas, relation, key.columns(), row)?;
                        if child.iter().any(|value| matches!(value, ProtocolValue::Null)) { continue; }
                        let mut found = false;
                        for parent_row in 0..parent_rows {
                            if tuple(data, schemas, key.referenced_relation(), key.referenced_columns(), parent_row)? == child {
                                found = true;
                                break;
                            }
                        }
                        if !found {
                            return Err(failure(relation, format!("foreign key {:?} has no matching parent", key.columns())));
                        }
                    }
                }
                _ => return Err(failure(relation, "unsupported future RelationConstraint variant")),
            }
        }

        // A modifier must never invalidate any composed SQL domain, including shared outcomes.
        let schema = schemas.get(relation).ok_or_else(|| failure(relation, "missing schema"))?;
        for column in schema.columns() {
            for row in 0..rows {
                if !permitted(
                    domains, relation, column.name(), column.data_type(),
                    field(data, schemas, relation, column.name(), row)?, None,
                    primary_columns.contains(column.name()),
                )? {
                    return Err(failure(relation, format!("column {} violates a composed query domain", column.name())));
                }
            }
        }
    }
    Ok(())
}
