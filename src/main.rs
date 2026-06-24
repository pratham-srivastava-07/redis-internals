use std::net::{TcpListener, SocketAddr};
mod client;
use client::handle_client;

fn main() -> std::io::Result<()> {
    println!("Hello, world!");
    let addrr = SocketAddr::from(([127, 0, 0, 1], 8080));

    let listener = TcpListener::bind(&addrr)?;

    for stream in listener.incoming() {
        // let mut stream = stream?;
        println!("{:?}", stream);
        let client = handle_client(stream?);
        println!("{:?}", client);
        // todo!()
    }

    Ok(())
}


