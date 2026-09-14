use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::time::Instant;

use crate::cmd::{RedisCmd, RedisValue, Store};
use crate::commands::{Outcome, unix_millis};
use crate::helpers::utils::DecodeError;
use crate::resp::decode_array_string;
use crate::stats::Stat;
use crate::sync_tcp::respond;

const AOF_PATH: &str = "appendonly.aof";

pub struct Aof {
    file: File,
}

impl Aof {
    pub fn new() -> io::Result<Self> {
        Ok(Self {
            file: OpenOptions::new()
                .create(true)
                .append(true)
                .open(AOF_PATH)?,
        })
    }

    pub fn append(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.file.write_all(bytes)
    }

    pub fn flush(&mut self) -> io::Result<()> {
        self.file.sync_all()
    }

    pub fn record_deleted(&mut self, keys: &[Vec<u8>]) -> io::Result<()> {
        for key in keys {
            self.append(&encode_command("DEL", std::slice::from_ref(key)))?;
        }
        Ok(())
    }

    pub fn record_mutation(&mut self, cmd: &RedisCmd, store: &Store) -> io::Result<()> {
        if cmd.cmd.eq_ignore_ascii_case("DEL") {
            return self.record_deleted(&cmd.args);
        }
        let key = &cmd.args[0];
        let Some(entry) = store.get(key) else {
            return self.record_deleted(std::slice::from_ref(key));
        };
        let now = Instant::now();
        if entry.expires_at.is_some_and(|deadline| deadline <= now) {
            return self.record_deleted(std::slice::from_ref(key));
        }
        let absolute = entry
            .expires_at
            .map(|deadline| -> io::Result<u64> {
                let remaining = u64::try_from(deadline.duration_since(now).as_millis())
                    .map_err(io::Error::other)?;
                unix_millis()?
                    .checked_add(remaining)
                    .ok_or_else(|| io::Error::other("expiry timestamp overflow"))
            })
            .transpose()?;
        let value = match &entry.value {
            RedisValue::Raw(bytes) => bytes.clone(),
            RedisValue::EmbStr(bytes) => bytes.to_vec(),
            RedisValue::Int(number) => number.to_string().into_bytes(),
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "unsupported AOF value",
                ));
            }
        };
        // Persist resulting values so expiry cannot change the meaning of a replayed INCR.
        let mut args = vec![key.clone(), value];
        if let Some(absolute) = absolute {
            args.extend([b"PXAT".to_vec(), absolute.to_string().into_bytes()]);
        }
        self.append(&encode_command("SET", &args))
    }

    pub fn load(store: &mut Store) -> io::Result<()> {
        let mut file = match OpenOptions::new().read(true).write(true).open(AOF_PATH) {
            Ok(file) => file,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(e),
        };
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;
        let mut offset = 0;
        let mut stats = Stat::new();
        while offset < buffer.len() {
            match decode_array_string(&buffer[offset..]) {
                Ok((tokens, consumed)) => {
                    let cmd = RedisCmd::from_tokens(tokens).map_err(|_| {
                        io::Error::new(io::ErrorKind::InvalidData, "invalid AOF command")
                    })?;
                    if !matches!(cmd.cmd.as_str(), "SET" | "INCR" | "DEL" | "EXPIRE")
                        || respond(&cmd, store, &mut stats, &mut io::sink())? == Outcome::Rejected
                    {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("invalid AOF operation at byte {offset}"),
                        ));
                    }
                    offset += consumed;
                }
                Err(DecodeError::Incomplete) => {
                    // Remove only an incomplete final frame before accepting new writes.
                    file.set_len(offset as u64)?;
                    file.sync_all()?;
                    break;
                }
                Err(DecodeError::Invalid) => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("corrupt AOF at byte {offset}"),
                    ));
                }
            }
        }
        Ok(())
    }
}

pub fn encode_command(name: &str, args: &[Vec<u8>]) -> Vec<u8> {
    let mut out = format!("*{}\r\n", args.len() + 1).into_bytes();
    for arg in std::iter::once(name.as_bytes()).chain(args.iter().map(Vec::as_slice)) {
        write!(out, "${}\r\n", arg.len()).unwrap();
        out.extend_from_slice(arg);
        out.extend_from_slice(b"\r\n");
    }
    out
}

#[cfg(test)]
pub fn aof_entry(cmd: &RedisCmd) -> Option<Vec<u8>> {
    match cmd.cmd.to_ascii_uppercase().as_str() {
        "SET" | "DEL" | "INCR" => Some(encode_command(&cmd.cmd, &cmd.args)),
        _ => None,
    }
}
