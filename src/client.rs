use std::{io::{Read, Write}, net::TcpStream};

pub fn handle_client(mut stream: TcpStream) -> std::io::Result<()> {
    let mut buffer= [0; 512];

    match stream.read(&mut buffer) {
        Ok(bytes_read) => {
            if bytes_read == 0 {
                println!("Client disconnected");
            }


            println!("Received data from client....{}", String::from_utf8_lossy(&buffer[..bytes_read]));

            if let Err(e) = stream.write_all(b"Hello from server") {
                eprintln!("Error printing data to the client")
            } 
        }

        Err(e) => {
            println!("Error connecting to the server {}", e);
        }
    }

    Ok(())
}