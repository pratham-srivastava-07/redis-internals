// Pipelining: one socket read can carry several commands glued together,
// or a single command split across reads. We separate the two jobs:
//   1. fill_box     -> drain the socket into a per-client inbox buffer
//   2. parse_commands -> carve out every COMPLETE command the inbox holds,
//                        leaving any partial tail behind for next time.

use std::io::{ErrorKind, Read};

use crate::cmd::{RedisCmd, RedisCmds};
use crate::helpers::utils::DecodeError;
use crate::resp::decode_array_string;
use crate::sync_tcp::ReadError;

pub fn read_commands<S: Read>(conn: &mut S) -> Result<RedisCmds, ReadError> {
    let mut buffer = [0u8; 1024];

    let n = match conn.read(&mut buffer) {
        Ok(0) => return Err(ReadError::Disconnected),
        Ok(n) => n,
        Err(ref e) if e.kind() == ErrorKind::WouldBlock => return Err(ReadError::WouldBlock),
        Err(_) => return Err(ReadError::Disconnected)
    };

    let mut cmds: Vec<RedisCmd> = Vec::new();

    let mut offset = 0;

    while offset < n {
        let (tokens, consumed) = decode_array_string(&buffer[offset..n]).map_err(|_| ReadError::Decode)?;

        if consumed == 0 {
            return Err(ReadError::Decode);
        }

        offset += consumed;

        if tokens.is_empty() {
            return Err(ReadError::Decode);
        }

        cmds.push(RedisCmd { cmd: tokens[0].clone(), args: tokens[1..].to_vec() })
    }
    if cmds.is_empty() {
        return Err(ReadError::Decode);
    }

    Ok(RedisCmds { cmds })
}

pub enum Fill {
    Ok,           // read some bytes; socket now drained to WouldBlock
    Disconnected, // client closed
}

// JOB 1: drain the socket completely into the client's inbox.
// (Edge-triggered: we MUST read until WouldBlock or mio won't wake us again.)
pub fn fill_box<S: Read>(conn: &mut S, inbox: &mut Vec<u8>) -> Fill {
    let mut temp = [0u8; 4096];
    loop {
        match conn.read(&mut temp) {
            Ok(0) => return Fill::Disconnected,
            Ok(n) => inbox.extend_from_slice(&temp[..n]),
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => return Fill::Ok,
            Err(_) => return Fill::Disconnected,
        }
    }
}

// JOB 2: pull out every COMPLETE command the inbox currently holds.
// A trailing partial command is LEFT in the inbox for next time.
// Err(()) means a genuinely malformed frame -> caller should close.
pub fn parse_commands(inbox: &mut Vec<u8>) -> Result<Vec<RedisCmd>, ()> {
    let mut cmds = Vec::new();
    let mut offset = 0;

    while offset < inbox.len() {
        match decode_array_string(&inbox[offset..]) {
            Ok((tokens, consumed)) => {
                offset += consumed; // advance past this command
                if tokens.is_empty() {
                    return Err(()); // "*0" as a command is nonsense
                }
                cmds.push(RedisCmd {
                    cmd: tokens[0].clone(),
                    args: tokens[1..].to_vec(),
                });
            }
            Err(DecodeError::Incomplete) => break, // not all here yet -> stop, keep tail
            Err(DecodeError::Invalid) => return Err(()), // garbage -> close
        }
    }

    // Discard only the bytes we fully consumed; keep any partial tail.
    inbox.drain(0..offset);
    Ok(cmds)
}
