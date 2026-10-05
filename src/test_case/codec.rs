use super::TestCaseError;

pub(super) const METADATA_HEADER: &str = "sql-tdg-test-case-metadata-v1";
pub(super) const APPROVED_RESULT_HEADER: &str = "sql-tdg-approved-result-v1";

pub(super) fn record(key: &str, value: &str) -> String {
    format!("{key}\t{value}")
}

pub(super) fn next_value<'a, I>(lines: &mut I, expected_key: &str) -> Result<&'a str, TestCaseError>
where
    I: Iterator<Item = &'a str>,
{
    let line = lines
        .next()
        .ok_or_else(|| invalid_metadata(format!("missing {expected_key} record")))?;
    let mut fields = line.split('\t');
    let key = fields
        .next()
        .ok_or_else(|| invalid_metadata(format!("empty {expected_key} record")))?;
    let value = fields
        .next()
        .ok_or_else(|| invalid_metadata(format!("{expected_key} record has no value")))?;
    if key != expected_key || fields.next().is_some() {
        return Err(invalid_metadata(format!(
            "expected {expected_key} record, got {line:?}"
        )));
    }
    Ok(value)
}

pub(super) fn parse_count(value: &str, field: &str) -> Result<usize, TestCaseError> {
    let count = parse_u64(value, field)?;
    usize::try_from(count).map_err(|_| {
        invalid_metadata(format!(
            "{field} count {count} cannot be represented on this platform"
        ))
    })
}

pub(super) fn parse_u64(value: &str, field: &str) -> Result<u64, TestCaseError> {
    value.parse::<u64>().map_err(|_| {
        invalid_metadata(format!(
            "{field} value {value:?} is not an unsigned integer"
        ))
    })
}

pub(super) fn encode_string(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len().saturating_mul(2));
    for byte in value.as_bytes() {
        encoded.push(
            char::from_digit(u32::from(*byte >> 4), 16)
                .expect("upper nibble is always a hexadecimal digit"),
        );
        encoded.push(
            char::from_digit(u32::from(*byte & 0x0f), 16)
                .expect("lower nibble is always a hexadecimal digit"),
        );
    }
    encoded
}

pub(super) fn decode_string(encoded: &str) -> Result<String, TestCaseError> {
    let mut characters = encoded.chars();
    let mut bytes = Vec::with_capacity(encoded.len() / 2);
    while let Some(high) = characters.next() {
        let low = characters
            .next()
            .ok_or_else(|| invalid_metadata("hex-encoded string has odd length"))?;
        let high = high
            .to_digit(16)
            .ok_or_else(|| invalid_metadata("hex-encoded string contains a non-hex digit"))?;
        let low = low
            .to_digit(16)
            .ok_or_else(|| invalid_metadata("hex-encoded string contains a non-hex digit"))?;
        let value = (high << 4) | low;
        bytes.push(u8::try_from(value).expect("two hexadecimal digits always fit into one byte"));
    }
    String::from_utf8(bytes).map_err(|_| invalid_metadata("hex-encoded string is not valid UTF-8"))
}

pub(super) fn invalid_metadata(message: impl Into<String>) -> TestCaseError {
    TestCaseError::InvalidMetadata {
        message: message.into(),
    }
}
