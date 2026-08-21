use std::collections::HashMap;
use std::io::{ErrorKind, Write};
use std::time::{Duration, Instant};
use mio::{Events, Interest, Poll, Token};
use mio::net::{TcpStream};
use crate::cmd::{Entry};
use crate::helpers::port::{get_socket_address, port_and_host};
use crate::sync_tcp::respond;
use crate::pipeline::{fill_box, parse_commands, Fill};
use crate::eviction::evict_keys;

const SERVER: Token = Token(0);

// Each client owns an inbox: bytes read off the socket but not yet framed
// into complete commands. A half-arrived command lives here until the rest
// of it shows up on a later read.
struct Client {
    stream: TcpStream,
    inbox: Vec<u8>,
}

pub fn run_event_loop()-> std::io::Result<()> {
    let (_port, _host) = port_and_host();
    let addrr = get_socket_address();

    let max_clients = 20000;
    let mut events = Events::with_capacity(max_clients);

    let mut listener= mio::net::TcpListener::bind(addrr)?;

    let mut poll = Poll::new()?;


    // register the listener
    poll.registry().register(&mut listener, SERVER, Interest::READABLE)?;

    let mut clients: HashMap<Token, Client> = HashMap::new();

    let mut next_token = 1;

    // hashmap for storing everything and getting everything for that particular session
    let mut store: HashMap<String, Entry> = HashMap::new();

    let mut last_sweep = Instant::now();

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
                                clients.insert(token, Client { stream, inbox: Vec::new() });
                            },
                            Err(ref e) if e.kind() == ErrorKind::WouldBlock => {break},
                            Err(e) => return Err(e)
                        }
                    }
                },
                token => {
                    if let Some(client) = clients.get_mut(&token) {
                        // JOB 1: drain the socket into this client's inbox
                        let fill = fill_box(&mut client.stream, &mut client.inbox);
                        let mut closed = matches!(fill, Fill::Disconnected);

                        // JOB 2: run every complete command, buffering the replies
                        match parse_commands(&mut client.inbox) {
                            Ok(cmds) => {
                                let mut outbuf: Vec<u8> = Vec::new();
                                for cmd in cmds {
                                    // Vec<u8> is a Write sink, so replies pile up in
                                    // outbuf instead of hitting the socket one by one.
                                    respond(cmd, &mut store, &mut outbuf);
                                }
                                if !outbuf.is_empty() {
                                    // one write for the whole pipeline batch
                                    if client.stream.write_all(&outbuf).is_err() {
                                        closed = true;
                                    }
                                }
                            }
                            Err(()) => closed = true, // malformed frame
                        }

                        if closed {
                            clients.remove(&token);
                        }
                    }
                },
            }
        }

        if last_sweep.elapsed() >= Duration::from_millis(100) {
            evict_keys(&mut store);
            last_sweep = Instant::now();
        }
    }
}
