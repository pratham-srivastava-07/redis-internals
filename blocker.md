# The Case of the Second Command That Never Was

*I built a Redis server from scratch. It answered the first command perfectly, then went completely deaf. Here's the debugging story — and the one weird rule about event loops that fixed it.*

---

I'm building a Redis clone from scratch in Rust, mostly to finally understand the machinery I've taken for granted my whole career: event loops, non-blocking sockets, the RESP protocol. Most days it's a delight.

Then I hit a bug so strange it felt supernatural.

My server would happily answer the **first** command from a client. And then — nothing. The second command vanished into the void. No response. No error. No crash. Not even a **log line**. The server just sat there, humming, pretending nothing had happened.

This is the story of hunting that bug down. If you've ever written an event loop with `epoll`, `kqueue`, `mio`, or IOCP, the ending will save you an afternoon.

---

## The crime scene

Here's what I saw. I started my server, connected with `redis-cli`, and typed `PING`:

```
Running async TCP server on port 8080 and host 127.0.0.1
Reached before entering the loop
Inside loop
client connected
Inside loop
Reached here
Token(1)
Reached here
```

```
127.0.0.1:8080> PING
PONG
```

`PONG`. Beautiful. The whole pipeline worked — connection accepted, command parsed, response written. I felt like a systems programmer.

Then I typed `PING` a second time:

```
127.0.0.1:8080> PING
```

And the cursor just... sat there. Blinking. Forever.

I flipped over to my server logs, expecting *something*. Here's the eerie part:

**Nothing new printed. Not a single line.**

Remember, the very first thing inside my loop is a `println!("Inside loop")`. For the second `PING`, that never fired. Which meant my event loop wasn't slow, wasn't erroring — it was **asleep**, and the second command couldn't wake it up.

That's a genuinely creepy symptom. The bytes left `redis-cli`. They arrived at my machine. And my server, sitting in its loop waiting for exactly those bytes, never noticed.

---

## The suspects

I did what everyone does: I started accusing things.

**Suspect #1: Is `redis-cli` broken?** I was using an ancient Windows build. Maybe it wasn't sending the second command? Ruled out — I watched the packets; the bytes were absolutely going out.

**Suspect #2: Is the connection dead?** Maybe the socket closed after the first command? No — `redis-cli` still thought it was connected, and my server never logged a disconnect.

**Suspect #3: Is my command parser hanging?** Maybe `read_command` was stuck in an infinite loop on the second call? No — because `println!("Inside loop")` comes *before* any parsing, and even *that* didn't run. The code never got that far.

That last realization was the turning point. The problem wasn't in my parsing, or my response, or my client. The problem was upstream of all of it:

> `poll.poll()` — the call that's supposed to wake up when a socket has data — was never waking up for the second command.

The OS was refusing to tell my program that new data had arrived. Why on earth would it do that?

---

## The event loop, and the thing I didn't understand about it

