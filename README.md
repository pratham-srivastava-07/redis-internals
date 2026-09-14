# redis-internals

A Redis server built from scratch in Rust, written to understand how Redis actually works under the hood: TCP event handling, the RESP wire protocol, in-memory storage, expiry, eviction, and crash-safe persistence.

This is not a wrapper around an existing Redis client library. Every layer — the network event loop, the protocol parser, the command dispatcher, the storage engine, and the persistence format — is implemented directly.

## Table of contents

- [What this is, in plain terms](#what-this-is-in-plain-terms)
- [Quick start](#quick-start)
- [Supported commands](#supported-commands)
- [Architecture overview](#architecture-overview)
- [How a request flows through the system](#how-a-request-flows-through-the-system)
- [The RESP protocol](#the-resp-protocol)
- [Storage and value encoding](#storage-and-value-encoding)
- [Expiry and eviction](#expiry-and-eviction)
- [Persistence (AOF)](#persistence-aof)
- [Project layout](#project-layout)
- [Testing](#testing)
- [Known limitations](#known-limitations)
- [Roadmap](#roadmap)

## What this is, in plain terms

Redis is a database that keeps all of its data in memory instead of on disk, which is what makes it extremely fast. It is normally used as a cache or a fast lookup store sitting in front of a slower, primary database.

This project reimplements the core of that idea in Rust:

- A server that many clients can talk to at once, without spawning a thread per client.
- A parser for RESP, the text-based protocol Redis clients speak.
- A key-value store held in memory, with support for values that expire automatically.
- A background process that removes old, unused, or expired data so the store does not grow without bound.
- A write-ahead log on disk, so data is not lost if the process restarts.

It is compatible enough with the real Redis wire protocol that it can be driven directly with `redis-cli`, the standard Redis command-line client.

## Quick start

Requires Rust (install via [rustup.rs](https://rustup.rs) if needed).

```sh
cargo run
```

The server listens on `127.0.0.1:7379`. In a second terminal, connect with `redis-cli`:

```sh
redis-cli -p 7379

127.0.0.1:7379> ping
PONG
127.0.0.1:7379> set greeting "hello world"
OK
127.0.0.1:7379> get greeting
"hello world"
127.0.0.1:7379> set session:token abc123 EX 60
OK
127.0.0.1:7379> ttl session:token
(integer) 60
127.0.0.1:7379> incr visits
(integer) 1
127.0.0.1:7379> object encoding visits
"int"
127.0.0.1:7379> del greeting
(integer) 1
```

## Supported commands

| Command | Behavior |
|---|---|
| `PING` | Replies `PONG`. |
| `PING <message>` | Echoes `<message>` back. |
| `SET key value` | Stores a string value. |
| `SET key value EX <seconds>` | Stores a value with a relative expiry, in seconds. |
| `SET key value PX <ms>` | Stores a value with a relative expiry, in milliseconds. |
| `SET key value PXAT <ms>` | Stores a value with an absolute expiry (Unix time in milliseconds). |
| `GET key` | Returns the value, or a nil reply if the key does not exist or has expired. |
| `INCR key` | Parses the value as an integer, adds one, stores and returns the result. Missing keys start at 0. |
| `DEL key [key ...]` | Deletes one or more keys, returns the count actually removed. |
| `EXPIRE key seconds` | Sets a relative expiry on an existing key. |
| `TTL key` | Returns seconds remaining (`-1` if the key has no expiry, `-2` if it does not exist). |
| `OBJECT ENCODING key` | Reports how a string value is internally represented: `int`, `embstr`, or `raw`. |
| `INFO KEYSPACE` | Reports key counts and average TTL for the keyspace. |

Anything else returns `-ERR unknown command`.

## Architecture overview

```
                    ┌───────────────────────────┐
                    │   TCP clients (redis-cli   │
                    │   or any RESP client)      │
                    └────────────┬───────────────┘
                                 │
                     ┌───────────▼────────────┐
                     │   events.rs             │
                     │   mio-based event loop  │
                     │   (non-blocking I/O,    │
                     │   one thread, all       │
                     │   clients multiplexed)  │
                     └───────────┬─────────────┘
                                 │ raw bytes in / out
                     ┌───────────▼────────────┐
                     │   pipeline.rs           │
                     │   buffers partial reads,│
                     │   splits multiple       │
                     │   pipelined commands    │
                     └───────────┬─────────────┘
                                 │
                     ┌───────────▼────────────┐
                     │   resp.rs / helpers/    │
                     │   RESP decoder          │
                     └───────────┬─────────────┘
                                 │ RedisCmd { name, args }
                     ┌───────────▼────────────┐
                     │   sync_tcp.rs           │
                     │   command dispatch      │
                     └──────┬─────────┬────────┘
                            │         │
               ┌────────────▼──┐   ┌──▼─────────────┐
               │  commands.rs   │   │  aof.rs         │
               │  reads/writes  │   │  logs every     │
               │  the in-memory │──▶│  mutation to    │
               │  store         │   │  disk           │
               └────────┬───────┘   └─────────────────┘
                        │
             ┌──────────▼────────────┐
             │  cmd.rs / object.rs   │
             │  in-memory store:     │
             │  HashMap<key, Obj>    │
             └──────────┬────────────┘
                        │
             ┌──────────▼────────────┐
             │  eviction.rs           │
             │  background sweep:     │
             │  expired-key removal   │
             │  + approximate LRU     │
             └─────────────────────────┘
```

## How a request flows through the system

1. **Accept.** The event loop (`events.rs`) is registered with the OS for readiness notifications on the listening socket. When a client connects, it is registered for its own notifications and tracked in a client table.
2. **Read.** When a client's socket becomes readable, its bytes are read into a per-client buffer (`pipeline.rs`). Because the socket is non-blocking, a read can return "no more data right now" without blocking the whole server — the event loop simply moves on to the next ready client.
3. **Parse.** The buffer is scanned for complete RESP command frames. A client can send several commands back to back (pipelining) before waiting for replies; each complete frame becomes a queued command, and any trailing partial frame is left in the buffer until more bytes arrive, even if a single command's data is split across multiple TCP reads.
4. **Execute.** Each queued command is dispatched by name (`sync_tcp.rs`) to its implementation in `commands.rs`, which reads or mutates the in-memory store.
5. **Persist.** If the command changed data, the resulting state is appended to the on-disk log (`aof.rs`) before the loop continues.
6. **Reply.** The response is encoded in RESP and queued for writing. If the socket cannot accept all the bytes immediately, the event loop switches that client to "waiting to write" and resumes once the socket is ready again, so one slow client cannot stall the others.
7. **Background sweep.** Roughly every 100ms, independent of any client activity, the server removes expired keys and enforces the maximum key limit via approximate LRU eviction, then flushes the log to disk.

All of this runs on a single OS thread. Concurrency comes from the event loop interleaving many clients' I/O, not from spawning a thread per connection — this is the same model the real Redis server uses.

## The RESP protocol

RESP (REdis Serialization Protocol) is the plain-text wire format Redis clients and servers use to talk to each other. Every value on the wire starts with a one-byte type marker:

| Byte | Type | Example on the wire |
|---|---|---|
| `+` | Simple string | `+OK\r\n` |
| `-` | Error | `-ERR unknown command\r\n` |
| `:` | Integer | `:42\r\n` |
| `$` | Bulk string | `$5\r\nhello\r\n` |
| `*` | Array | `*1\r\n$4\r\nPING\r\n` |

Clients always send commands as an array of bulk strings — for example, `SET key value` arrives on the wire as:

```
*3\r\n$3\r\nSET\r\n$3\r\nkey\r\n$5\r\nvalue\r\n
```

The decoder (`helpers/data_parse.rs`) reads the type byte and dispatches to the matching decoder. Arrays are decoded recursively, since each element is itself a full RESP value. The decoder enforces limits to protect against malformed or hostile input: a maximum frame size, a maximum array length, and a maximum nesting depth, so no single client can force the server into unbounded memory use or a stack overflow by sending a malicious or corrupted frame.

## Storage and value encoding

Internally, every stored value is one of three representations, chosen automatically based on the value's content and size:

- **Int** — the value parses cleanly as an integer, and is stored as one. This makes `INCR` cheap and exact.
- **EmbStr** ("embedded string") — a short string (44 bytes or fewer), stored compactly.
- **Raw** — anything longer, stored as a plain byte buffer.

This mirrors how real Redis encodes small strings, and `OBJECT ENCODING <key>` reports which representation a given key is using. Every stored entry also carries an optional expiry timestamp and a "last accessed" timestamp used for eviction (see below).

## Expiry and eviction

Two independent mechanisms keep the store from accumulating stale or excess data:

**Expired-key sweep.** Keys with a TTL are periodically sampled in small random batches. Any that have passed their expiry are removed. If a meaningful fraction of a sample turns out to be expired, another batch is sampled immediately, on the assumption there is likely more to clean up; otherwise the sweep stops for that cycle. This avoids scanning the entire keyspace on every sweep while still keeping expired keys from lingering.

**Approximate LRU eviction.** If the store exceeds its configured key limit, keys are evicted to bring it back under the limit. Rather than tracking exact access order for every key (which is expensive at scale), a small pool of eviction candidates is maintained: each sweep, a handful of random keys are sampled, their idle time is compared against what is already in the pool, and the pool is kept sorted so the most idle candidate is evicted first. This is the same approximation strategy real Redis uses for its LRU eviction policy, and it gets close to true least-recently-used behavior at a fraction of the bookkeeping cost.

## Persistence (AOF)

The server keeps an append-only file (`appendonly.aof`) as a durable log of every write. Instead of logging commands verbatim, mutations are normalized before being written:

- `INCR` is logged as the resulting `SET`, so replaying the log does not depend on re-running the arithmetic.
- Expiring keys are logged with an **absolute** expiry timestamp (not "expires in N seconds from now"), so replay produces the same expiry moment regardless of when the log is replayed.
- Deletions and keys that have already expired at write time are logged as `DEL`.

On startup, the log is replayed from the beginning to rebuild the in-memory store. If the process was killed mid-write and the log's last entry is incomplete, that trailing partial entry is discarded rather than treated as corruption, and the server starts up cleanly from everything before it.

## Project layout

```
src/
├── main.rs              Entry point — starts the event loop.
├── events.rs             The event loop itself: accept, read, dispatch, write, background sweep.
├── pipeline.rs           Buffers incoming bytes per client and splits them into complete commands.
├── resp.rs                Top-level helper: decode a RESP frame into command tokens.
├── helpers/
│   ├── utils.rs           Core RESP decoder: type-byte dispatch, line/length reading, limits.
│   ├── data_parse.rs      Per-type decoders (simple string, error, integer, bulk string, array).
│   └── port.rs             Server bind address.
├── cmd.rs                 RedisCmd (parsed command) and the Store type alias.
├── sync_tcp.rs            Dispatches a parsed command to its implementation by name.
├── commands.rs            Implementation of every supported command.
├── object.rs               The stored value type (Obj) and its encodings (Int / EmbStr / Raw).
├── types_encoding.rs       Encoding tag constants and integer parsing rules.
├── eviction.rs              Expired-key sweep and approximate LRU eviction.
├── aof.rs                    Append-only file: write, flush, and replay-on-startup.
├── stats.rs                  Keyspace statistics used by INFO KEYSPACE.
├── config.rs                  Tunable constants (eviction ratio, max key limit).
└── client.rs                  An earlier, single-client blocking prototype, kept for reference.

tests/
├── feature_audit.rs      End-to-end integration tests: spins up a real server and drives it
│                          over a socket, covering commands, edge cases, and malformed input.
└── support/                Test harness (a minimal RESP client used only by the tests).
```

## Testing

```sh
cargo test
```

Tests exist at two levels:

- **Unit tests**, alongside the modules they cover (for example `src/commands/tests.rs`, `src/eviction/tests.rs`), exercising individual functions directly.
- **Integration tests** (`tests/feature_audit.rs`) that start a real server on a real socket and drive it exactly as a client would — covering command arity errors, TTL edge cases, unicode keys, pipelined and split-across-packets commands, and oversized or malformed frames that should be rejected safely rather than crash the server.

## Known limitations

- Single-threaded: all clients are served by one event loop thread. This is deliberate (it mirrors real Redis's design), not a scalability bug, but it does mean CPU-heavy work in one command execution can delay replies to other clients.
- Only string values are supported. Lists, sets, and hashes are represented in the codebase (`ObjValue::_List`, `_Set`, `_Hash`) but not yet wired up to any commands.
- No replication, clustering, pub/sub, transactions, or Lua scripting.
- The append-only file is not compacted — it grows with every write and is never rewritten into a compact snapshot.
- `INFO` only implements the `KEYSPACE` section.

## Roadmap

- Wire up `LPUSH`/`RPUSH`/list commands using the existing `_List` variant.
- Wire up hash commands (`HSET`/`HGET`/etc.) using the existing `_Hash` variant.
- Wire up set commands using the existing `_Set` variant.
- AOF compaction / rewrite.
- Additional `INFO` sections (memory, server, stats).