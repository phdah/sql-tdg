//! Canonical protocol values and lossless Arrow materialization.

use std::cmp::Ordering;
use std::sync::Arc;

use arrow_array::ArrayRef;
use arrow_array::builder::{
    ArrayBuilder, BinaryBuilder, BooleanBuilder, Date32Builder, Decimal128Builder,
    FixedSizeBinaryBuilder, FixedSizeListBuilder, Float32Builder, Float64Builder, Int8Builder,
    Int16Builder, Int32Builder, Int64Builder, ListBuilder, MapBuilder, StringBuilder,
    StructBuilder, Time64MicrosecondBuilder, Time64NanosecondBuilder, TimestampMicrosecondBuilder,
    TimestampNanosecondBuilder, UInt8Builder, UInt16Builder, UInt32Builder, UInt64Builder,
    make_builder,
};
use arrow_schema::{DataType as ArrowDataType, Field, Fields, TimeUnit};
use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Timelike, Utc};
use rand_core::Rng;
use sql_semantic_protocol::{
    DataType, LiteralExpression, LiteralType, LiteralValue, SetMode, ValueDomain, ValueRange,
};

const MAX_FIXED_COLLECTION_LENGTH: u64 = 1_024;

/// One generated value represented independently from Arrow's physical layout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ProtocolValue {
    Null,
    Boolean(bool),
    Int8(i8),
    Int16(i16),
    Int32(i32),
    Int64(i64),
    UInt8(u8),
    UInt16(u16),
    UInt32(u32),
    UInt64(u64),
    Float32(u32),
    Float64(u64),
    Decimal128(i128),
    String(String),
    Binary(Vec<u8>),
    Date32(i32),
    TimeMicroseconds(i64),
    TimeNanoseconds(i64),
    TimestampMicroseconds(i64),
    TimestampNanoseconds(i64),
    List(Vec<ProtocolValue>),
    Struct(Vec<ProtocolValue>),
    Map(Vec<(ProtocolValue, ProtocolValue)>),
}

pub(crate) fn arrow_data_type(data_type: &DataType) -> Result<ArrowDataType, String> {
    match data_type {
        DataType::Boolean => Ok(ArrowDataType::Boolean),
        DataType::SignedInteger { bits } => match bits.unwrap_or(64) {
            0..=8 => Ok(ArrowDataType::Int8),
            9..=16 => Ok(ArrowDataType::Int16),
            17..=32 => Ok(ArrowDataType::Int32),
            33..=64 => Ok(ArrowDataType::Int64),
            bits => Err(format!("signed_integer({bits})")),
        },
        DataType::UnsignedInteger { bits } => match bits.unwrap_or(64) {
            0..=8 => Ok(ArrowDataType::UInt8),
            9..=16 => Ok(ArrowDataType::UInt16),
            17..=32 => Ok(ArrowDataType::UInt32),
            33..=64 => Ok(ArrowDataType::UInt64),
            bits => Err(format!("unsigned_integer({bits})")),
        },
        DataType::Decimal { precision, scale } => {
            let precision = precision.unwrap_or(38);
            let scale = scale.unwrap_or(0);
            if precision == 0 || precision > 38 || scale > precision || scale > i8::MAX as u64 {
                return Err(format!("decimal({precision},{scale})"));
            }
            Ok(ArrowDataType::Decimal128(
                u8::try_from(precision).map_err(|_| format!("decimal({precision},{scale})"))?,
                i8::try_from(scale).map_err(|_| format!("decimal({precision},{scale})"))?,
            ))
        }
        DataType::FloatingPoint { bits } => match bits.unwrap_or(64) {
            0..=32 => Ok(ArrowDataType::Float32),
            33..=64 => Ok(ArrowDataType::Float64),
            bits => Err(format!("floating_point({bits})")),
        },
        DataType::String { .. } => Ok(ArrowDataType::Utf8),
        DataType::Binary {
            length: Some(length),
            fixed: true,
        } => {
            let length = i32::try_from(*length).map_err(|_| format!("binary({length})"))?;
            Ok(ArrowDataType::FixedSizeBinary(length))
        }
        DataType::Binary { .. } => Ok(ArrowDataType::Binary),
        DataType::Date => Ok(ArrowDataType::Date32),
        DataType::Time { precision } => {
            if precision.is_none_or(|value| value <= 6) {
                Ok(ArrowDataType::Time64(TimeUnit::Microsecond))
            } else if precision.is_some_and(|value| value <= 9) {
                Ok(ArrowDataType::Time64(TimeUnit::Nanosecond))
            } else {
                Err(format!("time({})", precision.unwrap_or_default()))
            }
        }
        DataType::Timestamp { precision } => {
            if precision.is_none_or(|value| value <= 6) {
                Ok(ArrowDataType::Timestamp(TimeUnit::Microsecond, None))
            } else if precision.is_some_and(|value| value <= 9) {
                Ok(ArrowDataType::Timestamp(TimeUnit::Nanosecond, None))
            } else {
                Err(format!("timestamp({})", precision.unwrap_or_default()))
            }
        }
        DataType::Uuid => Ok(ArrowDataType::FixedSizeBinary(16)),
        DataType::Json => Ok(ArrowDataType::Utf8),
        DataType::Array { element, length } => {
            let element = element
                .as_deref()
                .ok_or_else(|| "array without an element datatype".to_owned())?;
            let child = arrow_data_type(element)?;
            let field = Arc::new(Field::new_list_field(child, is_nullable(element)));
            match length {
                Some(length) => {
                    if *length > MAX_FIXED_COLLECTION_LENGTH {
                        return Err(format!("array[{length}]"));
                    }
                    let length = i32::try_from(*length).map_err(|_| format!("array[{length}]"))?;
                    Ok(ArrowDataType::FixedSizeList(field, length))
                }
                None => Ok(ArrowDataType::List(field)),
            }
        }
        DataType::Map { key, value } => {
            if is_nullable(key) {
                return Err("map with nullable key".to_owned());
            }
            let key_field = Arc::new(Field::new("key", arrow_data_type(key)?, false));
            let value_field = Arc::new(Field::new(
                "value",
                arrow_data_type(value)?,
                is_nullable(value),
            ));
            let entries = ArrowDataType::Struct(Fields::from(vec![key_field, value_field]));
            Ok(ArrowDataType::Map(
                Arc::new(Field::new("entries", entries, false)),
                false,
            ))
        }
        DataType::Struct { fields } => Ok(ArrowDataType::Struct(Fields::from(
            fields
                .iter()
                .enumerate()
                .map(|(index, field)| {
                    Ok(Arc::new(Field::new(
                        field
                            .name()
                            .map(str::to_owned)
                            .unwrap_or_else(|| format!("field_{index}")),
                        arrow_data_type(field.data_type())?,
                        is_nullable(field.data_type()),
                    )))
                })
                .collect::<Result<Vec<_>, String>>()?,
        ))),
        DataType::Enum { .. } | DataType::Set { .. } => Ok(ArrowDataType::Utf8),
        DataType::Nullable(inner) => arrow_data_type(inner),
        DataType::Interval
        | DataType::BitString { .. }
        | DataType::Union { .. }
        | DataType::Table { .. }
        | DataType::Geometry { .. }
        | DataType::Regclass
        | DataType::TextSearchVector
        | DataType::TextSearchQuery
        | DataType::Any
        | DataType::Unspecified
        | DataType::Trigger => Err(data_type.kind().to_owned()),
        DataType::Custom { name, modifiers } => {
            if modifiers.is_empty() {
                Err(format!("custom:{name}"))
            } else {
                Err(format!("custom:{name}({})", modifiers.join(",")))
            }
        }
    }
}

pub(crate) fn is_supported(data_type: &DataType) -> Result<(), String> {
    arrow_data_type(data_type).map(|_| ())
}

pub(crate) fn candidates(
    data_type: &DataType,
    domain: Option<&ValueDomain>,
) -> Result<Vec<ProtocolValue>, String> {
    match data_type {
        DataType::Nullable(inner) => nullable_candidates(inner, domain),
        _ => non_null_candidates(data_type, domain),
    }
}

pub(crate) fn rejected_candidates(
    data_type: &DataType,
    domain: &ValueDomain,
) -> Result<Vec<ProtocolValue>, String> {
    match data_type {
        DataType::Nullable(inner) => nullable_rejected_candidates(inner, domain),
        _ => non_null_rejected_candidates(data_type, domain),
    }
}

pub(crate) fn value_satisfies_domain(
    data_type: &DataType,
    value: &ProtocolValue,
    domain: &ValueDomain,
) -> Result<bool, String> {
    if !value_is_representable(data_type, value)? {
        return Ok(false);
    }

    match domain {
        ValueDomain::Unbounded => Ok(true),
        ValueDomain::Empty => Ok(false),
        ValueDomain::Unknown(unknown) => Err(unknown.reason().to_owned()),
        ValueDomain::Ranges(ranges) => {
            if matches!(value, ProtocolValue::Null) {
                Ok(false)
            } else {
                value_in_any_range(value, ranges.ranges(), data_type)
            }
        }
        ValueDomain::Set(set) => {
            let members = set
                .values()
                .iter()
                .map(|literal| value_from_literal(data_type, literal))
                .collect::<Result<Vec<_>, _>>()?;
            let contains = members.contains(value);
            Ok(match set.mode() {
                SetMode::Include => contains,
                SetMode::Exclude => !contains,
            })
        }
        _ => Err("unsupported protocol value-domain variant".to_owned()),
    }
}

fn nullable_rejected_candidates(
    inner: &DataType,
    domain: &ValueDomain,
) -> Result<Vec<ProtocolValue>, String> {
    match domain {
        ValueDomain::Unbounded | ValueDomain::Empty => Ok(Vec::new()),
        ValueDomain::Unknown(unknown) => Err(unknown.reason().to_owned()),
        ValueDomain::Set(set) => match set.mode() {
            SetMode::Include => {
                let includes_null = set
                    .values()
                    .iter()
                    .any(|literal| matches!(literal.value(), LiteralValue::Null));
                let included = set
                    .values()
                    .iter()
                    .filter(|literal| !matches!(literal.value(), LiteralValue::Null))
                    .map(|literal| value_from_literal(inner, literal))
                    .collect::<Result<Vec<_>, _>>()?;
                let mut values = Vec::new();
                if !includes_null {
                    values.push(ProtocolValue::Null);
                }
                if let Ok(value) = non_excluded_value(inner, &included) {
                    push_unique(&mut values, value);
                }
                Ok(values)
            }
            SetMode::Exclude => {
                let mut values = Vec::new();
                for literal in set.values() {
                    let value = if matches!(literal.value(), LiteralValue::Null) {
                        ProtocolValue::Null
                    } else {
                        value_from_literal(inner, literal)?
                    };
                    push_unique(&mut values, value);
                }
                Ok(values)
            }
        },
        ValueDomain::Ranges(ranges) => {
            let mut values = vec![ProtocolValue::Null];
            for value in range_rejected_candidates(inner, ranges.ranges())? {
                push_unique(&mut values, value);
            }
            Ok(values)
        }
        _ => Err("unsupported protocol value-domain variant".to_owned()),
    }
}