Here's the relevant skeleton of my server. It's a standard single-threaded event loop built on [`mio`](https://github.com/tokio-rs/mio) (Rust's cross-platform wrapper over `epoll`/`kqueue`/IOCP):

```rust
loop {
    poll.poll(&mut events, None)?;   // sleep until a socket is ready
    println!("Inside loop");

    for event in events.iter() {
        match event.token() {
            SERVER => { /* a new client connected -> accept it */ }
            token  => { /* an existing client sent data -> read it */ }
        }
    }
}
```

The idea: one thread sleeps in `poll.poll()` until the OS says "hey, one of your sockets is ready." Then it handles whatever's ready and goes back to sleep. This is how a single thread serves thousands of clients — it never blocks on any one of them.

And here was the buggy code that handled a client sending data. Look closely, because the bug is hiding in plain sight:

```rust
// THE BUGGY VERSION
token => {
    if let Some(mut stream) = clients.get_mut(&token) {
        // read_command
        match read_command(&mut stream) {
            Ok(cmd) => respond(cmd, &mut stream),
            Err(_)  => break,
        }
    }
}
```

And `read_command`, the buggy version:

```rust
// THE BUGGY VERSION
pub fn read_command<S: Read>(con: &mut S) -> Result<RedisCmd, DecodeError> {
    let mut buffer = [0u8; 512];

    let n = match con.read(&mut buffer) {   // <-- reads EXACTLY ONCE
        Ok(n) => n,
        Err(_) => return Err(DecodeError),
    };

    let tokens = decode_array_string(&buffer[..n])?;
    // ... parse and return one command ...
}
```

It reads once, parses one command, returns. Clean, simple, obvious. And **completely wrong** for an event loop — for a reason that comes down to two words.

---

## The reveal: edge-triggered readiness

Here's the thing nobody tells you clearly when you start with event loops. There are **two ways** the OS can notify you that a socket is readable:

- **Level-triggered:** "This socket has data" — and it'll keep telling you *as long as there's unread data*. Forgiving.
- **Edge-triggered:** "Data just *arrived*" — it tells you **once**, at the moment of transition from no-data to data. Then it shuts up until the *next* new arrival.

`mio` uses **edge-triggered** notifications. And edge-triggered mode comes with one ironclad rule that, if you break it, produces *exactly* my bug:

> **After a readable event, you must keep calling `read()` until it returns `WouldBlock`. Only then does the OS re-arm the socket to notify you again.**

Read that again, because it's the whole ballgame. In edge-triggered mode, the OS only re-arms its "I'll wake you up" promise once you've drained the socket dry — once a `read()` has actually come back empty-handed with a `WouldBlock` error meaning "nothing left right now."

Now look at my buggy code again. `read_command` calls `con.read()` **exactly once**. It reads the first `PING`, gets a command, and returns — *without ever reading again to hit `WouldBlock`*.

So here's the fatal sequence:

1. First `PING` arrives → socket goes readable → `poll` wakes me → I read it once → `PONG`. 
2. But I **never** read again to get `WouldBlock`. So `mio` never re-arms the socket.
3. Second `PING` arrives... and because the socket was never re-armed, **the OS generates no new event.**
4. `poll.poll()` sleeps forever, waiting for a notification that will never come.
5. `println!("Inside loop")` never runs. The command is invisible. I lose my mind.

The bytes were sitting *right there* in the kernel's receive buffer. My server just never got the tap on the shoulder telling it to look.

---

## Why it was *especially* brutal on Windows

One detail made this even nastier: I'm on Windows, where `mio` runs on **IOCP** rather than Linux's `epoll`. The IOCP backend is *stricter* about this contract — it flatly will not deliver another readiness event until you've reached `WouldBlock`. On some Linux configurations you can get away with sloppiness here for a while; on Windows the bug shows up immediately, on literally the second command. So if you're following a Linux-based tutorial on a Windows machine and things mysteriously freeze — this might be you.

---

## The fix: drain until `WouldBlock`

The fix has two parts.

**Part 1:** Teach `read_command` to report *why* it stopped — specifically, to distinguish "no more data right now" (`WouldBlock`, the good kind of stop that re-arms the socket) from a real disconnect or a decode error:

```rust
// THE FIXED VERSION
#[derive(Debug)]
pub enum ReadError {
    WouldBlock,     // socket drained — this is what re-arms edge-triggered mode
    Disconnected,   // client closed the connection
    Decode,         // bytes arrived but weren't a valid command
}

pub fn read_command<S: Read>(con: &mut S) -> Result<RedisCmd, ReadError> {
    let mut buffer = [0u8; 512];

    let n = match con.read(&mut buffer) {
        Ok(0)  => return Err(ReadError::Disconnected),        // clean close
        Ok(n)  => n,
        Err(ref e) if e.kind() == ErrorKind::WouldBlock
               => return Err(ReadError::WouldBlock),          // <-- the crucial case
        Err(_) => return Err(ReadError::Disconnected),
    };

    let tokens = decode_array_string(&buffer[..n]).map_err(|_| ReadError::Decode)?;
    if tokens.is_empty() {
        return Err(ReadError::Decode);
    }

    Ok(RedisCmd { cmd: tokens[0].clone(), args: tokens[1..].to_vec() })
}
```

**Part 2:** In the event loop, **loop** the read until it returns `WouldBlock` — actually draining the socket, which is what makes `mio` re-arm it:

```rust
// THE FIXED VERSION
token => {
    if let Some(mut stream) = clients.get_mut(&token) {
        let mut closed = false;

        // Keep reading until the socket is dry. THIS re-arms edge-triggered mode.
        loop {
            match read_command(&mut stream) {
                Ok(cmd) => respond(cmd, &mut store, &mut stream),

                Err(ReadError::WouldBlock) => break,   // drained — wait for next event

                Err(ReadError::Disconnected) => { closed = true; break; }
                Err(ReadError::Decode)       => { closed = true; break; }
            }
        }

        if closed {
            clients.remove(&token);   // clean up the dead client
        }
    }
}
```

Now when a `PING` comes in, the loop reads it, responds, loops again, gets `WouldBlock`, and *breaks* — and that `WouldBlock` is the magic handshake that tells `mio`: "okay, re-arm this socket; wake me when more arrives." The second `PING` now generates an event. The third. Forever.

`PONG`. `PONG`. `PONG`. I have never been so happy to see a four-letter word repeat.

---

## The bug's evil twin: the accept loop

Once I understood the rule, I realized my `SERVER` arm — the code that accepts *new connections* — had the **exact same bug**. It accepted one connection and stopped:

```rust
// ALSO BUGGY: accepts once, never drains to WouldBlock
SERVER => {
    let (mut stream, _addr) = listener.accept()?;
    // ... register the one client ...
}
```

The listener socket is edge-triggered too. If two clients connect at nearly the same instant, you get **one** readable event — and if you only `accept()` once, the second client is stranded in the accept queue forever, exactly like my second `PING`. Same disease, same cure: loop until `WouldBlock`.

```rust
// FIXED: accept until the queue is dry
SERVER => {
    loop {
        match listener.accept() {
            Ok((mut stream, _addr)) => { /* register the new client */ }
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => break,
            Err(e) => return Err(e),
        }
    }
}
```

The rule isn't "drain your client sockets." It's **"drain everything you registered — clients *and* the listener — until `WouldBlock`."** Every edge-triggered source, every time.

---

## What I actually learned

This bug looked like sorcery, but the lesson is dead simple and I'll never forget it:

> **In an edge-triggered event loop, "readable" means "new data *arrived*," not "data is *available*." You get told once. If you don't drain the socket to `WouldBlock`, you never get told again.**

A few takeaways that outlived this specific bug:

1. **The absence of a log is itself a clue.** My breakthrough came from noticing that `"Inside loop"` *didn't* print. That ruled out my entire parsing/response layer and pointed straight at `poll`. When debugging, pay attention to the code that *should* have run and didn't.

2. **`WouldBlock` is not an error. It's a signal.** It literally means "you've read everything; stop and wait." Treating it as a normal, expected outcome — rather than lumping it into a generic error — is the difference between a working event loop and a broken one.

3. **Your platform's backend matters.** The same code that limps along on Linux can fail instantly on Windows/IOCP. If a networking tutorial "just works" for everyone but you, check what readiness backend you're actually running on.

4. **Simple code can be exactly wrong.** My buggy `read_command` was clean, readable, and obviously correct — for a *blocking* server. Dropped into an event loop, its very simplicity was the bug. Context decides correctness.

---

If you're building your own Redis, your own web server, or anything on top of an event loop and it mysteriously handles the first request and then goes silent — check whether you're draining your sockets to `WouldBlock`. There's a very good chance you've met the same ghost I did.

*I'm building Redis from scratch in Rust and writing up what I learn along the way — this is the first entry in that ongoing journey. Next up: **RESP**, the wire protocol behind every Redis command — what those cryptic `*1\r\n$4\r\nPING\r\n` bytes actually mean, and why length-prefixing everything is quietly brilliant. Stay tuned.* 🦀
