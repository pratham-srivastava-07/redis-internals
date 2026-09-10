// this file corresponds to how redis uses persistence for the entries it has in-memory.
// there are basically 2 ways to do it...RDB (snapshot) and AOF (append only file)

use std::{collections::HashMap, fs::{File, OpenOptions}, io::{self, Read, Write}};

use crate::{cmd::{Entry, RedisCmd}, helpers::utils::DecodeError, resp::decode_array_string, stats::Stat, sync_tcp::respond};

const AOF_PATH: &str = "appendonly.aof";

pub struct Aof {
    file: File
}

impl Aof {
    // creating the append only file 
    pub fn new() -> io::Result<Aof> {
        let file = OpenOptions::new().create(true).append(true).read(true).open(AOF_PATH)?;
        Ok(Aof { file })
    }

    // writing command's resp bytes out
    pub fn append(&mut self, bytes: &[u8]) {
        if let Err(e) = self.file.write_all(bytes) {
            eprintln!("aof append failed: {}", e);
        }
    }

    // forcing everything we have written to actually hit the disc
    pub fn flush(&mut self) {
        if let Err(e) = self.file.sync_all() {
            eprintln!("Flushing into disc failed: {}", e);
        }
    }

    // startup: read the whole file and replay commands 

    pub fn load(store: &mut HashMap<String, Entry>) -> io::Result<()> {
        let mut file = match OpenOptions::new().read(true).open(AOF_PATH) {
            Ok(f) => f,
            Err(ref e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(e)
        };

        let mut buffer: Vec<u8> = Vec::new();
        let mut offset = 0;
        let mut stats = Stat::new();


        file.read_to_end(&mut buffer)?;

        while offset < buffer.len() {
            match decode_array_string(&mut buffer[offset..]) {
                Ok((tokens, consumed)) => {
                    offset += consumed;
                    if tokens.is_empty() {
                        break;
                    }
                    let cmd = RedisCmd {
                        cmd: tokens[0].clone(),
                        args: tokens[1..].to_vec()
                    };
                    respond(cmd, store, &mut stats,&mut io::sink());
                }
                Err(DecodeError::Incomplete) => break,
                Err(DecodeError::Invalid) => break
            }
        }

        Ok(())

    }
}

fn encode_command(name: &str, args: &[String]) -> Vec<u8> {
    let mut out = Vec::new();
    let n = 1 + args.len();
    out.extend_from_slice(format!("*{}\r\n", n).as_bytes());
    out.extend_from_slice(format!("${}\r\n{}\r\n", name.len(), name).as_bytes());
    for a in args {
        out.extend_from_slice(format!("${}\r\n{}\r\n", a.len(), a).as_bytes());
    }
    out
}

 pub fn aof_entry(cmd: &RedisCmd) -> Option<Vec<u8>> {
    match cmd.cmd.to_uppercase().as_str() {
        "SET" if cmd.args.len() == 2 => Some(encode_command(&cmd.cmd, &cmd.args)),
        "DEL" if !cmd.args.is_empty() => Some(encode_command(&cmd.cmd, &cmd.args)),
        "INCR" if cmd.args.len() == 1 => Some(encode_command(&cmd.cmd, &cmd.args)),
        _ => None,
    }
}