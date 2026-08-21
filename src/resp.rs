use crate::helpers::utils::{DecodeError, decode_one};

#[warn(dead_code)]
pub fn _decode_data(data: &[u8]) -> Result<Box<dyn std::any::Any>, DecodeError> {
    // Ok(Box::new(42u32))

    if data.is_empty() {
        return Err(DecodeError::Incomplete);
    }

    let (value, _bytes_consumed) = decode_one(data)?;

    Ok(value)
}


pub fn decode_array_string(data: &[u8]) -> Result<(Vec<String>, usize), DecodeError> {
    let (value, bytes_con) = decode_one(data)?;

    let ts = match value.downcast::<Vec<Box<dyn std::any::Any>>>() {
        Ok(v) => v,
        Err(_) => return Err(DecodeError::Invalid) // decoded, but not an array
    };

    let mut tokens: Vec<String> = Vec::with_capacity(ts.len());

    for t in ts.iter() {
        match t.downcast_ref::<String>() {
            Some(s) => tokens.push(s.clone()),
            None => return Err(DecodeError::Invalid)
        }
    };

    Ok((tokens, bytes_con))

}