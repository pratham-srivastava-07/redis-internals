use std::net::{TcpListener, SocketAddr};
mod resp;
pub mod helpers;

mod cmd;
mod sync_tcp;
mod commands;

use crate::sync_tcp::{read_command, respond};


fn main() -> std::io::Result<()> {
    println!("Hello, world!");
    let addrr = SocketAddr::from(([127, 0, 0, 1], 8080));

    let listener = TcpListener::bind(&addrr)?;

    let mut client_num = 0;

    for stream in listener.incoming() {
        let stream = match stream {
            Ok(s) => {
                println!("{:?}", &s);
                s
            },
            Err(e) => {
                eprintln!("Failed to accept connection: {}", e);
                continue;
            }
        };
        client_num += 1;
        let peer = stream.peer_addr().ok();
        println!("Client connected  {:?}. Active clients: {}", peer, client_num);

        loop {
            match read_command(stream.try_clone()?) {
                Ok(cmd) => respond(cmd, &stream),
                Err(_) => break, // client disconnected or sent garbage
            }
        }

        stream.shutdown(std::net::Shutdown::Both).ok();
        client_num -= 1;
        println!("Client disconnected {:?}", peer);
    }

    Ok(())
}