fn non_null_rejected_candidates(
    data_type: &DataType,
    domain: &ValueDomain,
) -> Result<Vec<ProtocolValue>, String> {
    match domain {
        ValueDomain::Unbounded | ValueDomain::Empty => Ok(Vec::new()),
        ValueDomain::Unknown(unknown) => Err(unknown.reason().to_owned()),
        ValueDomain::Set(set) => match set.mode() {
            SetMode::Include => {
                let included = set
                    .values()
                    .iter()
                    .filter(|literal| !matches!(literal.value(), LiteralValue::Null))
                    .map(|literal| value_from_literal(data_type, literal))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(non_excluded_value(data_type, &included)
                    .ok()
                    .into_iter()
                    .collect())
            }
            SetMode::Exclude => {
                let mut values = Vec::new();
                for literal in set
                    .values()
                    .iter()
                    .filter(|literal| !matches!(literal.value(), LiteralValue::Null))
                {
                    push_unique(&mut values, value_from_literal(data_type, literal)?);
                }
                Ok(values)
            }
        },
        ValueDomain::Ranges(ranges) => range_rejected_candidates(data_type, ranges.ranges()),
        _ => Err("unsupported protocol value-domain variant".to_owned()),
    }
}

fn range_rejected_candidates(
    data_type: &DataType,
    ranges: &[sql_semantic_protocol::ValueRange],
) -> Result<Vec<ProtocolValue>, String> {
    let mut values = Vec::new();

    for candidate in default_values(data_type)? {
        if value_is_representable(data_type, &candidate)?
            && !value_in_any_range(&candidate, ranges, data_type)?
        {
            push_unique(&mut values, candidate);
        }
    }

    for range in ranges {
        if let Some(lower) = range.lower() {
            let lower_value = value_from_literal(data_type, lower.value())?;
            let candidate = if lower.inclusive() {
                step_value(&lower_value, false)
            } else {
                Some(lower_value)
            };
            if let Some(candidate) = candidate
                && value_is_representable(data_type, &candidate)?
                && !value_in_any_range(&candidate, ranges, data_type)?
            {
                push_unique(&mut values, candidate);
            }
        }

        if let Some(upper) = range.upper() {
            let upper_value = value_from_literal(data_type, upper.value())?;
            let candidate = if upper.inclusive() {
                step_value(&upper_value, true)
            } else {
                Some(upper_value)
            };
            if let Some(candidate) = candidate
                && value_is_representable(data_type, &candidate)?
                && !value_in_any_range(&candidate, ranges, data_type)?
            {
                push_unique(&mut values, candidate);
            }
        }
    }

    Ok(values)
}

