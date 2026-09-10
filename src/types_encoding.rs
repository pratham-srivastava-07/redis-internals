pub const OBJ_TYPE_STRING: u8 = 0 << 4;
pub const OBJ_ENCODING_RAW: u8 = 0;
pub const OBJ_ENCODING_INT: u8 = 1;
pub const OBJ_ENCODING_EMBSTR: u8 = 8;

pub fn get_type(te: u8) -> u8 {
    te & 0xf0
}

pub fn get_encoding(te: u8) -> u8 {
    te & 0x0f
}

// Canonical decimal only: reject '+', whitespace, leading zeros and '-0'.
pub fn parse_integer(bytes: &[u8]) -> Option<i64> {
    let text = std::str::from_utf8(bytes).ok()?;
    let number = text.parse::<i64>().ok()?;
    (number.to_string() == text).then_some(number)
}
