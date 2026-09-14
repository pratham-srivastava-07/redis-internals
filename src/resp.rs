use crate::helpers::utils::{DecodeError, RespValue, decode_one};

pub fn decode_array_string(data: &[u8]) -> Result<(Vec<Vec<u8>>, usize), DecodeError> {
    let (value, consumed) = decode_one(data)?;
    let RespValue::Array(Some(elements)) = value else {
        return Err(DecodeError::Invalid);
    };
    let mut tokens = Vec::with_capacity(elements.len());
    for element in elements {
        match element {
            RespValue::Bulk(Some(bytes)) => tokens.push(bytes),
            _ => return Err(DecodeError::Invalid),
        }
    }
    Ok((tokens, consumed))
}