fn value_in_any_range(
    value: &ProtocolValue,
    ranges: &[sql_semantic_protocol::ValueRange],
    data_type: &DataType,
) -> Result<bool, String> {
    for range in ranges {
        if value_in_range(value, range, data_type)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn value_is_representable(data_type: &DataType, value: &ProtocolValue) -> Result<bool, String> {
    match data_type {
        DataType::Nullable(inner) => {
            if matches!(value, ProtocolValue::Null) {
                Ok(true)
            } else {
                value_is_representable(inner, value)
            }
        }
        DataType::Decimal { precision, .. } => {
            let ProtocolValue::Decimal128(value) = value else {
                return Ok(false);
            };
            let limit = pow10_i128(precision.unwrap_or(38))?;
            Ok(*value > -limit && *value < limit)
        }
        DataType::FloatingPoint { bits } => match bits.unwrap_or(64) {
            0..=32 => Ok(matches!(
                value,
                ProtocolValue::Float32(bits) if f32::from_bits(*bits).is_finite()
            )),
            33..=64 => Ok(matches!(
                value,
                ProtocolValue::Float64(bits) if f64::from_bits(*bits).is_finite()
            )),
            _ => Ok(false),
        },
        DataType::String { length, fixed } => {
            let ProtocolValue::String(value) = value else {
                return Ok(false);
            };
            let width = value.chars().count() as u64;
            Ok(if *fixed {
                width == length.unwrap_or(width)
            } else {
                length.is_none_or(|length| width <= length)
            })
        }
        DataType::Binary { length, fixed } => {
            let ProtocolValue::Binary(value) = value else {
                return Ok(false);
            };
            let width = value.len() as u64;
            Ok(if *fixed {
                width == length.unwrap_or(width)
            } else {
                length.is_none_or(|length| width <= length)
            })
        }
        DataType::Time { precision } => {
            if precision.is_none_or(|value| value <= 6) {
                Ok(matches!(
                    value,
                    ProtocolValue::TimeMicroseconds(value)
                        if (0..=86_399_999_999).contains(value)
                ))
            } else if precision.is_some_and(|value| value <= 9) {
                Ok(matches!(
                    value,
                    ProtocolValue::TimeNanoseconds(value)
                        if (0..=86_399_999_999_999).contains(value)
                ))
            } else {
                Ok(false)
            }
        }
        DataType::Uuid => Ok(matches!(value, ProtocolValue::Binary(value) if value.len() == 16)),
        DataType::Enum { values } => Ok(matches!(
            value,
            ProtocolValue::String(value)
                if values.iter().any(|member| member.name() == value)
        )),
        DataType::Set { values } => Ok(matches!(
            value,
            ProtocolValue::String(value) if values.iter().any(|member| member == value)
        )),
        _ => Ok(build_array(data_type, std::slice::from_ref(value)).is_ok()),
    }
}

fn push_unique(values: &mut Vec<ProtocolValue>, value: ProtocolValue) {
    if !values.contains(&value) {
        values.push(value);
    }
}

fn nullable_candidates(
    inner: &DataType,
    domain: Option<&ValueDomain>,
) -> Result<Vec<ProtocolValue>, String> {
    match domain {
        None | Some(ValueDomain::Unbounded) => {
            let mut values = vec![ProtocolValue::Null];
            values.extend(default_values(inner)?);
            Ok(values)
        }
        Some(ValueDomain::Set(set)) => match set.mode() {
            SetMode::Include => {
                let mut values = Vec::new();
                for literal in set.values() {
                    if matches!(literal.value(), LiteralValue::Null) {
                        values.push(ProtocolValue::Null);
                    } else {
                        values.push(value_from_literal(inner, literal)?);
                    }
                }
                Ok(values)
            }
            SetMode::Exclude => {
                let excludes = set
                    .values()
                    .iter()
                    .filter(|literal| !matches!(literal.value(), LiteralValue::Null))
                    .map(|literal| value_from_literal(inner, literal))
                    .collect::<Result<Vec<_>, _>>()?;
                let excludes_null = set
                    .values()
                    .iter()
                    .any(|literal| matches!(literal.value(), LiteralValue::Null));
                let mut values = default_values(inner)?
                    .into_iter()
                    .filter(|value| !excludes.contains(value))
                    .collect::<Vec<_>>();
                if !excludes_null {
                    values.insert(0, ProtocolValue::Null);
                }
                if values.is_empty() {
                    values.push(non_excluded_value(inner, &excludes)?);
                }
                Ok(values)
            }
        },
        Some(ValueDomain::Ranges(ranges)) => range_candidates(inner, ranges.ranges()),
        Some(ValueDomain::Empty) => Ok(Vec::new()),
        Some(ValueDomain::Unknown(unknown)) => Err(unknown.reason().to_owned()),
        Some(_) => Err("unsupported protocol value-domain variant".to_owned()),
    }
}

fn non_null_candidates(
    data_type: &DataType,
    domain: Option<&ValueDomain>,
) -> Result<Vec<ProtocolValue>, String> {
    match domain {
        None | Some(ValueDomain::Unbounded) => default_values(data_type),
        Some(ValueDomain::Ranges(ranges)) => range_candidates(data_type, ranges.ranges()),
        Some(ValueDomain::Set(set)) => match set.mode() {
            SetMode::Include => set
                .values()
                .iter()
                .map(|literal| {
                    if matches!(literal.value(), LiteralValue::Null) {
                        Err("NULL cannot be generated for a non-nullable datatype".to_owned())
                    } else {
                        value_from_literal(data_type, literal)
                    }
                })
                .collect(),
            SetMode::Exclude => {
                let excludes = set
                    .values()
                    .iter()
                    .filter(|literal| !matches!(literal.value(), LiteralValue::Null))
                    .map(|literal| value_from_literal(data_type, literal))
                    .collect::<Result<Vec<_>, _>>()?;
                let mut values = default_values(data_type)?
                    .into_iter()
                    .filter(|value| !excludes.contains(value))
                    .collect::<Vec<_>>();
                if values.is_empty() {
                    values.push(non_excluded_value(data_type, &excludes)?);
                }
                Ok(values)
            }
        },
        Some(ValueDomain::Empty) => Ok(Vec::new()),
        Some(ValueDomain::Unknown(unknown)) => Err(unknown.reason().to_owned()),
        Some(_) => Err("unsupported protocol value-domain variant".to_owned()),
    }
}

pub(crate) fn sample_range_value<R: Rng + ?Sized>(
    data_type: &DataType,
    ranges: &[ValueRange],
    rng: &mut R,
) -> Result<ProtocolValue, String> {
    if ranges.is_empty() {
        return Err("protocol range domain has no ranges".to_owned());
    }

    let index = sample_random_index(rng, ranges.len())?;
    let range = ranges
        .get(index)
        .ok_or_else(|| "selected protocol range is missing".to_owned())?;
    let (lower, upper) = inclusive_range_bounds(data_type, range)?;
    sample_inclusive_interval(rng, &lower, &upper)
}

pub(crate) fn sample_rejected_range_value<R: Rng + ?Sized>(
    data_type: &DataType,
    ranges: &[ValueRange],
    rng: &mut R,
) -> Result<ProtocolValue, String> {
    let non_null_type = match data_type {
        DataType::Nullable(inner) => inner.as_ref(),
        _ => data_type,
    };
    let complements = complement_ranges(non_null_type, ranges)?;
    if complements.is_empty() {
        if matches!(data_type, DataType::Nullable(_)) {
            return Ok(ProtocolValue::Null);
        }
        return Err("protocol range has no representable complement".to_owned());
    }

    let index = sample_random_index(rng, complements.len())?;
    let (lower, upper) = complements
        .get(index)
        .ok_or_else(|| "selected rejected protocol range is missing".to_owned())?;
    sample_inclusive_interval(rng, lower, upper)
}

fn inclusive_range_bounds(
    data_type: &DataType,
    range: &ValueRange,
) -> Result<(ProtocolValue, ProtocolValue), String> {
    let non_null_type = match data_type {
        DataType::Nullable(inner) => inner.as_ref(),
        _ => data_type,
    };
    let (type_min, type_max) = ordered_type_bounds(non_null_type)?;

    let lower = match range.lower() {
        Some(bound) => {
            let value = value_from_literal(non_null_type, bound.value())?;
            if bound.inclusive() {
                value
            } else {
                step_value(&value, true).ok_or_else(|| {
                    "exclusive lower bound has no representable successor".to_owned()
                })?
            }
        }
        None => type_min.clone(),
    };
    let upper = match range.upper() {
        Some(bound) => {
            let value = value_from_literal(non_null_type, bound.value())?;
            if bound.inclusive() {
                value
            } else {
                step_value(&value, false).ok_or_else(|| {
                    "exclusive upper bound has no representable predecessor".to_owned()
                })?
            }
        }
        None => type_max.clone(),
    };

    if !value_is_representable(non_null_type, &lower)?
        || !value_is_representable(non_null_type, &upper)?
        || compare_values(&lower, &type_min)? == Ordering::Less
        || compare_values(&upper, &type_max)? == Ordering::Greater
        || compare_values(&lower, &upper)? == Ordering::Greater
    {
        return Err("protocol range contains no representable value for datatype".to_owned());
    }

    Ok((lower, upper))
}

fn complement_ranges(
    data_type: &DataType,
    ranges: &[ValueRange],
) -> Result<Vec<(ProtocolValue, ProtocolValue)>, String> {
    let (type_min, type_max) = ordered_type_bounds(data_type)?;
    let mut allowed = Vec::with_capacity(ranges.len());

    for range in ranges {
        let bounds = inclusive_range_bounds(data_type, range)?;
        let position = allowed
            .iter()
            .position(|(lower, _): &(ProtocolValue, ProtocolValue)| {
                compare_values(&bounds.0, lower).is_ok_and(|order| order == Ordering::Less)
            })
            .unwrap_or(allowed.len());
        allowed.insert(position, bounds);
    }

    let mut result = Vec::new();
    let mut cursor = Some(type_min);
    for (lower, upper) in allowed {
        let Some(current) = cursor.clone() else {
            break;
        };

        if compare_values(&lower, &current)? == Ordering::Greater
            && let Some(gap_upper) = step_value(&lower, false)
            && compare_values(&current, &gap_upper)? != Ordering::Greater
        {
            result.push((current.clone(), gap_upper));
        }

        if compare_values(&upper, &current)? != Ordering::Less {
            cursor = if compare_values(&upper, &type_max)? == Ordering::Less {
                step_value(&upper, true)
            } else {
                None
            };
        }
    }

    if let Some(current) = cursor
        && compare_values(&current, &type_max)? != Ordering::Greater
    {
        result.push((current, type_max));
    }

    Ok(result)
}

fn ordered_type_bounds(data_type: &DataType) -> Result<(ProtocolValue, ProtocolValue), String> {
    match data_type {
        DataType::SignedInteger { bits } => match bits.unwrap_or(64) {
            0..=8 => Ok((ProtocolValue::Int8(i8::MIN), ProtocolValue::Int8(i8::MAX))),
            9..=16 => Ok((
                ProtocolValue::Int16(i16::MIN),
                ProtocolValue::Int16(i16::MAX),
            )),
            17..=32 => Ok((
                ProtocolValue::Int32(i32::MIN),
                ProtocolValue::Int32(i32::MAX),
            )),
            33..=64 => Ok((
                ProtocolValue::Int64(i64::MIN),
                ProtocolValue::Int64(i64::MAX),
            )),
            bits => Err(format!("signed_integer({bits})")),
        },
        DataType::UnsignedInteger { bits } => match bits.unwrap_or(64) {
            0..=8 => Ok((ProtocolValue::UInt8(u8::MIN), ProtocolValue::UInt8(u8::MAX))),
            9..=16 => Ok((
                ProtocolValue::UInt16(u16::MIN),
                ProtocolValue::UInt16(u16::MAX),
            )),
            17..=32 => Ok((
                ProtocolValue::UInt32(u32::MIN),
                ProtocolValue::UInt32(u32::MAX),
            )),
            33..=64 => Ok((
                ProtocolValue::UInt64(u64::MIN),
                ProtocolValue::UInt64(u64::MAX),
            )),
            bits => Err(format!("unsigned_integer({bits})")),
        },
        DataType::Decimal { precision, scale } => {
            let precision = precision.unwrap_or(38);
            decimal_shape(precision, scale.unwrap_or(0))?;
            let limit = pow10_i128(precision)?
                .checked_sub(1)
                .ok_or_else(|| "decimal precision underflow".to_owned())?;
            Ok((
                ProtocolValue::Decimal128(-limit),
                ProtocolValue::Decimal128(limit),
            ))
        }
        DataType::FloatingPoint { bits } if bits.unwrap_or(64) <= 32 => Ok((
            ProtocolValue::Float32((-f32::MAX).to_bits()),
            ProtocolValue::Float32(f32::MAX.to_bits()),
        )),
        DataType::FloatingPoint { bits } if bits.unwrap_or(64) <= 64 => Ok((
            ProtocolValue::Float64((-f64::MAX).to_bits()),
            ProtocolValue::Float64(f64::MAX.to_bits()),
        )),
        DataType::FloatingPoint { bits } => Err(format!("floating_point({})", bits.unwrap_or(64))),
        DataType::Date => Ok((
            ProtocolValue::Date32(i32::MIN),
            ProtocolValue::Date32(i32::MAX),
        )),
        DataType::Time { precision } if precision.is_none_or(|value| value <= 6) => Ok((
            ProtocolValue::TimeMicroseconds(0),
            ProtocolValue::TimeMicroseconds(86_399_999_999),
        )),
        DataType::Time { precision } if precision.is_some_and(|value| value <= 9) => Ok((
            ProtocolValue::TimeNanoseconds(0),
            ProtocolValue::TimeNanoseconds(86_399_999_999_999),
        )),
        DataType::Time { precision } => Err(format!("time({})", precision.unwrap_or_default())),
        DataType::Timestamp { precision } if precision.is_none_or(|value| value <= 6) => Ok((
            ProtocolValue::TimestampMicroseconds(i64::MIN),
            ProtocolValue::TimestampMicroseconds(i64::MAX),
        )),
        DataType::Timestamp { precision } if precision.is_some_and(|value| value <= 9) => Ok((
            ProtocolValue::TimestampNanoseconds(i64::MIN),
            ProtocolValue::TimestampNanoseconds(i64::MAX),
        )),
        DataType::Timestamp { precision } => {
            Err(format!("timestamp({})", precision.unwrap_or_default()))
        }
        DataType::Nullable(inner) => ordered_type_bounds(inner),
        _ => Err("datatype does not support ordered protocol ranges".to_owned()),
    }
}

fn sample_inclusive_interval<R: Rng + ?Sized>(
    rng: &mut R,
    lower: &ProtocolValue,
    upper: &ProtocolValue,
) -> Result<ProtocolValue, String> {
    macro_rules! sample_signed {
        ($low:expr, $high:expr, $variant:ident, $target:ty) => {{
            let sampled = sample_i128_inclusive(rng, i128::from(*$low), i128::from(*$high))?;
            let value = <$target>::try_from(sampled)
                .map_err(|_| "sampled signed value is outside datatype range".to_owned())?;
            Ok(ProtocolValue::$variant(value))
        }};
    }
    macro_rules! sample_unsigned {
        ($low:expr, $high:expr, $variant:ident, $target:ty) => {{
            let sampled = sample_u128_inclusive(rng, u128::from(*$low), u128::from(*$high))?;
            let value = <$target>::try_from(sampled)
                .map_err(|_| "sampled unsigned value is outside datatype range".to_owned())?;
            Ok(ProtocolValue::$variant(value))
        }};
    }

    match (lower, upper) {
        (ProtocolValue::Int8(low), ProtocolValue::Int8(high)) => {
            sample_signed!(low, high, Int8, i8)
        }
        (ProtocolValue::Int16(low), ProtocolValue::Int16(high)) => {
            sample_signed!(low, high, Int16, i16)
        }
        (ProtocolValue::Int32(low), ProtocolValue::Int32(high)) => {
            sample_signed!(low, high, Int32, i32)
        }
        (ProtocolValue::Int64(low), ProtocolValue::Int64(high)) => {
            sample_signed!(low, high, Int64, i64)
        }
        (ProtocolValue::UInt8(low), ProtocolValue::UInt8(high)) => {
            sample_unsigned!(low, high, UInt8, u8)
        }
        (ProtocolValue::UInt16(low), ProtocolValue::UInt16(high)) => {
            sample_unsigned!(low, high, UInt16, u16)
        }
        (ProtocolValue::UInt32(low), ProtocolValue::UInt32(high)) => {
            sample_unsigned!(low, high, UInt32, u32)
        }
        (ProtocolValue::UInt64(low), ProtocolValue::UInt64(high)) => {
            sample_unsigned!(low, high, UInt64, u64)
        }
        (ProtocolValue::Decimal128(low), ProtocolValue::Decimal128(high)) => Ok(
            ProtocolValue::Decimal128(sample_i128_inclusive(rng, *low, *high)?),
        ),
        (ProtocolValue::Date32(low), ProtocolValue::Date32(high)) => {
            sample_signed!(low, high, Date32, i32)
        }
        (ProtocolValue::TimeMicroseconds(low), ProtocolValue::TimeMicroseconds(high)) => {
            Ok(ProtocolValue::TimeMicroseconds(sample_i128_inclusive(
                rng,
                i128::from(*low),
                i128::from(*high),
            )? as i64))
        }
        (ProtocolValue::TimeNanoseconds(low), ProtocolValue::TimeNanoseconds(high)) => {
            Ok(ProtocolValue::TimeNanoseconds(sample_i128_inclusive(
                rng,
                i128::from(*low),
                i128::from(*high),
            )? as i64))
        }
        (ProtocolValue::TimestampMicroseconds(low), ProtocolValue::TimestampMicroseconds(high)) => {
            Ok(ProtocolValue::TimestampMicroseconds(
                i64::try_from(sample_i128_inclusive(
                    rng,
                    i128::from(*low),
                    i128::from(*high),
                )?)
                .map_err(|_| "sampled timestamp is outside datatype range".to_owned())?,
            ))
        }
        (ProtocolValue::TimestampNanoseconds(low), ProtocolValue::TimestampNanoseconds(high)) => {
            Ok(ProtocolValue::TimestampNanoseconds(
                i64::try_from(sample_i128_inclusive(
                    rng,
                    i128::from(*low),
                    i128::from(*high),
                )?)
                .map_err(|_| "sampled timestamp is outside datatype range".to_owned())?,
            ))
        }
        (ProtocolValue::Float32(low), ProtocolValue::Float32(high)) => {
            let low = f32::from_bits(*low);
            let high = f32::from_bits(*high);
            let unit = (rng.next_u64() as f64) / (u64::MAX as f64);
            let value = ((f64::from(low) * (1.0 - unit)) + (f64::from(high) * unit)) as f32;
            Ok(ProtocolValue::Float32(value.clamp(low, high).to_bits()))
        }
        (ProtocolValue::Float64(low), ProtocolValue::Float64(high)) => {
            let low = f64::from_bits(*low);
            let high = f64::from_bits(*high);
            let unit = (rng.next_u64() as f64) / (u64::MAX as f64);
            let value = (low * (1.0 - unit)) + (high * unit);
            Ok(ProtocolValue::Float64(value.clamp(low, high).to_bits()))
        }
        _ => Err("range bounds do not share a supported ordered datatype".to_owned()),
    }
}

fn sample_i128_inclusive<R: Rng + ?Sized>(
    rng: &mut R,
    lower: i128,
    upper: i128,
) -> Result<i128, String> {
    if lower > upper {
        return Err("range lower bound exceeds upper bound".to_owned());
    }

    const SIGN: u128 = 1_u128 << 127;
    let lower = (lower as u128) ^ SIGN;
    let upper = (upper as u128) ^ SIGN;
    let sampled = sample_u128_inclusive(rng, lower, upper)?;
    Ok((sampled ^ SIGN) as i128)
}

fn sample_u128_inclusive<R: Rng + ?Sized>(
    rng: &mut R,
    lower: u128,
    upper: u128,
) -> Result<u128, String> {
    if lower > upper {
        return Err("range lower bound exceeds upper bound".to_owned());
    }

    let span = upper - lower;
    if span == u128::MAX {
        return Ok(random_u128(rng));
    }
    let width = span + 1;
    Ok(lower + sample_u128_below(rng, width)?)
}

fn sample_u128_below<R: Rng + ?Sized>(rng: &mut R, upper: u128) -> Result<u128, String> {
    if upper == 0 {
        return Err("cannot sample from an empty range".to_owned());
    }

    let zone = u128::MAX - (u128::MAX % upper);
    loop {
        let candidate = random_u128(rng);
        if candidate < zone {
            return Ok(candidate % upper);
        }
    }
}

fn random_u128<R: Rng + ?Sized>(rng: &mut R) -> u128 {
    (u128::from(rng.next_u64()) << 64) | u128::from(rng.next_u64())
}

fn sample_random_index<R: Rng + ?Sized>(rng: &mut R, len: usize) -> Result<usize, String> {
    let upper = u64::try_from(len).map_err(|_| "range count is too large to sample".to_owned())?;
    if upper == 0 {
        return Err("cannot sample from an empty range list".to_owned());
    }
    let zone = u64::MAX - (u64::MAX % upper);
    loop {
        let candidate = rng.next_u64();
        if candidate < zone {
            return usize::try_from(candidate % upper)
                .map_err(|_| "sampled range index does not fit usize".to_owned());
        }
    }
}

fn range_candidates(
    data_type: &DataType,
    ranges: &[sql_semantic_protocol::ValueRange],
) -> Result<Vec<ProtocolValue>, String> {
    let mut values = Vec::with_capacity(ranges.len());
    for range in ranges {
        let candidate = if let Some(lower) = range.lower() {
            let value = value_from_literal(data_type, lower.value())?;
            if lower.inclusive() {
                value
            } else {
                step_value(&value, true).ok_or_else(|| {
                    "exclusive lower bound has no representable successor".to_owned()
                })?
            }
        } else if let Some(upper) = range.upper() {
            let value = value_from_literal(data_type, upper.value())?;
            if upper.inclusive() {
                value
            } else {
                step_value(&value, false).ok_or_else(|| {
                    "exclusive upper bound has no representable predecessor".to_owned()
                })?
            }
        } else {
            representative_value(data_type)?
        };

        if !value_in_range(&candidate, range, data_type)? {
            return Err("protocol range contains no representable value for datatype".to_owned());
        }
        values.push(candidate);
    }
    Ok(values)
}

fn value_in_range(
    value: &ProtocolValue,
    range: &sql_semantic_protocol::ValueRange,
    data_type: &DataType,
) -> Result<bool, String> {
    if let Some(lower) = range.lower() {
        let lower_value = value_from_literal(data_type, lower.value())?;
        let order = compare_values(value, &lower_value)?;
        if order == Ordering::Less || (order == Ordering::Equal && !lower.inclusive()) {
            return Ok(false);
        }
    }
    if let Some(upper) = range.upper() {
        let upper_value = value_from_literal(data_type, upper.value())?;
        let order = compare_values(value, &upper_value)?;
        if order == Ordering::Greater || (order == Ordering::Equal && !upper.inclusive()) {
            return Ok(false);
        }
    }
    Ok(true)
}

fn default_values(data_type: &DataType) -> Result<Vec<ProtocolValue>, String> {
    match data_type {
        DataType::Boolean => Ok(vec![
            ProtocolValue::Boolean(false),
            ProtocolValue::Boolean(true),
        ]),
        DataType::SignedInteger { bits } => signed_defaults(bits.unwrap_or(64)),
        DataType::UnsignedInteger { bits } => unsigned_defaults(bits.unwrap_or(64)),
        DataType::Decimal { precision, scale } => {
            let precision = precision.unwrap_or(38);
            let scale = scale.unwrap_or(0);
            decimal_shape(precision, scale)?;
            let limit = pow10_i128(precision)?
                .checked_sub(1)
                .ok_or_else(|| "decimal precision underflow".to_owned())?;
            Ok(vec![
                ProtocolValue::Decimal128(-limit),
                ProtocolValue::Decimal128(0),
                ProtocolValue::Decimal128(limit),
            ])
        }
        DataType::FloatingPoint { bits } => {
            if bits.unwrap_or(64) <= 32 {
                Ok([-1.0_f32, 0.0, 1.0]
                    .into_iter()
                    .map(|value| ProtocolValue::Float32(value.to_bits()))
                    .collect())
            } else if bits.unwrap_or(64) <= 64 {
                Ok([-1.0_f64, 0.0, 1.0]
                    .into_iter()
                    .map(|value| ProtocolValue::Float64(value.to_bits()))
                    .collect())
            } else {
                Err(format!("floating_point({})", bits.unwrap_or(64)))
            }
        }
        DataType::String { length, fixed } => {
            if *fixed {
                let length = usize::try_from(length.unwrap_or(1))
                    .map_err(|_| "fixed string length is too large".to_owned())?;
                if length > MAX_FIXED_COLLECTION_LENGTH as usize {
                    return Err(format!("fixed string length {length} is too large"));
                }
                Ok(vec![ProtocolValue::String("x".repeat(length))])
            } else {
                let max = length.unwrap_or(u64::MAX);
                let mut values = vec![ProtocolValue::String(String::new())];
                if max > 0 {
                    values.push(ProtocolValue::String(
                        "value".chars().take(max as usize).collect(),
                    ));
                }
                Ok(values)
            }
        }
        DataType::Binary { length, fixed } => {
            if *fixed {
                let length = usize::try_from(length.unwrap_or(1))
                    .map_err(|_| "fixed binary length is too large".to_owned())?;
                if length > MAX_FIXED_COLLECTION_LENGTH as usize {
                    return Err(format!("fixed binary length {length} is too large"));
                }
                Ok(vec![ProtocolValue::Binary(vec![0xA5; length])])
            } else {
                let max = length.unwrap_or(u64::MAX);
                let mut values = vec![ProtocolValue::Binary(Vec::new())];
                if max > 0 {
                    values.push(ProtocolValue::Binary(vec![0, 0xFF]));
                    if max == 1 {
                        values[1] = ProtocolValue::Binary(vec![0xFF]);
                    }
                }
                Ok(values)
            }
        }
        DataType::Date => Ok(vec![
            ProtocolValue::Date32(-1),
            ProtocolValue::Date32(0),
            ProtocolValue::Date32(1),
        ]),
        DataType::Time { precision } => {
            if precision.is_none_or(|value| value <= 6) {
                Ok(vec![
                    ProtocolValue::TimeMicroseconds(0),
                    ProtocolValue::TimeMicroseconds(43_200_000_000),
                    ProtocolValue::TimeMicroseconds(86_399_999_999),
                ])
            } else if precision.is_some_and(|value| value <= 9) {
                Ok(vec![
                    ProtocolValue::TimeNanoseconds(0),
                    ProtocolValue::TimeNanoseconds(43_200_000_000_000),
                    ProtocolValue::TimeNanoseconds(86_399_999_999_999),
                ])
            } else {
                Err(format!("time({})", precision.unwrap_or_default()))
            }
        }
        DataType::Timestamp { precision } => {
            if precision.is_none_or(|value| value <= 6) {
                Ok(vec![
                    ProtocolValue::TimestampMicroseconds(-1),
                    ProtocolValue::TimestampMicroseconds(0),
                    ProtocolValue::TimestampMicroseconds(1),
                ])
            } else if precision.is_some_and(|value| value <= 9) {
                Ok(vec![
                    ProtocolValue::TimestampNanoseconds(-1),
                    ProtocolValue::TimestampNanoseconds(0),
                    ProtocolValue::TimestampNanoseconds(1),
                ])
            } else {
                Err(format!("timestamp({})", precision.unwrap_or_default()))
            }
        }
        DataType::Uuid => Ok(vec![
            ProtocolValue::Binary(vec![0; 16]),
            ProtocolValue::Binary(vec![0xFF; 16]),
        ]),
        DataType::Json => Ok(vec![
            ProtocolValue::String("null".to_owned()),
            ProtocolValue::String("{}".to_owned()),
            ProtocolValue::String("[]".to_owned()),
        ]),
        DataType::Array { element, length } => {
            let element = element
                .as_deref()
                .ok_or_else(|| "array without an element datatype".to_owned())?;
            let representative = representative_value(element)?;
            match length {
                Some(length) => {
                    if *length > MAX_FIXED_COLLECTION_LENGTH {
                        return Err(format!("array[{length}]"));
                    }
                    Ok(vec![ProtocolValue::List(vec![
                        representative;
                        usize::try_from(*length)
                            .map_err(|_| format!(
                                "array[{length}]"
                            ))?
                    ])])
                }
                None => Ok(vec![
                    ProtocolValue::List(Vec::new()),
                    ProtocolValue::List(vec![representative]),
                ]),
            }
        }
        DataType::Map { key, value } => {
            let key = representative_non_null_value(key)?;
            let value = representative_value(value)?;
            Ok(vec![
                ProtocolValue::Map(Vec::new()),
                ProtocolValue::Map(vec![(key, value)]),
            ])
        }
        DataType::Struct { fields } => Ok(vec![ProtocolValue::Struct(
            fields
                .iter()
                .map(|field| representative_value(field.data_type()))
                .collect::<Result<Vec<_>, _>>()?,
        )]),
        DataType::Enum { values } => {
            if values.is_empty() {
                return Err("enum has no declared values".to_owned());
            }
            Ok(values
                .iter()
                .map(|value| ProtocolValue::String(value.name().to_owned()))
                .collect())
        }
        DataType::Set { values } => {
            if values.is_empty() {
                return Err("set has no declared values".to_owned());
            }
            Ok(values.iter().cloned().map(ProtocolValue::String).collect())
        }
        DataType::Nullable(inner) => {
            let mut values = vec![ProtocolValue::Null];
            values.extend(default_values(inner)?);
            Ok(values)
        }
        _ => Err(data_type.kind().to_owned()),
    }
}

fn representative_value(data_type: &DataType) -> Result<ProtocolValue, String> {
    default_values(data_type)?
        .into_iter()
        .next()
        .ok_or_else(|| format!("{} has no representative value", data_type.kind()))
}

fn representative_non_null_value(data_type: &DataType) -> Result<ProtocolValue, String> {
    match data_type {
        DataType::Nullable(inner) => representative_non_null_value(inner),
        _ => default_values(data_type)?
            .into_iter()
            .find(|value| !matches!(value, ProtocolValue::Null))
            .ok_or_else(|| format!("{} has no non-null representative value", data_type.kind())),
    }
}

fn non_excluded_value(
    data_type: &DataType,
    excludes: &[ProtocolValue],
) -> Result<ProtocolValue, String> {
    if let DataType::Enum { values } = data_type {
        return values
            .iter()
            .map(|value| ProtocolValue::String(value.name().to_owned()))
            .find(|value| !excludes.contains(value))
            .ok_or_else(|| "enum exclusion removes every declared value".to_owned());
    }
    if let DataType::Set { values } = data_type {
        return values
            .iter()
            .cloned()
            .map(ProtocolValue::String)
            .find(|value| !excludes.contains(value))
            .ok_or_else(|| "set exclusion removes every declared value".to_owned());
    }
    if matches!(data_type, DataType::Boolean) {
        return [ProtocolValue::Boolean(false), ProtocolValue::Boolean(true)]
            .into_iter()
            .find(|value| !excludes.contains(value))
            .ok_or_else(|| "boolean exclusion removes every value".to_owned());
    }
    if matches!(data_type, DataType::String { .. }) {
        return non_excluded_string(data_type, excludes);
    }
    if matches!(data_type, DataType::Binary { .. }) {
        return non_excluded_binary(data_type, excludes);
    }
    if matches!(data_type, DataType::Uuid) {
        return non_excluded_fixed_binary(16, excludes, "UUID");
    }
    if matches!(data_type, DataType::Json) {
        for index in 0..=excludes.len().saturating_add(1) {
            let candidate = ProtocolValue::String(index.to_string());
            if !excludes.contains(&candidate) {
                return Ok(candidate);
            }
        }
        return Err("JSON exclusion leaves no generated alternative".to_owned());
    }

    for candidate in default_values(data_type)? {
        if !excludes.contains(&candidate) && value_is_representable(data_type, &candidate)? {
            return Ok(candidate);
        }
    }

    let start = representative_non_null_value(data_type)?;
    for up in [true, false] {
        let mut value = start.clone();
        for _ in 0..excludes.len().saturating_add(2) {
            if !excludes.contains(&value) && value_is_representable(data_type, &value)? {
                return Ok(value);
            }
            let Some(next) = step_value(&value, up) else {
                break;
            };
            if !value_is_representable(data_type, &next)? {
                break;
            }
            value = next;
        }
    }

    Err("excluded domain has no representable alternative".to_owned())
}

fn non_excluded_string(
    data_type: &DataType,
    excludes: &[ProtocolValue],
) -> Result<ProtocolValue, String> {
    let DataType::String { length, fixed } = data_type else {
        return Err("expected string datatype".to_owned());
    };
    let width = if *fixed {
        usize::try_from(length.unwrap_or(1))
            .map_err(|_| "fixed string length is too large".to_owned())?
    } else {
        usize::from(length.unwrap_or(1) > 0)
    };

    if width == 0 {
        let candidate = ProtocolValue::String(String::new());
        return (!excludes.contains(&candidate))
            .then_some(candidate)
            .ok_or_else(|| "string exclusion removes every representable value".to_owned());
    }

    for index in 0..=excludes.len().saturating_add(1) {
        let character = indexed_scalar(index)
            .ok_or_else(|| "string exclusion search exceeded Unicode scalar values".to_owned())?;
        let value = if *fixed {
            character.to_string().repeat(width)
        } else {
            character.to_string()
        };
        let candidate = ProtocolValue::String(value);
        if !excludes.contains(&candidate) {
            return Ok(candidate);
        }
    }

    Err("string exclusion leaves no generated alternative".to_owned())
}

fn indexed_scalar(index: usize) -> Option<char> {
    let offset = u32::try_from(index).ok()?;
    let code = 0x21_u32.checked_add(offset)?;
    let code = if code >= 0xD800 {
        code.checked_add(0x800)?
    } else {
        code
    };
    char::from_u32(code)
}

fn non_excluded_binary(
    data_type: &DataType,
    excludes: &[ProtocolValue],
) -> Result<ProtocolValue, String> {
    let DataType::Binary { length, fixed } = data_type else {
        return Err("expected binary datatype".to_owned());
    };
    let max_width = usize::try_from(length.unwrap_or(std::mem::size_of::<usize>() as u64))
        .map_err(|_| "binary length is too large".to_owned())?;

    if *fixed {
        return non_excluded_fixed_binary(max_width, excludes, "binary");
    }

    for width in 0..=max_width.min(std::mem::size_of::<usize>()) {
        if let Ok(value) = non_excluded_fixed_binary(width, excludes, "binary") {
            return Ok(value);
        }
    }

    Err("binary exclusion leaves no generated alternative".to_owned())
}

fn non_excluded_fixed_binary(
    width: usize,
    excludes: &[ProtocolValue],
    label: &str,
) -> Result<ProtocolValue, String> {
    for index in 0..=excludes.len().saturating_add(1) {
        let Some(bytes) = counter_bytes(index, width) else {
            break;
        };
        let candidate = ProtocolValue::Binary(bytes);
        if !excludes.contains(&candidate) {
            return Ok(candidate);
        }
    }
    Err(format!(
        "{label} exclusion removes every generated alternative"
    ))
}

fn counter_bytes(index: usize, width: usize) -> Option<Vec<u8>> {
    if width == 0 {
        return (index == 0).then(Vec::new);
    }

    let mut remaining = index;
    let mut bytes = vec![0; width];
    for byte in bytes.iter_mut().rev() {
        *byte = (remaining & 0xff) as u8;
        remaining >>= 8;
    }
    (remaining == 0).then_some(bytes)
}

fn signed_defaults(bits: u16) -> Result<Vec<ProtocolValue>, String> {
    match bits {
        0..=8 => Ok(vec![
            ProtocolValue::Int8(i8::MIN),
            ProtocolValue::Int8(0),
            ProtocolValue::Int8(i8::MAX),
        ]),
        9..=16 => Ok(vec![
            ProtocolValue::Int16(i16::MIN),
            ProtocolValue::Int16(0),
            ProtocolValue::Int16(i16::MAX),
        ]),
        17..=32 => Ok(vec![
            ProtocolValue::Int32(i32::MIN),
            ProtocolValue::Int32(0),
            ProtocolValue::Int32(i32::MAX),
        ]),
        33..=64 => Ok(vec![
            ProtocolValue::Int64(i64::MIN),
            ProtocolValue::Int64(0),
            ProtocolValue::Int64(i64::MAX),
        ]),
        _ => Err(format!("signed_integer({bits})")),
    }
}

fn unsigned_defaults(bits: u16) -> Result<Vec<ProtocolValue>, String> {
    match bits {
        0..=8 => Ok(vec![
            ProtocolValue::UInt8(u8::MIN),
            ProtocolValue::UInt8(1),
            ProtocolValue::UInt8(u8::MAX),
        ]),
        9..=16 => Ok(vec![
            ProtocolValue::UInt16(u16::MIN),
            ProtocolValue::UInt16(1),
            ProtocolValue::UInt16(u16::MAX),
        ]),
        17..=32 => Ok(vec![
            ProtocolValue::UInt32(u32::MIN),
            ProtocolValue::UInt32(1),
            ProtocolValue::UInt32(u32::MAX),
        ]),
        33..=64 => Ok(vec![
            ProtocolValue::UInt64(u64::MIN),
            ProtocolValue::UInt64(1),
            ProtocolValue::UInt64(u64::MAX),
        ]),
        _ => Err(format!("unsigned_integer({bits})")),
    }
}

fn value_from_literal(
    data_type: &DataType,
    literal: &LiteralExpression,
) -> Result<ProtocolValue, String> {
    if matches!(literal.value(), LiteralValue::Null) {
        return match data_type {
            DataType::Nullable(_) => Ok(ProtocolValue::Null),
            _ => Err("NULL literal requires a nullable datatype".to_owned()),
        };
    }

    match data_type {
        DataType::Nullable(inner) => value_from_literal(inner, literal),
        DataType::Boolean => match literal.value() {
            LiteralValue::Boolean(value) => Ok(ProtocolValue::Boolean(*value)),
            _ => Err("expected boolean literal".to_owned()),
        },
        DataType::SignedInteger { bits } => {
            let value = literal_number(literal)?
                .parse::<i128>()
                .map_err(|_| "integer literal is outside supported range".to_owned())?;
            match bits.unwrap_or(64) {
                0..=8 => {
                    Ok(ProtocolValue::Int8(i8::try_from(value).map_err(|_| {
                        "integer literal overflows INT8".to_owned()
                    })?))
                }
                9..=16 => Ok(ProtocolValue::Int16(
                    i16::try_from(value)
                        .map_err(|_| "integer literal overflows INT16".to_owned())?,
                )),
                17..=32 => Ok(ProtocolValue::Int32(
                    i32::try_from(value)
                        .map_err(|_| "integer literal overflows INT32".to_owned())?,
                )),
                33..=64 => Ok(ProtocolValue::Int64(
                    i64::try_from(value)
                        .map_err(|_| "integer literal overflows INT64".to_owned())?,
                )),
                bits => Err(format!("signed_integer({bits})")),
            }
        }
        DataType::UnsignedInteger { bits } => {
            let value = literal_number(literal)?
                .parse::<u128>()
                .map_err(|_| "unsigned integer literal is outside supported range".to_owned())?;
            match bits.unwrap_or(64) {
                0..=8 => {
                    Ok(ProtocolValue::UInt8(u8::try_from(value).map_err(|_| {
                        "integer literal overflows UINT8".to_owned()
                    })?))
                }
                9..=16 => Ok(ProtocolValue::UInt16(
                    u16::try_from(value)
                        .map_err(|_| "integer literal overflows UINT16".to_owned())?,
                )),
                17..=32 => Ok(ProtocolValue::UInt32(
                    u32::try_from(value)
                        .map_err(|_| "integer literal overflows UINT32".to_owned())?,
                )),
                33..=64 => Ok(ProtocolValue::UInt64(
                    u64::try_from(value)
                        .map_err(|_| "integer literal overflows UINT64".to_owned())?,
                )),
                bits => Err(format!("unsigned_integer({bits})")),
            }
        }
        DataType::Decimal { precision, scale } => {
            let precision = precision.unwrap_or(38);
            let scale = scale.unwrap_or(0);
            Ok(ProtocolValue::Decimal128(parse_decimal_scaled(
                literal_number(literal)?,
                precision,
                scale,
            )?))
        }
        DataType::FloatingPoint { bits } => {
            let value = literal_number(literal)?
                .parse::<f64>()
                .map_err(|_| "invalid floating-point literal".to_owned())?;
            if !value.is_finite() {
                return Err("non-finite floating-point literal is unsupported".to_owned());
            }
            if bits.unwrap_or(64) <= 32 {
                let value = value as f32;
                if !value.is_finite() {
                    return Err("floating-point literal overflows FLOAT32".to_owned());
                }
                Ok(ProtocolValue::Float32(value.to_bits()))
            } else if bits.unwrap_or(64) <= 64 {
                Ok(ProtocolValue::Float64(value.to_bits()))
            } else {
                Err(format!("floating_point({})", bits.unwrap_or(64)))
            }
        }
        DataType::String { length, fixed } => {
            let text = literal_text(literal)?;
            let char_count = text.chars().count() as u64;
            if length.is_some_and(|length| char_count > length) {
                return Err("string literal exceeds declared length".to_owned());
            }
            if *fixed {
                let width = length.unwrap_or(char_count);
                let mut value = text.to_owned();
                value.extend(std::iter::repeat_n(
                    ' ',
                    usize::try_from(width.saturating_sub(char_count))
                        .map_err(|_| "fixed string length is too large".to_owned())?,
                ));
                Ok(ProtocolValue::String(value))
            } else {
                Ok(ProtocolValue::String(text.to_owned()))
            }
        }
        DataType::Binary { length, fixed } => {
            let mut value = literal_text(literal)?.as_bytes().to_vec();
            let value_len = value.len() as u64;
            if length.is_some_and(|length| value_len > length) {
                return Err("binary literal exceeds declared length".to_owned());
            }
            if *fixed {
                let width = length.unwrap_or(value_len);
                value.resize(
                    usize::try_from(width)
                        .map_err(|_| "fixed binary length is too large".to_owned())?,
                    0,
                );
            }
            Ok(ProtocolValue::Binary(value))
        }
        DataType::Date => {
            let date = NaiveDate::parse_from_str(literal_text(literal)?, "%Y-%m-%d")
                .map_err(|_| "invalid DATE literal".to_owned())?;
            let epoch = NaiveDate::from_ymd_opt(1970, 1, 1).expect("Unix epoch is valid");
            let days = date.signed_duration_since(epoch).num_days();
            Ok(ProtocolValue::Date32(i32::try_from(days).map_err(
                |_| "DATE literal is outside Arrow range".to_owned(),
            )?))
        }
        DataType::Time { precision } => {
            let time = NaiveTime::parse_from_str(literal_text(literal)?, "%H:%M:%S%.f")
                .map_err(|_| "invalid TIME literal".to_owned())?;
            let nanos = i64::from(time.num_seconds_from_midnight()) * 1_000_000_000
                + i64::from(time.nanosecond());
            if precision.is_none_or(|value| value <= 6) {
                if nanos % 1_000 != 0 {
                    return Err("TIME literal exceeds declared microsecond precision".to_owned());
                }
                Ok(ProtocolValue::TimeMicroseconds(nanos / 1_000))
            } else if precision.is_some_and(|value| value <= 9) {
                Ok(ProtocolValue::TimeNanoseconds(nanos))
            } else {
                Err(format!("time({})", precision.unwrap_or_default()))
            }
        }
        DataType::Timestamp { precision } => {
            let timestamp = parse_timestamp(literal_text(literal)?)?;
            if precision.is_none_or(|value| value <= 6) {
                Ok(ProtocolValue::TimestampMicroseconds(
                    timestamp.timestamp_micros(),
                ))
            } else if precision.is_some_and(|value| value <= 9) {
                Ok(ProtocolValue::TimestampNanoseconds(
                    timestamp.timestamp_nanos_opt().ok_or_else(|| {
                        "TIMESTAMP literal is outside nanosecond range".to_owned()
                    })?,
                ))
            } else {
                Err(format!("timestamp({})", precision.unwrap_or_default()))
            }
        }
        DataType::Uuid => Ok(ProtocolValue::Binary(parse_uuid(literal_text(literal)?)?)),
        DataType::Json => Ok(ProtocolValue::String(literal_text(literal)?.to_owned())),
        DataType::Enum { values } => {
            let text = literal_text(literal)?;
            if values.iter().any(|value| value.name() == text) {
                Ok(ProtocolValue::String(text.to_owned()))
            } else {
                Err(format!("enum literal {text:?} is not declared"))
            }
        }
        DataType::Set { values } => {
            let text = literal_text(literal)?;
            if values.iter().any(|value| value == text) {
                Ok(ProtocolValue::String(text.to_owned()))
            } else {
                Err(format!("set literal {text:?} is not declared"))
            }
        }
        DataType::Array { .. } | DataType::Map { .. } | DataType::Struct { .. } => {
            Err("nested literals are not represented by protocol scalar domains".to_owned())
        }
        _ => Err(data_type.kind().to_owned()),
    }
}

fn literal_number(literal: &LiteralExpression) -> Result<&str, String> {
    match (literal.literal_type(), literal.value()) {
        (LiteralType::Integer | LiteralType::Decimal, LiteralValue::Number(value)) => Ok(value),
        _ => Err("expected numeric literal".to_owned()),
    }
}

fn literal_text(literal: &LiteralExpression) -> Result<&str, String> {
    match literal.value() {
        LiteralValue::Text(value) => Ok(value),
        _ => Err("expected text literal".to_owned()),
    }
}

fn parse_timestamp(text: &str) -> Result<DateTime<Utc>, String> {
    if let Ok(timestamp) = DateTime::parse_from_rfc3339(text) {
        return Ok(timestamp.with_timezone(&Utc));
    }
    for format in ["%Y-%m-%d %H:%M:%S%.f", "%Y-%m-%dT%H:%M:%S%.f"] {
        if let Ok(timestamp) = NaiveDateTime::parse_from_str(text, format) {
            return Ok(Utc.from_utc_datetime(&timestamp));
        }
    }
    if let Ok(date) = NaiveDate::parse_from_str(text, "%Y-%m-%d") {
        let timestamp = date
            .and_hms_opt(0, 0, 0)
            .ok_or_else(|| "invalid TIMESTAMP literal".to_owned())?;
        return Ok(Utc.from_utc_datetime(&timestamp));
    }
    Err("invalid TIMESTAMP literal".to_owned())
}

fn parse_uuid(text: &str) -> Result<Vec<u8>, String> {
    let compact = text
        .chars()
        .filter(|character| *character != '-')
        .collect::<String>();
    if compact.len() != 32 || !compact.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("invalid UUID literal".to_owned());
    }
    (0..16)
        .map(|index| {
            u8::from_str_radix(&compact[index * 2..index * 2 + 2], 16)
                .map_err(|_| "invalid UUID literal".to_owned())
        })
        .collect()
}

fn decimal_shape(precision: u64, scale: u64) -> Result<(), String> {
    if precision == 0 || precision > 38 || scale > precision {
        Err(format!("decimal({precision},{scale})"))
    } else {
        Ok(())
    }
}

fn parse_decimal_scaled(text: &str, precision: u64, scale: u64) -> Result<i128, String> {
    decimal_shape(precision, scale)?;
    let text = text.trim();
    let (negative, unsigned) = match text.as_bytes().first() {
        Some(b'-') => (true, &text[1..]),
        Some(b'+') => (false, &text[1..]),
        _ => (false, text),
    };
    let exponent_index = unsigned.find('e').or_else(|| unsigned.find('E'));
    let (mantissa, exponent) = match exponent_index {
        Some(index) => (
            &unsigned[..index],
            unsigned[index + 1..]
                .parse::<i32>()
                .map_err(|_| "invalid decimal exponent".to_owned())?,
        ),
        None => (unsigned, 0),
    };
    let (whole, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    if whole.is_empty() && fraction.is_empty() {
        return Err("invalid decimal literal".to_owned());
    }
    let digits = format!("{whole}{fraction}");
    if !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err("invalid decimal literal".to_owned());
    }

    let mut coefficient = if digits.is_empty() {
        0
    } else {
        digits
            .parse::<i128>()
            .map_err(|_| "decimal literal exceeds Decimal128".to_owned())?
    };
    let decimal_shift = i32::try_from(scale)
        .map_err(|_| "decimal scale is too large".to_owned())?
        + exponent
        - i32::try_from(fraction.len()).map_err(|_| "decimal literal is too long".to_owned())?;

    if decimal_shift >= 0 {
        coefficient = coefficient
            .checked_mul(pow10_i128(decimal_shift as u64)?)
            .ok_or_else(|| "decimal literal exceeds Decimal128".to_owned())?;
    } else {
        let divisor = pow10_i128(u64::from(decimal_shift.unsigned_abs()))?;
        if coefficient % divisor != 0 {
            return Err("decimal literal has more fractional precision than declared".to_owned());
        }
        coefficient /= divisor;
    }

    if negative {
        coefficient = coefficient
            .checked_neg()
            .ok_or_else(|| "decimal literal exceeds Decimal128".to_owned())?;
    }

    let limit = pow10_i128(precision)?;
    if coefficient <= -limit || coefficient >= limit {
        return Err(format!("decimal literal exceeds precision {precision}"));
    }
    Ok(coefficient)
}

fn pow10_i128(power: u64) -> Result<i128, String> {
    let power = u32::try_from(power).map_err(|_| "decimal exponent is too large".to_owned())?;
    10_i128
        .checked_pow(power)
        .ok_or_else(|| "decimal exponent exceeds Decimal128".to_owned())
}

fn step_value(value: &ProtocolValue, up: bool) -> Option<ProtocolValue> {
    macro_rules! checked_step {
        ($value:expr, $variant:ident) => {
            if up {
                $value.checked_add(1).map(ProtocolValue::$variant)
            } else {
                $value.checked_sub(1).map(ProtocolValue::$variant)
            }
        };
    }

    match value {
        ProtocolValue::Int8(value) => checked_step!(*value, Int8),
        ProtocolValue::Int16(value) => checked_step!(*value, Int16),
        ProtocolValue::Int32(value) => checked_step!(*value, Int32),
        ProtocolValue::Int64(value) => checked_step!(*value, Int64),
        ProtocolValue::UInt8(value) => checked_step!(*value, UInt8),
        ProtocolValue::UInt16(value) => checked_step!(*value, UInt16),
        ProtocolValue::UInt32(value) => checked_step!(*value, UInt32),
        ProtocolValue::UInt64(value) => checked_step!(*value, UInt64),
        ProtocolValue::Decimal128(value) => checked_step!(*value, Decimal128),
        ProtocolValue::Date32(value) => checked_step!(*value, Date32),
        ProtocolValue::TimeMicroseconds(value) => checked_step!(*value, TimeMicroseconds),
        ProtocolValue::TimeNanoseconds(value) => checked_step!(*value, TimeNanoseconds),
        ProtocolValue::TimestampMicroseconds(value) => checked_step!(*value, TimestampMicroseconds),
        ProtocolValue::TimestampNanoseconds(value) => checked_step!(*value, TimestampNanoseconds),
        ProtocolValue::Float32(bits) => {
            next_f32(f32::from_bits(*bits), up).map(|value| ProtocolValue::Float32(value.to_bits()))
        }
        ProtocolValue::Float64(bits) => {
            next_f64(f64::from_bits(*bits), up).map(|value| ProtocolValue::Float64(value.to_bits()))
        }
        _ => None,
    }
}

fn next_f32(value: f32, up: bool) -> Option<f32> {
    if value.is_nan() || (up && value == f32::INFINITY) || (!up && value == f32::NEG_INFINITY) {
        return None;
    }
    if value == 0.0 {
        return Some(if up {
            f32::from_bits(1)
        } else {
            f32::from_bits((1_u32 << 31) | 1)
        });
    }
    let bits = value.to_bits();
    Some(f32::from_bits(if (value > 0.0) == up {
        bits + 1
    } else {
        bits - 1
    }))
}

fn next_f64(value: f64, up: bool) -> Option<f64> {
    if value.is_nan() || (up && value == f64::INFINITY) || (!up && value == f64::NEG_INFINITY) {
        return None;
    }
    if value == 0.0 {
        return Some(if up {
            f64::from_bits(1)
        } else {
            f64::from_bits((1_u64 << 63) | 1)
        });
    }
    let bits = value.to_bits();
    Some(f64::from_bits(if (value > 0.0) == up {
        bits + 1
    } else {
        bits - 1
    }))
}

fn compare_values(left: &ProtocolValue, right: &ProtocolValue) -> Result<Ordering, String> {
    macro_rules! compare {
        ($left:expr, $right:expr) => {
            Ok($left.cmp($right))
        };
    }

    match (left, right) {
        (ProtocolValue::Int8(left), ProtocolValue::Int8(right)) => compare!(left, right),
        (ProtocolValue::Int16(left), ProtocolValue::Int16(right)) => compare!(left, right),
        (ProtocolValue::Int32(left), ProtocolValue::Int32(right)) => compare!(left, right),
        (ProtocolValue::Int64(left), ProtocolValue::Int64(right)) => compare!(left, right),
        (ProtocolValue::UInt8(left), ProtocolValue::UInt8(right)) => compare!(left, right),
        (ProtocolValue::UInt16(left), ProtocolValue::UInt16(right)) => compare!(left, right),
        (ProtocolValue::UInt32(left), ProtocolValue::UInt32(right)) => compare!(left, right),
        (ProtocolValue::UInt64(left), ProtocolValue::UInt64(right)) => compare!(left, right),
        (ProtocolValue::Decimal128(left), ProtocolValue::Decimal128(right)) => {
            compare!(left, right)
        }
        (ProtocolValue::Date32(left), ProtocolValue::Date32(right)) => compare!(left, right),
        (ProtocolValue::TimeMicroseconds(left), ProtocolValue::TimeMicroseconds(right)) => {
            compare!(left, right)
        }
        (ProtocolValue::TimeNanoseconds(left), ProtocolValue::TimeNanoseconds(right)) => {
            compare!(left, right)
        }
        (
            ProtocolValue::TimestampMicroseconds(left),
            ProtocolValue::TimestampMicroseconds(right),
        ) => compare!(left, right),
        (ProtocolValue::TimestampNanoseconds(left), ProtocolValue::TimestampNanoseconds(right)) => {
            compare!(left, right)
        }
        (ProtocolValue::Float32(left), ProtocolValue::Float32(right)) => f32::from_bits(*left)
            .partial_cmp(&f32::from_bits(*right))
            .ok_or_else(|| "NaN is not orderable".to_owned()),
        (ProtocolValue::Float64(left), ProtocolValue::Float64(right)) => f64::from_bits(*left)
            .partial_cmp(&f64::from_bits(*right))
            .ok_or_else(|| "NaN is not orderable".to_owned()),
        _ => Err("datatype does not support ordered protocol ranges".to_owned()),
    }
}

pub(crate) fn build_array(
    data_type: &DataType,
    values: &[ProtocolValue],
) -> Result<ArrayRef, String> {
    let arrow_type = arrow_data_type(data_type)?;
    let mut builder = make_builder(&arrow_type, values.len());
    for value in values {
        append_value(builder.as_mut(), &arrow_type, value)?;
    }
    let array = builder.finish();
    if array.data_type() != &arrow_type {
        return Err(format!(
            "Arrow builder produced {} instead of {arrow_type}",
            array.data_type()
        ));
    }
    Ok(array)
}

fn append_value(
    builder: &mut dyn ArrayBuilder,
    data_type: &ArrowDataType,
    value: &ProtocolValue,
) -> Result<(), String> {
    if matches!(value, ProtocolValue::Null) {
        return append_null(builder, data_type);
    }

    match (data_type, value) {
        (ArrowDataType::Boolean, ProtocolValue::Boolean(value)) => {
            builder_mut::<BooleanBuilder>(builder)?.append_value(*value);
        }
        (ArrowDataType::Int8, ProtocolValue::Int8(value)) => {
            builder_mut::<Int8Builder>(builder)?.append_value(*value);
        }
        (ArrowDataType::Int16, ProtocolValue::Int16(value)) => {
            builder_mut::<Int16Builder>(builder)?.append_value(*value);
        }
        (ArrowDataType::Int32, ProtocolValue::Int32(value)) => {
            builder_mut::<Int32Builder>(builder)?.append_value(*value);
        }
        (ArrowDataType::Int64, ProtocolValue::Int64(value)) => {
            builder_mut::<Int64Builder>(builder)?.append_value(*value);
        }
        (ArrowDataType::UInt8, ProtocolValue::UInt8(value)) => {
            builder_mut::<UInt8Builder>(builder)?.append_value(*value);
        }
        (ArrowDataType::UInt16, ProtocolValue::UInt16(value)) => {
            builder_mut::<UInt16Builder>(builder)?.append_value(*value);
        }
        (ArrowDataType::UInt32, ProtocolValue::UInt32(value)) => {
            builder_mut::<UInt32Builder>(builder)?.append_value(*value);
        }
        (ArrowDataType::UInt64, ProtocolValue::UInt64(value)) => {
            builder_mut::<UInt64Builder>(builder)?.append_value(*value);
        }
        (ArrowDataType::Float32, ProtocolValue::Float32(value)) => {
            builder_mut::<Float32Builder>(builder)?.append_value(f32::from_bits(*value));
        }
        (ArrowDataType::Float64, ProtocolValue::Float64(value)) => {
            builder_mut::<Float64Builder>(builder)?.append_value(f64::from_bits(*value));
        }
        (ArrowDataType::Decimal128(_, _), ProtocolValue::Decimal128(value)) => {
            builder_mut::<Decimal128Builder>(builder)?.append_value(*value);
        }
        (ArrowDataType::Utf8, ProtocolValue::String(value)) => {
            builder_mut::<StringBuilder>(builder)?.append_value(value);
        }
        (ArrowDataType::Binary, ProtocolValue::Binary(value)) => {
            builder_mut::<BinaryBuilder>(builder)?.append_value(value);
        }
        (ArrowDataType::FixedSizeBinary(_), ProtocolValue::Binary(value)) => {
            builder_mut::<FixedSizeBinaryBuilder>(builder)?
                .append_value(value)
                .map_err(|error| error.to_string())?;
        }
        (ArrowDataType::Date32, ProtocolValue::Date32(value)) => {
            builder_mut::<Date32Builder>(builder)?.append_value(*value);
        }
        (ArrowDataType::Time64(TimeUnit::Microsecond), ProtocolValue::TimeMicroseconds(value)) => {
            builder_mut::<Time64MicrosecondBuilder>(builder)?.append_value(*value);
        }
        (ArrowDataType::Time64(TimeUnit::Nanosecond), ProtocolValue::TimeNanoseconds(value)) => {
            builder_mut::<Time64NanosecondBuilder>(builder)?.append_value(*value);
        }
        (
            ArrowDataType::Timestamp(TimeUnit::Microsecond, _),
            ProtocolValue::TimestampMicroseconds(value),
        ) => {
            builder_mut::<TimestampMicrosecondBuilder>(builder)?.append_value(*value);
        }
        (
            ArrowDataType::Timestamp(TimeUnit::Nanosecond, _),
            ProtocolValue::TimestampNanoseconds(value),
        ) => {
            builder_mut::<TimestampNanosecondBuilder>(builder)?.append_value(*value);
        }
        (ArrowDataType::List(field), ProtocolValue::List(values)) => {
            let list = builder_mut::<ListBuilder<Box<dyn ArrayBuilder>>>(builder)?;
            for value in values {
                append_value(list.values().as_mut(), field.data_type(), value)?;
            }
            list.append(true);
        }
        (ArrowDataType::FixedSizeList(field, size), ProtocolValue::List(values)) => {
            if values.len()
                != usize::try_from(*size).map_err(|_| "negative fixed list size".to_owned())?
            {
                return Err(format!(
                    "fixed list expects {size} values, got {}",
                    values.len()
                ));
            }
            let list = builder_mut::<FixedSizeListBuilder<Box<dyn ArrayBuilder>>>(builder)?;
            for value in values {
                append_value(list.values().as_mut(), field.data_type(), value)?;
            }
            list.append(true);
        }
        (ArrowDataType::Struct(fields), ProtocolValue::Struct(values)) => {
            if fields.len() != values.len() {
                return Err(format!(
                    "struct expects {} values, got {}",
                    fields.len(),
                    values.len()
                ));
            }
            let struct_builder = builder_mut::<StructBuilder>(builder)?;
            for ((field_builder, field), value) in struct_builder
                .field_builders_mut()
                .iter_mut()
                .zip(fields.iter())
                .zip(values)
            {
                append_value(field_builder.as_mut(), field.data_type(), value)?;
            }
            struct_builder.append(true);
        }
        (ArrowDataType::Map(field, _), ProtocolValue::Map(entries)) => {
            let ArrowDataType::Struct(fields) = field.data_type() else {
                return Err("Arrow map entry field is not a struct".to_owned());
            };
            let map =
                builder_mut::<MapBuilder<Box<dyn ArrayBuilder>, Box<dyn ArrayBuilder>>>(builder)?;
            for (key, value) in entries {
                if matches!(key, ProtocolValue::Null) {
                    return Err("map keys cannot be NULL".to_owned());
                }
                append_value(map.keys().as_mut(), fields[0].data_type(), key)?;
                append_value(map.values().as_mut(), fields[1].data_type(), value)?;
            }
            map.append(true).map_err(|error| error.to_string())?;
        }
        _ => {
            return Err(format!(
                "value {value:?} does not match Arrow datatype {data_type}"
            ));
        }
    }
    Ok(())
}

fn append_null(builder: &mut dyn ArrayBuilder, data_type: &ArrowDataType) -> Result<(), String> {
    match data_type {
        ArrowDataType::Boolean => builder_mut::<BooleanBuilder>(builder)?.append_null(),
        ArrowDataType::Int8 => builder_mut::<Int8Builder>(builder)?.append_null(),
        ArrowDataType::Int16 => builder_mut::<Int16Builder>(builder)?.append_null(),
        ArrowDataType::Int32 => builder_mut::<Int32Builder>(builder)?.append_null(),
        ArrowDataType::Int64 => builder_mut::<Int64Builder>(builder)?.append_null(),
        ArrowDataType::UInt8 => builder_mut::<UInt8Builder>(builder)?.append_null(),
        ArrowDataType::UInt16 => builder_mut::<UInt16Builder>(builder)?.append_null(),
        ArrowDataType::UInt32 => builder_mut::<UInt32Builder>(builder)?.append_null(),
        ArrowDataType::UInt64 => builder_mut::<UInt64Builder>(builder)?.append_null(),
        ArrowDataType::Float32 => builder_mut::<Float32Builder>(builder)?.append_null(),
        ArrowDataType::Float64 => builder_mut::<Float64Builder>(builder)?.append_null(),
        ArrowDataType::Decimal128(_, _) => {
            builder_mut::<Decimal128Builder>(builder)?.append_null();
        }
        ArrowDataType::Utf8 => builder_mut::<StringBuilder>(builder)?.append_null(),
        ArrowDataType::Binary => builder_mut::<BinaryBuilder>(builder)?.append_null(),
        ArrowDataType::FixedSizeBinary(_) => {
            builder_mut::<FixedSizeBinaryBuilder>(builder)?.append_null();
        }
        ArrowDataType::Date32 => builder_mut::<Date32Builder>(builder)?.append_null(),
        ArrowDataType::Time64(TimeUnit::Microsecond) => {
            builder_mut::<Time64MicrosecondBuilder>(builder)?.append_null();
        }
        ArrowDataType::Time64(TimeUnit::Nanosecond) => {
            builder_mut::<Time64NanosecondBuilder>(builder)?.append_null();
        }
        ArrowDataType::Timestamp(TimeUnit::Microsecond, _) => {
            builder_mut::<TimestampMicrosecondBuilder>(builder)?.append_null();
        }
        ArrowDataType::Timestamp(TimeUnit::Nanosecond, _) => {
            builder_mut::<TimestampNanosecondBuilder>(builder)?.append_null();
        }
        ArrowDataType::List(_) => {
            builder_mut::<ListBuilder<Box<dyn ArrayBuilder>>>(builder)?.append(false);
        }
        ArrowDataType::FixedSizeList(field, size) => {
            let list = builder_mut::<FixedSizeListBuilder<Box<dyn ArrayBuilder>>>(builder)?;
            for _ in 0..*size {
                append_null(list.values().as_mut(), field.data_type())?;
            }
            list.append(false);
        }
        ArrowDataType::Struct(fields) => {
            let struct_builder = builder_mut::<StructBuilder>(builder)?;
            for (field_builder, field) in struct_builder
                .field_builders_mut()
                .iter_mut()
                .zip(fields.iter())
            {
                append_null(field_builder.as_mut(), field.data_type())?;
            }
            struct_builder.append(false);
        }
        ArrowDataType::Map(_, _) => {
            builder_mut::<MapBuilder<Box<dyn ArrayBuilder>, Box<dyn ArrayBuilder>>>(builder)?
                .append(false)
                .map_err(|error| error.to_string())?;
        }
        _ => {
            return Err(format!(
                "NULL is unsupported for Arrow datatype {data_type}"
            ));
        }
    }
    Ok(())
}

fn builder_mut<T: 'static>(builder: &mut dyn ArrayBuilder) -> Result<&mut T, String> {
    builder.as_any_mut().downcast_mut::<T>().ok_or_else(|| {
        format!(
            "unexpected Arrow builder for {}",
            std::any::type_name::<T>()
        )
    })
}

