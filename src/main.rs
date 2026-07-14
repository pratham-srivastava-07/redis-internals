mod resp;
pub mod helpers;
mod client;
mod cmd;
mod sync_tcp;
mod commands;
mod events;


use crate::{events::run_event_loop};


fn main() -> std::io::Result<()> {
    // // println!("Hello, world!");
    // println!("Startinng redis built from scratch");
    // let addrr = SocketAddr::from(([127, 0, 0, 1], 8080));

    // let listener = TcpListener::bind(&addrr)?;
    // println!("Started a TCP connection at port {:?}", &addrr);

    // let mut client_num = 0;

    // for stream in listener.incoming() {
    //     let mut stream = match stream {
    //         Ok(s) => {
    //             println!("{:?}", &s);
    //             s
    //         },
    //         Err(e) => {
    //             eprintln!("Failed to accept connection: {}", e);
    //             continue;
    //         }
    //     };
    //     client_num += 1;
    //     println!("Client connected {}", client_num);
    //     // let peer = stream.peer_addr().ok();

    //     let peer = match stream.peer_addr() {
    //         Ok(p) => p.to_string(),
    //         Err(_) => String::from("unknown")
    //     };
    //     println!("Client connected  {:?}. Active clients: {}", peer, client_num);

    //     loop {
    //         match read_command(&mut stream) {
    //             Ok(cmd) => respond(cmd, &mut stream),
    //             Err(_) => break, // client disconnected or sent garbage
    //         }
    //     }

    //     stream.shutdown(std::net::Shutdown::Both).ok();
    //     client_num -= 1;
    //     println!("Client disconnected {:?}", peer);
    // }

    // Ok(())
    run_event_loop()
}