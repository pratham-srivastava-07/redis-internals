use std::net::{TcpListener, SocketAddr};
mod client;
use client::handle_client;
mod resp;
pub mod helpers;



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

        if let Err(e) = handle_client(stream) {
            eprintln!("Error {}", e);
        }

        println!("Client disconnected {:?}", peer);
        client_num -= 1;
        
    }

    Ok(())
}


