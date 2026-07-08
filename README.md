# redis-internals

A toy Redis server written in Rust, built from scratch to learn how Redis works under the hood — TCP handling, the RESP protocol, and command evaluation.

## What works

- TCP server listening on `127.0.0.1:8080`
- RESP protocol decoding: simple strings, errors, integers, bulk strings, and arrays
- RESP encoding for replies
- Commands:
  - `PING` → `+PONG`
  - `PING <message>` → echoes the message back as a bulk string
  - Anything else → `-ERR unknown command`

## Running

```sh
cargo run
```

Then connect with redis-cli from another terminal:

```sh
redis-cli -p 8080
127.0.0.1:8080> ping
PONG
127.0.0.1:8080> ping hello
"hello"
```

## Project structure

```
src/
├── main.rs            server entry point: accept loop, per-client command loop
├── sync_tcp.rs        reads a command from the socket, dispatches, writes the reply
├── cmd.rs             RedisCmd struct (command name + args)
├── commands.rs        command implementations (PING) and RESP reply encoding
├── resp.rs            top-level decode helpers (frame -> Vec<String>)
└── helpers/
    ├── utils.rs       decode_one (type-byte dispatch) and read_length
    └── data_parse.rs  per-type decoders: +, -, :, $, *
```

## How decoding works

Every RESP frame starts with a type byte:

| byte | type          | example              |
|------|---------------|----------------------|
| `+`  | simple string | `+OK\r\n`            |
| `-`  | error         | `-ERR oops\r\n`      |
| `:`  | integer       | `:42\r\n`            |
| `$`  | bulk string   | `$5\r\nhello\r\n`    |
| `*`  | array         | `*1\r\n$4\r\nPING\r\n` |

`decode_one` looks at the first byte and dispatches to the matching decoder. Each decoder returns the value plus the number of bytes it consumed, which is what lets the array decoder walk through its elements (arrays are decoded recursively — each element is itself a full RESP frame).

Clients send commands as arrays of bulk strings, so `PING hello` arrives as `*2\r\n$4\r\nPING\r\n$5\r\nhello\r\n`.

## Known limitations

- Single-threaded: one client is served at a time
- One `read()` per command (max 512 bytes) — commands split across TCP packets or larger than 512 bytes fail
- Decoded values are passed around as `Box<dyn Any>` and downcast at use sites; a proper `enum Value` would be the cleaner design
- No storage yet — `GET`/`SET` are the obvious next step
