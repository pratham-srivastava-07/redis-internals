use crate::helpers::data_parse::{
    decode_arrays, decode_bulk_string, decode_errors, decode_integer, decode_simple_string,
};

pub const MAX_FRAME_BYTES: usize = 64 * 1024 * 1024;
// AOF normalization may add an absolute expiration to a full-size client frame.
pub const MAX_DECODE_BYTES: usize = MAX_FRAME_BYTES + 1024;
pub const MAX_ARRAY_ELEMENTS: usize = 1024;
const MAX_DEPTH: usize = 32;

#[derive(Debug, PartialEq)]
pub enum DecodeError {
    Incomplete,
    Invalid,
}

#[derive(Debug, PartialEq)]
pub enum RespValue {
    Simple(Vec<u8>),
    Error(Vec<u8>),
    Integer(i64),
    Bulk(Option<Vec<u8>>),
    Array(Option<Vec<RespValue>>),
}

pub fn decode_one(data: &[u8]) -> Result<(RespValue, usize), DecodeError> {
    decode_at(data, 0)
}

pub(super) fn decode_at(data: &[u8], depth: usize) -> Result<(RespValue, usize), DecodeError> {
    if depth > MAX_DEPTH {
        return Err(DecodeError::Invalid);
    }
    match data.first() {
        None => Err(DecodeError::Incomplete),
        Some(b'+') => decode_simple_string(data),
        Some(b'-') => decode_errors(data),
        Some(b':') => decode_integer(data),
        Some(b'$') => decode_bulk_string(data),
        Some(b'*') => decode_arrays(data, depth),
        _ => Err(DecodeError::Invalid),
    }
}

pub fn read_line(data: &[u8]) -> Result<(&[u8], usize), DecodeError> {
    for (pos, &byte) in data.iter().enumerate() {
        if pos > 65536 {
            return Err(DecodeError::Invalid);
        }
        if byte == b'\n' {
            return Err(DecodeError::Invalid);
        }
        if byte == b'\r' {
            match data.get(pos + 1) {
                None => return Err(DecodeError::Incomplete),
                Some(b'\n') => return Ok((&data[..pos], pos + 2)),
                _ => return Err(DecodeError::Invalid),
            }
        }
    }
    Err(DecodeError::Incomplete)
}

pub fn read_length(data: &[u8]) -> Result<(i64, usize), DecodeError> {
    let (line, consumed) = read_line(data)?;
    if line.is_empty() || line.len() > 20 || line.starts_with(b"+") {
        return Err(DecodeError::Invalid);
    }
    let text = std::str::from_utf8(line).map_err(|_| DecodeError::Invalid)?;
    let value = text.parse::<i64>().map_err(|_| DecodeError::Invalid)?;
    Ok((value, consumed))
}
