pub use crate::object::{Obj as Entry, ObjValue as RedisValue};
use std::collections::HashMap;

pub type Store = HashMap<Vec<u8>, Entry>;

#[derive(Debug)]
pub struct RedisCmd {
    pub cmd: String,
    pub args: Vec<Vec<u8>>,
}

impl RedisCmd {
    pub fn from_tokens(
        mut tokens: Vec<Vec<u8>>,
    ) -> Result<Self, crate::helpers::utils::DecodeError> {
        if tokens.is_empty() || !tokens[0].is_ascii() {
            return Err(crate::helpers::utils::DecodeError::Invalid);
        }
        let name = String::from_utf8(tokens.remove(0))
            .map_err(|_| crate::helpers::utils::DecodeError::Invalid)?;
        Ok(Self {
            cmd: name.to_ascii_uppercase(),
            args: tokens,
        })
    }
}
