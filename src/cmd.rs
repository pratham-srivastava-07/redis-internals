use std::{collections::{HashMap, HashSet}, time::Instant};

pub struct RedisCmd {
    pub cmd: String,
    pub args: Vec<String>
}

pub enum RedisValue {
    String(String),
    _List(Vec<String>),
    _Set(HashSet<String>),
    _Hash(HashMap<String, String>),
}

pub struct Entry {
    pub value: RedisValue,
    pub expires_at: Option<Instant>
}