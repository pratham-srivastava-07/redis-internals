// all the decode_data_type helpers live here 

use crate::helpers::utils::{DecodeError, decode_one, read_length};

pub fn decode_simple_string(data: &[u8]) -> Result<(Box<dyn std::any::Any>, usize), DecodeError> {
    let mut pos = 1; 

    while pos < data.len() && data[pos] != b'\r' {
        pos += 1;
    }

    if pos >= data.len() {
        return Err(DecodeError);
    }

    let s = String::from_utf8_lossy(&data[1..pos]).to_string();

    return Ok((Box::new(s), pos+2));
}

pub fn decode_bulk_string(data: &[u8]) -> Result<(Box<dyn std::any::Any>, usize), DecodeError> {
    let mut pos = 1;

    let (len, delta) = read_length(&data[pos..])?;

    pos += delta + 2; 
    let end = pos + len;

    if end + 2 > data.len() {
        return Err(DecodeError); 
    }

    let s = String::from_utf8_lossy(&data[pos..end]).to_string();

    return Ok((Box::new(s), end + 2));
}

pub fn decode_integer(data: &[u8]) -> Result<(Box<dyn std::any::Any>, usize), DecodeError> {
    // Ok((Box::new(42u32), 0))
    let mut pos = 1;
    let mut value: u64 = 0;

    while pos < data.len() && data[pos] != b'\r' {
        value  = value * 10 + (&data[pos] - b'0') as u64;
        pos += 1;
    }

    if pos >= data.len() {
        return Err(DecodeError);
    }

    let i: u64 = match String::from_utf8_lossy(&data[1..pos]).parse() {
        Ok(n) => n,
        Err(_) => return Err(DecodeError),
    };

    return Ok((Box::new(i), pos+2));

}

pub fn decode_arrays(data: &[u8]) -> Result<(Box<dyn std::any::Any>, usize), DecodeError> {
    let mut pos = 1;

    let (count, delta) = read_length(&data[pos..])?;

    pos += delta + 2; 

    let mut elements: Vec<Box<dyn std::any::Any>> = Vec::new();
    // elements.concat()

    for _ in 0..count {
        let (value, consumed) = decode_one(&data[pos..])?;
        elements.push(value);
        pos += consumed;
    }

    Ok((Box::new(elements), pos))
}

pub fn decode_errors(data: &[u8]) -> Result<(Box<dyn std::any::Any>, usize), DecodeError> {
    return decode_simple_string(data);
}