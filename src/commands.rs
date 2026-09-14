use crate::cmd::{Entry, RedisValue, Store};
use crate::eviction::get_current_clock;
use crate::stats::Stat;
use crate::types_encoding::*;
use std::io::{self, Write};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[derive(Debug, PartialEq)]
pub enum Outcome {
    ReadOnly,
    Modified,
    Rejected,
}

fn reply<S: Write>(stream: &mut S, bytes: &[u8], outcome: Outcome) -> io::Result<Outcome> {
    stream.write_all(bytes)?;
    Ok(outcome)
}

fn error<S: Write>(stream: &mut S, message: &str) -> io::Result<Outcome> {
    write!(stream, "-{message}\r\n")?;
    Ok(Outcome::Rejected)
}

fn arity<S: Write>(stream: &mut S, command: &str) -> io::Result<Outcome> {
    error(
        stream,
        &format!("ERR wrong number of arguments for '{command}' command"),
    )
}

pub fn unix_millis() -> io::Result<u64> {
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(io::Error::other)?;
    u64::try_from(elapsed.as_millis()).map_err(io::Error::other)
}

fn write_bulk<S: Write>(value: &[u8], stream: &mut S) -> io::Result<()> {
    write!(stream, "${}\r\n", value.len())?;
    stream.write_all(value)?;
    stream.write_all(b"\r\n")
}

fn remove_expired(key: &[u8], store: &mut Store) {
    if store
        .get(key)
        .is_some_and(|obj| obj.expires_at.is_some_and(|end| Instant::now() >= end))
    {
        store.remove(key);
    }
}

pub fn eval_ping<S: Write>(args: &[Vec<u8>], stream: &mut S) -> io::Result<Outcome> {
    if args.len() > 1 {
        return arity(stream, "ping");
    }
    if let Some(message) = args.first() {
        write_bulk(message, stream)?;
        Ok(Outcome::ReadOnly)
    } else {
        reply(stream, b"+PONG\r\n", Outcome::ReadOnly)
    }
}

pub fn set_command<S: Write>(
    args: &[Vec<u8>],
    store: &mut Store,
    stream: &mut S,
) -> io::Result<Outcome> {
    if args.len() < 2 {
        return arity(stream, "set");
    }
    if args.len() != 2 && args.len() != 4 {
        return error(stream, "ERR syntax error");
    }
    let mut expires_at = None;
    if args.len() == 4 {
        let option = &args[2];
        if !option.eq_ignore_ascii_case(b"EX")
            && !option.eq_ignore_ascii_case(b"PX")
            && !option.eq_ignore_ascii_case(b"PXAT")
        {
            return error(stream, "ERR syntax error");
        }
        let Some(value) = parse_integer(&args[3]) else {
            return error(stream, "ERR value is not an integer or out of range");
        };
        if value <= 0 {
            return error(stream, "ERR invalid expire time in 'set' command");
        }
        let now_ms = unix_millis()?;
        let delta = if option.eq_ignore_ascii_case(b"EX") {
            (value as u64).checked_mul(1000)
        } else if option.eq_ignore_ascii_case(b"PXAT") {
            Some((value as u64).saturating_sub(now_ms))
        } else {
            Some(value as u64)
        };
        let deadline = delta
            .filter(|ms| {
                now_ms
                    .checked_add(*ms)
                    .is_some_and(|end| end <= i64::MAX as u64)
            })
            .and_then(|ms| Instant::now().checked_add(Duration::from_millis(ms)));
        let Some(deadline) = deadline else {
            return error(stream, "ERR invalid expire time in 'set' command");
        };
        expires_at = Some(deadline);
    }
    store.insert(
        args[0].clone(),
        Entry {
            value: RedisValue::from_bytes(args[1].clone()),
            expires_at,
            last_accessed_at: get_current_clock(),
        },
    );
    reply(stream, b"+OK\r\n", Outcome::Modified)
}

pub fn get_command<S: Write>(
    args: &[Vec<u8>],
    store: &mut Store,
    stream: &mut S,
) -> io::Result<Outcome> {
    if args.len() != 1 {
        return arity(stream, "get");
    }
    remove_expired(&args[0], store);
    let Some(entry) = store.get_mut(&args[0]) else {
        return reply(stream, b"$-1\r\n", Outcome::ReadOnly);
    };
    entry.last_accessed_at = get_current_clock();
    match &entry.value {
        RedisValue::Raw(bytes) => write_bulk(bytes, stream)?,
        RedisValue::EmbStr(bytes) => write_bulk(bytes, stream)?,
        RedisValue::Int(number) => write_bulk(number.to_string().as_bytes(), stream)?,
        _ => {
            return error(
                stream,
                "WRONGTYPE Operation against a key holding the wrong kind of value",
            );
        }
    }
    Ok(Outcome::ReadOnly)
}

pub fn set_ttl<S: Write>(
    args: &[Vec<u8>],
    store: &mut Store,
    stream: &mut S,
) -> io::Result<Outcome> {
    if args.len() != 1 {
        return arity(stream, "ttl");
    }
    remove_expired(&args[0], store);
    let ttl = match store.get(&args[0]) {
        None => -2,
        Some(entry) => match entry.expires_at {
            None => -1,
            Some(end) => {
                ((end.saturating_duration_since(Instant::now()).as_millis() + 500) / 1000) as i64
            }
        },
    };
    write!(stream, ":{ttl}\r\n")?;
    Ok(Outcome::ReadOnly)
}

