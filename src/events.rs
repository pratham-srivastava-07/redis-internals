use std::collections::HashMap;
use std::io::{Error, ErrorKind};
use mio::{Events, Interest, Poll, Token};
use mio::net::{TcpStream};
use crate::cmd::{Entry, RedisValue};
use crate::helpers::port::{get_socket_address, port_and_host};
use crate::sync_tcp::{read_command, respond, ReadError};

const SERVER: Token = Token(0);


pub fn run_event_loop()-> std::io::Result<()> {
    let (_port, _host) = port_and_host();
    let addrr = get_socket_address();

    let max_clients = 20000;
    let mut events = Events::with_capacity(max_clients);

    let mut listener= mio::net::TcpListener::bind(addrr)?;

    let mut poll = Poll::new()?;


    // register the listener
    poll.registry().register(&mut listener, SERVER, Interest::READABLE)?;

    let mut clients: HashMap<Token, TcpStream> = HashMap::new();
    // let mut store = 

    let mut next_token = 1;

    // hashmap for storing everything and getting everything for that particular session
    let mut store: HashMap<String, Entry> = HashMap::new();


    loop {
        poll.poll(&mut events, None)?;

        for event in events.iter() {
            match event.token() {
                SERVER => {
                    loop {
                        match listener.accept() {
                            Ok((mut stream, _addrr)) => {
                                let token = Token(next_token);
                                next_token += 1;
                                poll.registry().register(&mut stream, token, Interest::READABLE)?;
                                clients.insert(token, stream);
                            },
                            Err(ref e) if e.kind() == ErrorKind::WouldBlock => {break},
                            Err(e) => return Err(e)

                        }
                    }
                    
                },
                token => {
                    if let Some(mut stream) = clients.get_mut(&token) {
                        let mut closed = false;
                        // read_command 
                        loop {
                            match read_command(&mut stream) {
                                Ok(cmd) => respond(cmd, &mut store, &mut stream),

                                Err(ReadError::WouldBlock) => {
                                    break;
                                }

                                Err(ReadError::Disconnected) => {
                                    closed = true;
                                    break;
                                }

                                Err(ReadError::Decode) => {
                                    closed = true;
                                    break;
                                }
                            }
                        }
                        if closed {
                            clients.remove(&token).ok_or_else(|| {
                                Error::new(ErrorKind::NotFound, "client not found")
                            })?;
                        }
                        // execute and 
                    }
                }
            }
        }
    }
}