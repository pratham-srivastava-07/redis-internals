use std::any::Any;
use std::collections::HashMap;
use std::io::{Error, Write};

use crate::cmd::RedisValue;


pub fn eval_ping<S: Write>(args: Vec<String>, stream: &mut S) -> std::io::Result<()> {
    if args.len() >= 2 {
        return stream.write_all(b"-ERR wrong number of arguments for 'ping' command\r\n");
    }

    let res = if args.is_empty() {
        encode(&"PONG", true)
    } else {
        encode(&args[0], false)
    };

    stream.write_all(&res)
}

pub fn encode(value: &dyn Any, is_simple: bool) -> Vec<u8> {
    if let Some(s) = value.downcast_ref::<String>() {
        return encode_string(s, is_simple);
    }
    if let Some(s) = value.downcast_ref::<&str>() {
        return encode_string(s, is_simple);
    }
    if let Some(i) = value.downcast_ref::<i64>() {
        return format!(":{}\r\n", i).into_bytes();
    }
    Vec::new()
}

fn encode_string(s: &str, is_simple: bool) -> Vec<u8> {
    if is_simple {
        format!("+{}\r\n", s).into_bytes()
    } else {
        format!("${}\r\n{}\r\n", s.len(), s).into_bytes()
    }
}


// SET, GET  && TTL
pub fn set_command<S: Write>(args: Vec<String>, store: &mut HashMap<String, RedisValue>, stream: &mut S) -> std::io::Result<()>  {
    // in memory data 
    // let mut data: HashMap<String, RedisValue> = HashMap::new();

    if args.is_empty() {
        return Err(Error::new(std::io::ErrorKind::InvalidInput, "is empty"));
    }

    let key = &args[0];
    let value = &args[1];

    store.insert(key.clone(), RedisValue::String(value.clone()));

    stream.write_all(b"+OK\r\n")
}

pub fn get_command<S: Write>(args: Vec<String>, store: &mut HashMap<String, RedisValue>, stream: &mut S) -> std::io::Result<()> {
    if args.is_empty() {
        return stream.write_all(b"-ERR wrong number of arguments for 'get' command\r\n");
    }

    let key = &args[0];

    match store.get(key) {
        Some(RedisValue::String(val)) => {
            let reply = format!("${}\r\n{}\r\n", val.len(), val);
            stream.write_all(reply.as_bytes())
        }
        Some(_) => {
            stream.write_all(b"-WRONGTYPE Operation against a key holding the wrong kind of value\r\n")
        }
        None => {
            stream.write_all(b"-1\r\n")
        }
    }
}

