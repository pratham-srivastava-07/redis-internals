use crate::helpers::utils::{
    DecodeError, MAX_ARRAY_ELEMENTS, MAX_DECODE_BYTES, RespValue, decode_at, read_length, read_line,
};

pub fn decode_simple_string(data: &[u8]) -> Result<(RespValue, usize), DecodeError> {
    let (line, consumed) = read_line(&data[1..])?;
    Ok((RespValue::Simple(line.to_vec()), consumed + 1))
}

pub fn decode_errors(data: &[u8]) -> Result<(RespValue, usize), DecodeError> {
    let (line, consumed) = read_line(&data[1..])?;
    Ok((RespValue::Error(line.to_vec()), consumed + 1))
}

pub fn decode_integer(data: &[u8]) -> Result<(RespValue, usize), DecodeError> {
    let (number, consumed) = read_length(&data[1..])?;
    Ok((RespValue::Integer(number), consumed + 1))
}

pub fn decode_bulk_string(data: &[u8]) -> Result<(RespValue, usize), DecodeError> {
    let (length, consumed) = read_length(&data[1..])?;
    let start = consumed + 1;
    if length == -1 {
        return Ok((RespValue::Bulk(None), start));
    }
    let length = usize::try_from(length).map_err(|_| DecodeError::Invalid)?;
    let end = start.checked_add(length).ok_or(DecodeError::Invalid)?;
    let total = end
        .checked_add(2)
        .filter(|n| *n <= MAX_DECODE_BYTES)
        .ok_or(DecodeError::Invalid)?;
    if data.len() < total {
        return Err(DecodeError::Incomplete);
    }
    if &data[end..total] != b"\r\n" {
        return Err(DecodeError::Invalid);
    }
    Ok((RespValue::Bulk(Some(data[start..end].to_vec())), total))
}

pub fn decode_arrays(data: &[u8], depth: usize) -> Result<(RespValue, usize), DecodeError> {
    let (count, consumed) = read_length(&data[1..])?;
    let mut pos = consumed + 1;
    if count == -1 {
        return Ok((RespValue::Array(None), pos));
    }
    let count = usize::try_from(count)
        .ok()
        .filter(|n| *n <= MAX_ARRAY_ELEMENTS)
        .ok_or(DecodeError::Invalid)?;
    let mut elements = Vec::new();
    for _ in 0..count {
        let remaining = data.get(pos..).ok_or(DecodeError::Incomplete)?;
        let (value, consumed) = decode_at(remaining, depth + 1)?;
        pos = pos
            .checked_add(consumed)
            .filter(|n| *n <= MAX_DECODE_BYTES)
            .ok_or(DecodeError::Invalid)?;
        elements.push(value);
    }
    Ok((RespValue::Array(Some(elements)), pos))
}