fn is_nullable(data_type: &DataType) -> bool {
    matches!(data_type, DataType::Nullable(_))
}

#[cfg(test)]
mod tests {
    use arrow_array::{
        Array, Decimal128Array, Int64Array, ListArray, MapArray, StringArray, StructArray,
        UInt64Array,
    };
    use sql_semantic_protocol::{DataType, DataTypeField, parse_data_type};

    use super::{ProtocolValue, arrow_data_type, build_array, default_values};

    #[test]
    fn integer_boundaries_are_lossless() {
        let signed = DataType::SignedInteger { bits: Some(64) };
        let unsigned = DataType::UnsignedInteger { bits: Some(64) };

        let signed_values = default_values(&signed).expect("INT64 values should be supported");
        let signed_array = build_array(&signed, &signed_values).expect("INT64 array should build");
        let signed_array = signed_array
            .as_any()
            .downcast_ref::<Int64Array>()
            .expect("INT64 storage");
        assert_eq!(signed_array.value(0), i64::MIN);
        assert_eq!(signed_array.value(2), i64::MAX);

        let unsigned_values = default_values(&unsigned).expect("UINT64 values should be supported");
        let unsigned_array =
            build_array(&unsigned, &unsigned_values).expect("UINT64 array should build");
        let unsigned_array = unsigned_array
            .as_any()
            .downcast_ref::<UInt64Array>()
            .expect("UINT64 storage");
        assert_eq!(unsigned_array.value(0), u64::MIN);
        assert_eq!(unsigned_array.value(2), u64::MAX);
    }

