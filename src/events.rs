use mio::net::TcpStream;
use mio::{Events, Interest, Poll, Token};
use std::collections::{HashMap, VecDeque};
use std::io::{self, ErrorKind, Write};
use std::time::{Duration, Instant};

use crate::aof::Aof;
use crate::cmd::{RedisCmd, Store};
use crate::commands::Outcome;
use crate::eviction::{ApproxLru, evict_keys};
use crate::helpers::port::get_socket_address;
use crate::pipeline::{Fill, fill_box, parse_commands};
use crate::stats::Stat;
use crate::sync_tcp::respond;

const SERVER: Token = Token(0);
const MAX_CLIENTS: usize = 20_000;

struct Client {
    stream: TcpStream,
    inbox: Vec<u8>,
    pending: VecDeque<RedisCmd>,
    output: Vec<u8>,
    written: usize,
    read_closed: bool,
}

fn enforce_capacity(lru: &mut ApproxLru, store: &mut Store, aof: &mut Aof) -> io::Result<()> {
    aof.record_deleted(&lru.enforce_limit(store))
}

fn flush_output<W: Write>(
    writer: &mut W,
    output: &mut Vec<u8>,
    written: &mut usize,
) -> io::Result<bool> {
    while *written < output.len() {
        match writer.write(&output[*written..]) {
            Ok(0) => return Err(io::ErrorKind::WriteZero.into()),
            Ok(n) => *written += n,
            Err(e) if e.kind() == ErrorKind::Interrupted => continue,
            Err(e) if e.kind() == ErrorKind::WouldBlock => return Ok(false),
            Err(e) => return Err(e),
        }
    }
    output.clear();
    *written = 0;
    Ok(true)
}

fn drive_client(
    client: &mut Client,
    store: &mut Store,
    stats: &mut Stat,
    lru: &mut ApproxLru,
    aof: &mut Aof,
) -> io::Result<bool> {
    loop {
        match flush_output(&mut client.stream, &mut client.output, &mut client.written) {
            Ok(false) => return Ok(false),
            Err(_) => return Ok(true),
            Ok(true) => {}
        }
        let Some(cmd) = client.pending.pop_front() else {
            return Ok(client.read_closed);
        };
        let outcome = respond(&cmd, store, stats, &mut client.output)?;
        if outcome == Outcome::Modified {
            aof.record_mutation(&cmd, store)?;
        }
        enforce_capacity(lru, store, aof)?;
    }
}

pub fn run_event_loop() -> io::Result<()> {
    let mut listener = mio::net::TcpListener::bind(get_socket_address())?;
    let mut poll = Poll::new()?;
    poll.registry()
        .register(&mut listener, SERVER, Interest::READABLE)?;
    let mut events = Events::with_capacity(1024);
    let mut clients: HashMap<Token, Client> = HashMap::new();
    let mut next_token = 1;
    let mut store = Store::new();
    Aof::load(&mut store)?;
    let mut aof = Aof::new()?;
    let mut lru = ApproxLru::new();
    enforce_capacity(&mut lru, &mut store, &mut aof)?;
    aof.flush()?;
    let mut stats = Stat::new();
    let mut last_sweep = Instant::now();

    loop {
        let timeout = Duration::from_millis(100).saturating_sub(last_sweep.elapsed());
        poll.poll(&mut events, Some(timeout))?;
        for event in events.iter() {
            if event.token() == SERVER {
                loop {
                    match listener.accept() {
                        Ok((mut stream, _)) => {
                            if clients.len() >= MAX_CLIENTS {
                                continue;
                            }
                            let token = Token(next_token);
                            next_token += 1;
                            poll.registry()
                                .register(&mut stream, token, Interest::READABLE)?;
                            clients.insert(
                                token,
                                Client {
                                    stream,
                                    inbox: Vec::new(),
                                    pending: VecDeque::new(),
                                    output: Vec::new(),
                                    written: 0,
                                    read_closed: false,
                                },
                            );
                        }
                        Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                        Err(e) if e.kind() == ErrorKind::Interrupted => continue,
                        Err(e) => return Err(e),
                    }
                }
                continue;
            }
            let token = event.token();
            let Some(client) = clients.get_mut(&token) else {
                continue;
            };
            let mut close = false;
            if event.is_readable() && !client.read_closed && client.output.is_empty() {
                match fill_box(&mut client.stream, &mut client.inbox) {
                    Fill::Disconnected => client.read_closed = true,
                    Fill::Invalid => close = true,
                    Fill::Ok => {}
                }
                if !close {
                    match parse_commands(&mut client.inbox) {
                        Ok(commands) => client.pending.extend(commands),
                        Err(_) => close = true,
                    }
                }
            }
            if !close {
                close = drive_client(client, &mut store, &mut stats, &mut lru, &mut aof)?;
            }
            if close {
                poll.registry().deregister(&mut client.stream)?;
                clients.remove(&token);
            } else {
                // Pause reads while a reply is blocked; resume on writable readiness.
                let interest = if !client.output.is_empty() {
                    Interest::WRITABLE
                } else {
                    Interest::READABLE
                };
                poll.registry()
                    .reregister(&mut client.stream, token, interest)?;
            }
        }
        if last_sweep.elapsed() >= Duration::from_millis(100) {
            evict_keys(&mut store);
            aof.flush()?;
            last_sweep = Instant::now();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partial_reply_resumes_after_would_block() {
        struct Writer {
            calls: usize,
            received: Vec<u8>,
        }
        impl Write for Writer {
            fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
                self.calls += 1;
                if self.calls == 2 {
                    return Err(ErrorKind::WouldBlock.into());
                }
                let n = bytes.len().min(2);
                self.received.extend_from_slice(&bytes[..n]);
                Ok(n)
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        let mut writer = Writer {
            calls: 0,
            received: Vec::new(),
        };
        let mut output = b"+PONG\r\n".to_vec();
        let mut written = 0;
        assert!(!flush_output(&mut writer, &mut output, &mut written).unwrap());
        assert_eq!(written, 2);
        assert!(flush_output(&mut writer, &mut output, &mut written).unwrap());
        assert_eq!(writer.received, b"+PONG\r\n");
        assert!(output.is_empty());
    }
}
