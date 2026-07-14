use std::{io::{Read, Write, ErrorKind}};


use crate::{cmd::RedisCmd, commands::eval_ping, resp::decode_array_string};

#[derive(Debug)]
pub enum ReadError {
    WouldBlock,
    Disconnected,
    Decode
}

pub fn read_command<S: Read>(con: &mut S) -> Result<RedisCmd, ReadError> {
    let mut buffer = [0u8; 512];

    let n = match con.read(&mut buffer) {
        Ok(0) => return Err(ReadError::Disconnected),
        Ok(n) => n,
        Err(ref e) if e.kind() == ErrorKind::WouldBlock => return Err(ReadError::WouldBlock),
        Err(_) => return Err(ReadError::Disconnected)
    };

    let tokens = decode_array_string(&buffer[..n]).map_err(|_| ReadError::Decode)?;

    if tokens.is_empty() {
        return Err(ReadError::Decode);
    }

    Ok(RedisCmd {
        cmd: tokens[0].clone(),
        args: tokens[1..].to_vec()
    })

}


pub fn respond<S: Write>(cmd: RedisCmd, stream: &mut S) {
    let val = eval_and_respond(cmd, stream);

    if val.is_err() {
        respond_error("Error", stream)
    }
}

fn respond_error<S: Write>(err: &str, stream: &mut S) {
    let _ = stream.write_all(format!("-{}\r\n", err).as_bytes());
}

fn eval_and_respond<S: Write>(cmd: RedisCmd,stream: &mut S) -> std::io::Result<()> {
    match cmd.cmd.to_uppercase().as_str() {
        "PING" => eval_ping(cmd.args, stream),
        _ => stream.write_all(b"-ERR unknown command\r\n"),
    }
}