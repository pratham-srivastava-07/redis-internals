use crate::types_encoding::*;
use std::collections::{HashMap, HashSet};
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct Obj {
    pub value: ObjValue,
    pub expires_at: Option<Instant>,
    pub last_accessed_at: u32,
}

#[derive(Debug, Clone)]
pub enum ObjValue {
    Raw(Vec<u8>),
    Int(i64),
    // Models the encoding, not Redis's single object + string allocation.
    EmbStr(Box<[u8]>),
    _List(Vec<String>),
    _Set(HashSet<String>),
    _Hash(HashMap<String, String>),
}

impl ObjValue {
    pub fn from_bytes(value: Vec<u8>) -> Self {
        if let Some(number) = parse_integer(&value) {
            Self::Int(number)
        } else if value.len() <= 44 {
            Self::EmbStr(value.into_boxed_slice())
        } else {
            Self::Raw(value)
        }
    }
}

impl Obj {
    pub fn type_encoding(&self) -> u8 {
        // Derive the tag so it cannot disagree with the stored value.
        match &self.value {
            ObjValue::EmbStr(_) => OBJ_TYPE_STRING | OBJ_ENCODING_EMBSTR,
            ObjValue::Int(_) => OBJ_TYPE_STRING | OBJ_ENCODING_INT,
            ObjValue::Raw(_) => OBJ_TYPE_STRING | OBJ_ENCODING_RAW,
            ObjValue::_List(_) => 1 << 4,
            ObjValue::_Set(_) => 2 << 4,
            ObjValue::_Hash(_) => 4 << 4,
        }
    }
}
