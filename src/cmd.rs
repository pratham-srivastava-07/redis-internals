use std::collections::{HashMap, HashSet};

pub struct RedisCmd {
    pub cmd: String,
    pub args: Vec<String>
}

pub enum RedisValue {
    String(String),
    List(Vec<String>),
    Set(HashSet<String>),
    Hash(HashMap<String, String>),
}