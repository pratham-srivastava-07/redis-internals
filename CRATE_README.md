# Vynk

A Redis implementation with extra features, written from scratch in Rust. It includes approximate LRU eviction, append-only persistence, and TinyLFU-style admission control.

Vynk is a standalone server binary, not a Rust client library or a full Redis replacement.

## Install and run

```sh
cargo install vynk --locked
mkdir vynk-data
cd vynk-data
vynk
```

Vynk listens on `127.0.0.1:7379` and creates `appendonly.aof` in the current directory. Restart from that directory to restore state. Use `vynk --help` or `vynk --version` for CLI information.

In another terminal, connect using a separately installed `redis-cli`:

```sh
redis-cli -p 7379
```

```text
SET greeting "hello"
GET greeting
CACHE.PUT product:42 "keyboard" EX 60
TTL product:42
CACHE.STATS
```

## Commands

- `PING [message]`
- `SET key value [EX seconds | PX milliseconds | PXAT timestamp]`
- `GET key`, `INCR key`, `DEL key [key ...]`
- `EXPIRE key seconds`, `TTL key`
- `OBJECT ENCODING key`, `INFO [KEYSPACE]`
- `CACHE.PUT key value [EX seconds | PX milliseconds | PXAT timestamp]`
- `CACHE.STATS`

Commands use RESP2 arrays of bulk strings. Keys and string values preserve arbitrary bytes.

## Admission control

Ordinary `SET` retains its existing insertion behavior. Use `CACHE.PUT` when an application can tolerate a cache fill being declined. It returns `STORED` or `REJECTED`.

Valid GET requests update recent frequency history, including misses. When the cache is full, a new key must have a higher estimated frequency than the victim selected by the approximate LRU pool. Ties preserve the resident key. Accepted replacements evict one victim; existing keys can be overwritten without a frequency comparison.

The frequency sketch and Bloom doorkeeper use 9 KiB plus fixed bookkeeping. History ages and restarts empty. CACHE.STATS reports hits, misses, admissions, rejections, admission evictions, aging passes, and metadata bytes. This implements TinyLFU-style admission, without a W-TinyLFU recency window.

## Current boundaries

- Experimental `0.1.0` release. Not a drop-in Redis replacement.
- A fixed 100-key capacity, not a byte-based memory budget. Ordinary capacity eviction can remove a batch of 40 keys. Sampling traverses the store.
- A single event loop handles commands and disk synchronization. No authentication, TLS, replication, clustering, transactions, or collection commands.
- AOF writes are periodically synchronized. Acknowledgement does not guarantee power-loss durability. The log grows without compaction and is read into memory at startup.
- Client input buffers are bounded at 64 MiB. These limits do not bound total process memory.
- No measured throughput or latency advantage over Redis is claimed.

## Verification and source

The repository includes Rust unit tests and isolated TCP integration tests covering commands, admission, malformed inputs, expiration, output backpressure, and restart recovery.

```sh
cargo test -- --test-threads=1
cargo test --release -- --test-threads=1
```

Port 7379 must be free for the integration tests. Test servers use temporary data directories.

[Source and full documentation](https://github.com/pratham-srivastava-07/vynk) · [Issues](https://github.com/pratham-srivastava-07/vynk/issues) · [TinyLFU paper](https://arxiv.org/abs/1512.00727)
