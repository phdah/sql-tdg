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
