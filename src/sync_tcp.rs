use std::{io::{Read, Write}, net::TcpStream};

use crate::{cmd::RedisCmd, commands::eval_ping, helpers::{utils::DecodeError}, resp::decode_array_string};

pub fn read_command(mut con: TcpStream) -> Result<RedisCmd, DecodeError> {
    let mut buffer = [0u8; 512];

    let n = match con.read(&mut buffer) {
        Ok(n) => n,
        Err(_) => return Err(DecodeError)
    };

    let tokens = decode_array_string(&buffer[..n])?;

    if tokens.is_empty() {
        return Err(DecodeError);
    }

    Ok(RedisCmd {
        cmd: tokens[0].clone(),
        args: tokens[1..].to_vec()
    })

}


pub fn respond(cmd: RedisCmd, stream: &TcpStream) {
    let val = eval_and_respond(cmd, stream);

    if val.is_err() {
        respond_error("Error", stream)
    }
}

fn respond_error(err: &str, mut stream: &TcpStream) {
    let _ = stream.write_all(format!("-{}\r\n", err).as_bytes());
}

fn eval_and_respond(cmd: RedisCmd, mut stream: &TcpStream) -> std::io::Result<()> {
    match cmd.cmd.to_uppercase().as_str() {
        "PING" => eval_ping(cmd.args, stream),
        _ => stream.write_all(b"-ERR unknown command\r\n"),
    }
}