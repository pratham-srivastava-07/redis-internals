use std::any::{Any};
use std::collections::HashMap;
use std::io::{Error, Write};
use std::time::{Duration, Instant};

use crate::cmd::{Entry, RedisValue};


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
pub fn set_command<S: Write>(args: Vec<String>, store: &mut HashMap<String, Entry>, stream: &mut S) -> std::io::Result<()>  {
    // in memory data 
    // let mut data: HashMap<String, RedisValue> = HashMap::new();

    if args.is_empty() {
        return Err(Error::new(std::io::ErrorKind::InvalidInput, "is empty"));
    }

    if args.len() < 2 {
        return stream.write_all(b"-ERR wrong number of arguments for 'set' command\r\n");
    }
    // OLD IMPL 
    
    // if let Some(arg) = args.get(2) {
    //     println!("{:?}", arg);
    //     let extract_time: u64 = match *&args[3].parse::<u64>() {
    //         Ok(n) =>  n,
    //         Err(_) => return Err(Error::new(std::io::ErrorKind::InvalidInput, "ERR value is not an integer or out of range"))
    //     };

    //     println!("{:?}", extract_time);
    // }

    let key = &args[0];
    let value = &args[1];

    let expires_at = match args.get(2) {
        Some(opt) => {
            let num: u64 = match args.get(3).and_then(|s| s.parse::<u64>().ok()) {
                Some(n) => n,
                None => return stream.write_all(b"-ERR value is not an integer or out of range\r\n")
            };
            match opt.to_uppercase().as_str() {
                "EX" => Some(Instant::now() + Duration::from_secs(num)),
                "PX" => Some(Instant::now() + Duration::from_millis(num)),
                _ => return stream.write_all(b"-ERR syntax error\r\n")
            }
        }

        None => None,
    };

    store.insert(key.to_string(), Entry {
        value: RedisValue::String(value.to_string()),
        expires_at
    });

    stream.write_all(b"+OK\r\n")
}

pub fn get_command<S: Write>(args: Vec<String>, store: &mut HashMap<String, Entry>, stream: &mut S) -> std::io::Result<()> {
    if args.is_empty() {
        return stream.write_all(b"-ERR wrong number of arguments for 'get' command\r\n");
    }

    let key = &args[0];

    let expiration = match store.get(key) {
        Some(entry) => matches!(entry.expires_at, Some(exp) if Instant::now() >= exp),
        None => false
    };

    if expiration {
        store.remove(key);
    }

    match store.get(key) {
        Some(entry) => match &entry.value {
            RedisValue::String(val) => {
                let reply = format!("${}\r\n{}\r\n", val.len(), val);
                stream.write_all(reply.as_bytes())
            }
            _ => stream.write_all(b"-WRONGTYPE Operation against a key holding the wrong kind of value\r\n")
        }

        None => stream.write_all(b"$-1\r\n")
    }
}

pub fn set_ttl<S: Write>(args: Vec<String>, store: &mut HashMap<String, Entry>, stream: &mut S) -> std::io::Result<()> {
    if args.is_empty() {
        return stream.write_all(b"-ERR wrong number of arguments for 'ttl' command\r\n");
    }

    // println!("args: {:?}", args);

    let key = &args[0];

    let expired = match store.get(key) {
        Some(n) => matches!(n.expires_at, Some(m) if Instant::now() >= m),
        None => false
    };

    if expired {
        store.remove(key);
    }

    let reply: i64 = match store.get(key) {
        None => -2,
        Some(entry) => match entry.expires_at {
            None => -1,
            Some(exp) => {
                exp.saturating_duration_since(Instant::now()).as_secs() as i64 
            }
        }
    };

    // println!("{:?}", type_name_of_val(&expired));
    stream.write_all(format!(":{}\r\n", reply).as_bytes())


}

pub fn delete_keys<S: Write>(key_args: Vec<String>, store: &mut HashMap<String, Entry>, stream: &mut S) -> std::io::Result<()> {
    if key_args.is_empty() {
        return stream.write_all(b"-ERR wrong number of arguments for 'del' command\r\n");
    }
    let mut count = 0;
    // if key exists, delete the key and return the number of keys deleted, only return number of keys deleted regaurdless of how many keys are provided 
    for key in key_args {
        if store.contains_key(&key) {
            store.remove(&key.to_string());
            count += 1;
        } else {
            //
        }
    }
    // if key does not exist, return 0

    stream.write_all(format!(":{}\r\n", count).as_bytes())
}

pub fn expire_command<S: Write>(args: Vec<String>, store: &mut HashMap<String, Entry>, stream: &mut S) -> std::io::Result<()> {
    if args.is_empty() {
        return stream.write_all(b"-ERR wrong number of arguments for 'del' command\r\n");
    }

    if args.len() < 2 {
        return stream.write_all(b"ERR wrong number of arguments for 'expire' command\r\n");
    }

    let key = &args[0];
    // let ttl_in_secs = &args[1];

    let secs = match args[1].parse::<u64>() {
        Ok(n) => n,
        Err(_) => return stream.write_all(b"-ERR value is not an integer or out of range\r\n"),
    };

    // check if key exists in store
    if !store.contains_key(&key.to_string()) {
        return stream.write_all(b"ERR key does not exist\r\n");
    }
    // if the command is able to set an expiration, it returns 1 (true)
    match store.get_mut(key) {
        Some(entry) => {
            entry.expires_at = Some(Instant::now() + Duration::from_secs(secs));
            stream.write_all(b":1\r\n")
        }
        None => stream.write_all(b":0\r\n"),
    }
}

#[cfg(test)]
mod tests;

