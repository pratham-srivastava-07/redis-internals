use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;

pub fn handle_client(stream: TcpStream) -> std::io::Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut stream = stream;
    let mut line = String::new();

    loop {
        line.clear();
        let bytes_read = reader.read_line(&mut line)?;  // reads until '\n'

        if bytes_read == 0 {
            return Ok(()); // client disconnected
        }

        let msg = line.trim_end();          // strip the \r\n
        println!("Received full line: {}", msg);
        stream.write_all(&format!("+{}\r\n", msg).into_bytes())?;
    }
}