#[derive(Debug)]
struct DecodeError;

pub fn decode_one(data: &[u8]) -> Result<(Box<dyn std::any::Any>, usize), DecodeError> {
    if data.is_empty() {
        return Err(DecodeError);
    }
    Ok((Box::new(42u32), 5))
}