pub fn delete_keys<S: Write>(
    args: &[Vec<u8>],
    store: &mut Store,
    stream: &mut S,
) -> io::Result<Outcome> {
    if args.is_empty() {
        return arity(stream, "del");
    }
    let mut deleted = 0;
    for key in args {
        remove_expired(key, store);
        if store.remove(key).is_some() {
            deleted += 1;
        }
    }
    write!(stream, ":{deleted}\r\n")?;
    Ok(Outcome::Modified)
}

pub fn expire_command<S: Write>(
    args: &[Vec<u8>],
    store: &mut Store,
    stream: &mut S,
) -> io::Result<Outcome> {
    if args.len() != 2 {
        return arity(stream, "expire");
    }
    let Some(seconds) = parse_integer(&args[1]) else {
        return error(stream, "ERR value is not an integer or out of range");
    };
    let deadline = if seconds > 0 {
        let now_ms = unix_millis()?;
        let checked = (seconds as u64)
            .checked_mul(1000)
            .filter(|ms| {
                now_ms
                    .checked_add(*ms)
                    .is_some_and(|end| end <= i64::MAX as u64)
            })
            .and_then(|ms| Instant::now().checked_add(Duration::from_millis(ms)));
        let Some(deadline) = checked else {
            return error(stream, "ERR invalid expire time in 'expire' command");
        };
        Some(deadline)
    } else {
        None
    };
    remove_expired(&args[0], store);
    let Some(entry) = store.get_mut(&args[0]) else {
        return reply(stream, b":0\r\n", Outcome::ReadOnly);
    };
    if let Some(deadline) = deadline {
        entry.expires_at = Some(deadline);
    } else {
        store.remove(&args[0]);
    }
    reply(stream, b":1\r\n", Outcome::Modified)
}

pub fn incr_command<S: Write>(
    args: &[Vec<u8>],
    store: &mut Store,
    stream: &mut S,
) -> io::Result<Outcome> {
    if args.len() != 1 {
        return arity(stream, "incr");
    }
    let key = &args[0];
    remove_expired(key, store);
    let number = match store.get(key).map(|entry| &entry.value) {
        None => Some(0),
        Some(RedisValue::Int(number)) => Some(*number),
        Some(RedisValue::Raw(bytes)) => parse_integer(bytes),
        Some(RedisValue::EmbStr(bytes)) => parse_integer(bytes),
        _ => {
            return error(
                stream,
                "WRONGTYPE Operation against a key holding the wrong kind of value",
            );
        }
    };
    let Some(number) = number else {
        return error(stream, "ERR value is not an integer or out of range");
    };
    let Some(next) = number.checked_add(1) else {
        return error(stream, "ERR increment or decrement would overflow");
    };
    let entry = store.entry(key.clone()).or_insert(Entry {
        value: RedisValue::Int(next),
        expires_at: None,
        last_accessed_at: get_current_clock(),
    });
    entry.value = RedisValue::Int(next);
    entry.last_accessed_at = get_current_clock();
    write!(stream, ":{next}\r\n")?;
    Ok(Outcome::Modified)
}

pub fn object_command<S: Write>(
    args: &[Vec<u8>],
    store: &mut Store,
    stream: &mut S,
) -> io::Result<Outcome> {
    if args.len() != 2 || !args[0].eq_ignore_ascii_case(b"ENCODING") {
        return error(stream, "ERR syntax: OBJECT ENCODING key");
    }
    remove_expired(&args[1], store);
    let Some(entry) = store.get(&args[1]) else {
        return reply(stream, b"$-1\r\n", Outcome::ReadOnly);
    };
    if get_type(entry.type_encoding()) != OBJ_TYPE_STRING {
        return error(
            stream,
            "ERR encoding inspection is only implemented for strings",
        );
    }
    let name: &[u8] = match get_encoding(entry.type_encoding()) {
        OBJ_ENCODING_RAW => b"raw",
        OBJ_ENCODING_INT => b"int",
        OBJ_ENCODING_EMBSTR => b"embstr",
        _ => unreachable!(),
    };
    write_bulk(name, stream)?;
    Ok(Outcome::ReadOnly)
}

pub fn eval_info<S: Write>(
    args: &[Vec<u8>],
    store: &Store,
    stats: &mut Stat,
    stream: &mut S,
) -> io::Result<Outcome> {
    if args.len() > 1
        || args
            .first()
            .is_some_and(|arg| !arg.eq_ignore_ascii_case(b"KEYSPACE"))
    {
        return error(stream, "ERR supported INFO section: keyspace");
    }
    let now = Instant::now();
    let (mut keys, mut expires, mut ttl_ms) = (0, 0, 0u128);
    for entry in store.values() {
        match entry.expires_at {
            Some(end) if end <= now => continue,
            Some(end) => {
                expires += 1;
                ttl_ms += end.duration_since(now).as_millis();
            }
            None => {}
        }
        keys += 1;
    }
    stats.update_stat_db(0, "keys".into(), keys);
    stats.update_stat_db(0, "expires".into(), expires);
    let mut info = String::from("# Keyspace\r\n");
    if keys > 0 {
        let average = if expires == 0 {
            0
        } else {
            ttl_ms / expires as u128
        };
        info.push_str(&format!(
            "db0:keys={keys},expires={expires},avg_ttl={average}\r\n"
        ));
    }
    write_bulk(info.as_bytes(), stream)?;
    Ok(Outcome::ReadOnly)
}

#[cfg(test)]
mod tests;
