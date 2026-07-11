use std::collections::HashMap;
use mio::{Events, Interest, Poll, Token};
use mio::net::{TcpListener, TcpStream};
use crate::helpers::port::{get_socket_address, port_and_host};
use crate::sync_tcp::{read_command, respond};

const SERVER: Token = Token(0);


pub fn run_event_loop()-> std::io::Result<()> {
    let (port, host) = port_and_host();
    let addrr = get_socket_address();

    println!("Running async TCP server on port {:?} and host {:?}", port, host);

    let max_clients = 20000;
    let mut events = Events::with_capacity(max_clients);
    let mut listener= mio::net::TcpListener::bind(addrr)?;
    let mut poll = Poll::new()?;

    // register the listener
    poll.registry().register(&mut listener, SERVER, Interest::READABLE)?;

    let mut clients: HashMap<Token, TcpStream> = HashMap::new();

    let mut next_token = 1;

    loop {
        poll.poll(&mut events, None)?;

        for event in events.iter() {
            match event.token() {
                SERVER => {
                    let (mut stream, _addr) = listener.accept()?;
                    let token = Token(next_token);
                    next_token += 1;
                    poll.registry().register(&mut stream, token, Interest::READABLE)?;
                    clients.insert(token, stream);
                },
                token => {
                    if let Some(mut stream) = clients.get_mut(&token) {
                        // read_command 
                        match read_command(&mut stream) {
                            Ok(cmd) => respond(cmd, &mut stream),
                            Err(_) => break,
                        }
                        // execute and 
                    }
                }
            }
        }
    }

    Ok(())
}