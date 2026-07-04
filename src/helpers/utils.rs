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

// pub fn decode_second(data: &[u8]) -> Result<usize, DecodeError> {
//     if data.is_empty() {
//         return Err(DecodeError);
//     }

//     let end = match data.iter().position(|&b| b == b'\r') {
//         Some(pos) => pos,
//         None => return Err(DecodeError)
//     };


// }