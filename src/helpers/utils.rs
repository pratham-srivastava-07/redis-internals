use crate::helpers::data_parse::{decode_arrays, decode_bulk_string, decode_errors, decode_integer, decode_simple_string};

#[derive(Debug)]
pub struct DecodeError;

pub fn decode_one(data: &[u8]) -> Result<(Box<dyn std::any::Any>, usize), DecodeError> {
    if data.is_empty() {
        return Err(DecodeError);
    }
    let first_special_char = data[0];

    match first_special_char {
       b'+' => decode_simple_string(data),
       b':' => decode_integer(data),
       b'$' => decode_bulk_string(data),
       b'*' => decode_arrays(data),
       b'-' => decode_errors(data),
       _ => Err(DecodeError)
    }
}

pub fn read_length(data: &[u8]) -> Result<(usize, usize), DecodeError> {
    let mut pos = 0;

    let mut length: usize = 0;

    while pos < data.len() {
        let byte = data[pos];

        if !(byte >= b'0' && byte <= b'9') {
            return Ok((length, pos));
        }
        length = length * 10 + (byte - b'0') as usize;
        pos += 1;
    }

    Err(DecodeError)
    
}