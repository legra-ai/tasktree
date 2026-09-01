use super::{
    SPAN_BYTE_LEN,
    SPAN_HEX_LEN,
    TRACE_HEX_LEN,
    TaskIdentityError,
    ZERO_SPAN_BYTES,
    decode_hex_array,
    invalid_format,
};

/// Return the trace hex portion when `raw` is a valid W3C traceparent.
pub(super) fn trace_hex(raw: &str) -> Result<Option<&str>, TaskIdentityError> {
    let mut parts = raw.split('-');
    let Some(version) = parts.next() else {
        return Ok(None);
    };
    let Some(trace_id) = parts.next() else {
        return Ok(None);
    };
    let Some(parent_id) = parts.next() else {
        return Ok(None);
    };
    let Some(flags) = parts.next() else {
        return Ok(None);
    };
    if parts.next().is_some() {
        return Err(invalid_format(raw, "traceparent has too many fields"));
    }
    if version.len() != 2 || trace_id.len() != TRACE_HEX_LEN || parent_id.len() != SPAN_HEX_LEN {
        return Ok(None);
    }
    validate_hex_fields(raw, version, parent_id, flags)?;
    validate_parent_id(raw, parent_id)?;
    Ok(Some(trace_id))
}

fn validate_hex_fields(
    raw: &str,
    version: &str,
    parent_id: &str,
    flags: &str,
) -> Result<(), TaskIdentityError> {
    if flags.len() != 2 {
        return Err(invalid_format(
            raw,
            "traceparent flags must be 2 hex characters",
        ));
    }
    if !version
        .chars()
        .all(|character| character.is_ascii_hexdigit())
        || !parent_id
            .chars()
            .all(|character| character.is_ascii_hexdigit())
        || !flags.chars().all(|character| character.is_ascii_hexdigit())
    {
        return Err(invalid_format(raw, "traceparent fields must be hex"));
    }
    Ok(())
}

fn validate_parent_id(raw: &str, parent_id: &str) -> Result<(), TaskIdentityError> {
    let parent_bytes = decode_hex_array::<SPAN_BYTE_LEN, SPAN_HEX_LEN>(
        parent_id,
        raw,
        "traceparent parent ID must be 16 hex characters",
    )?;
    if parent_bytes == ZERO_SPAN_BYTES {
        return Err(invalid_format(
            raw,
            "traceparent parent ID must not be all zero",
        ));
    }
    Ok(())
}
