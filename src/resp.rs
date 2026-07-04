use crate::helpers::utils::{DecodeError, decode_one};

pub fn decode_data(data: &[u8]) -> Result<Box<dyn std::any::Any>, DecodeError> {
    // Ok(Box::new(42u32))

    if data.is_empty() {
        return Err(DecodeError);
    }

    let (value, _bytes_consumed) = decode_one(data)?;

    Ok(value)
}