    #[test]
    fn decimal_boundaries_preserve_scale() {
        let data_type = DataType::Decimal {
            precision: Some(10),
            scale: Some(2),
        };
        let values = default_values(&data_type).expect("DECIMAL should be supported");
        let array = build_array(&data_type, &values).expect("DECIMAL array should build");
        let array = array
            .as_any()
            .downcast_ref::<Decimal128Array>()
            .expect("Decimal128 storage");

        assert_eq!(array.value(0), -9_999_999_999);
        assert_eq!(array.value(1), 0);
        assert_eq!(array.value(2), 9_999_999_999);
        assert_eq!(array.data_type(), &arrow_data_type(&data_type).unwrap());
    }

    #[test]
    fn nullable_values_materialize_nulls_deterministically() {
        let data_type = DataType::Nullable(Box::new(DataType::SignedInteger { bits: Some(64) }));
        let values = default_values(&data_type).expect("nullable INT64 should be supported");
        let array = build_array(&data_type, &values).expect("nullable array should build");

        assert!(array.is_null(0));
        assert!(!array.is_null(1));
    }

    #[test]
    fn nested_values_keep_arrow_types() {
        let data_type = DataType::Struct {
            fields: vec![
                DataTypeField::new(
                    Some("items".to_owned()),
                    DataType::Array {
                        element: Some(Box::new(DataType::SignedInteger { bits: Some(64) })),
                        length: None,
                    },
                ),
                DataTypeField::new(
                    Some("attributes".to_owned()),
                    DataType::Map {
                        key: Box::new(DataType::String {
                            length: None,
                            fixed: false,
                        }),
                        value: Box::new(DataType::SignedInteger { bits: Some(64) }),
                    },
                ),
            ],
        };

        let values = default_values(&data_type).expect("nested type should be supported");
        let array = build_array(&data_type, &values).expect("nested array should build");
        let struct_array = array
            .as_any()
            .downcast_ref::<StructArray>()
            .expect("struct storage");
        assert_eq!(struct_array.len(), 1);
        assert!(
            struct_array
                .column(0)
                .as_any()
                .downcast_ref::<ListArray>()
                .is_some()
        );
        assert!(
            struct_array
                .column(1)
                .as_any()
                .downcast_ref::<MapArray>()
                .is_some()
        );
    }

    #[test]
    fn enum_values_are_limited_to_declared_members() {
        let data_type =
            parse_data_type("ENUM('ready', 'done')", "mysql").expect("enum should normalize");
        let values = default_values(&data_type).expect("enum should be supported");
        assert_eq!(
            values,
            vec![
                ProtocolValue::String("ready".to_owned()),
                ProtocolValue::String("done".to_owned()),
            ]
        );

        let array = build_array(&data_type, &values).expect("enum array should build");
        let strings = array
            .as_any()
            .downcast_ref::<StringArray>()
            .expect("enum uses UTF-8 storage");
        assert_eq!(strings.value(0), "ready");
        assert_eq!(strings.value(1), "done");
    }
}
