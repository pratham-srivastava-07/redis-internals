The internet has spent a decade making event loops sound like dark magic. They're not. They're a coffee shop. Let me show you.

If you've ever googled "what is an event loop," you've probably come away *more* confused than when you started. The explanations are a mess because everyone tangles three separate things into one:

- the **event loop** (a pattern),
- **async/await** (syntax sugar built *on top of* the pattern), and
- **JavaScript** (one specific place the pattern happens to live).

People say "the event loop" when they mean JavaScript, say "async" when they mean the loop, and draw diagrams with seventeen boxes named "microtask queue" that help nobody.

So let's throw all that out and start from zero, with a coffee shop. By the end you'll understand event loops well enough to *build* one — in fact, if you read my [last post on I/O multiplexing](/blogs/multithreading-vs-io-multiplexing), you already built one and didn't realize it. We'll connect those dots too.

---

## The coffee shop that explains everything

Picture a tiny coffee shop with **one barista**. Call her Eve. (Eve. Event loop. I'm so sorry.)

Eve is the *only* worker. She takes orders, makes drinks, and calls out names when drinks are ready. There's a line of customers, and — this is the important part — **making a drink takes time**. The espresso machine has to pull a shot. The milk has to steam.

Now, there are two ways Eve could run this shop.

### The naive way (blocking): one customer at a time, start to finish

Eve takes Customer 1's order, then **stands and stares at the espresso machine** for 90 seconds until the drink is completely done, hands it over, and only *then* turns to Customer 2.

This is a **blocking** server. While she's frozen watching one espresso, a line of twelve people stands there fuming. Customer 12 waits 18 minutes for a coffee that took 90 seconds to make. Eve did nothing wrong except *wait*. She wasted enormous time standing idle in front of a machine that didn't need her.

This is exactly the single-threaded blocking server from my earlier posts — one slow client freezes everyone.

### The Eve way (event loop): never stand and wait

Real Eve is smarter. Here's her actual algorithm:

1. Take Customer 1's order, hit **start** on the espresso machine, put a ticket on the counter, and **immediately** turn to Customer 2.
2. Take Customer 2's order, start their drink, next.
3. When *any* machine dings "I'm done!", she grabs that finished drink, calls the name, hands it over.
4. She just keeps spinning: take an order, start a machine, hand off whatever's ready, repeat. Forever.

Eve **never stands idle waiting for one specific thing.** She only ever does work that's *ready to be done right now* — take an order that's waiting, or grab a drink that just finished. The machines do the slow waiting; Eve just reacts to dings.

**That's an event loop.** That's the whole idea. A single worker, looping forever, who never blocks on any one task and instead reacts to "this thing is ready now" notifications.

---

## Okay, the un-cute definition

An **event loop** is:

> A single thread running an infinite loop that waits for events ("this is ready"), and dispatches each one to the code that handles it — never blocking on any single operation.

Three pieces, mapping straight back to Eve:

| Coffee shop | Event loop |
|-------------|------------|
| Eve, the one barista | the single thread |
| "keep spinning forever" | the `loop { }` |
| a machine dinging "done!" | an **event** (a socket is readable, a timer fired, a file finished loading) |
| Eve reacting to the ding | the **handler** / callback for that event |
| Eve **never** standing idle at one machine | **non-blocking** — the golden rule |

And the thing that makes it possible is a magic question Eve can ask the universe:

> *"Which of my machines is ready **right now**?"*

In real code, that question is a system call — `epoll` on Linux, `kqueue` on macOS, IOCP on Windows. You hand the OS a big list of things you're waiting on, and it puts your thread to sleep until **at least one** of them is ready, then wakes you with the exact list of who's ready. No idle staring. No polling each machine one by one.

---

## "But wait, isn't this just... a loop?"

Yes! And that's the punchline nobody tells you: **an event loop is genuinely just a `while` loop with good manners.** The entire concept, in pseudocode:

```
loop {
    let ready_things = ask_os_who_is_ready();   // <- the ONE place we sleep
    for thing in ready_things {
        handle(thing);                          // react — but NEVER block here
    }
}
```

That's it. That's the terrifying "event loop." Frame this and hang it on your wall. Everything else — Node.js, Redis, Nginx, Tokio, the thing running your browser right now — is a fancier version of these five lines.

---

## You already built one (here's the receipts)

In my I/O multiplexing post, the `mio` server *was* an event loop. Let me put Eve's algorithm next to that real Rust code so you can see they're the same shape:

```rust
loop {
    // "Which of my machines is ready right now?" — Eve asks the OS.
    // The ONLY place the whole program is allowed to sleep.
    poll.poll(&mut events, None)?;

    // Handle every ready thing, one by one.
    for event in events.iter() {
        match event.token() {
            SERVER => {
                // New customers at the door — check in EVERY one that's waiting.
                loop {
                    match listener.accept() {
                        Ok((stream, _addr)) => { /* register the new client */ }
                        Err(e) if e.kind() == WouldBlock => break, // nobody else waiting
                        Err(e) => return Err(e),
                    }
                }
            }
            token => {
                // A customer sent data — take EVERYTHING they've sent so far.
                loop {
                    match stream.read(&mut buf) {
                        Ok(0) => break,                            // they left
                        Ok(n) => { /* handle these bytes */ }
                        Err(e) if e.kind() == WouldBlock => break, // nothing left to read
                        Err(_) => break,
                    }
                }
            }
        }
    }
}
```

Line for line, it's the coffee shop:

- **`poll.poll(...)`** is Eve waiting for a ding. It's the single blocking point, and it's *fine* to block here because we're blocking on **"anything at all"**, not on one specific customer. She's not staring at one machine — she's listening to the *whole room* and reacting to whatever pings first.
- **`for event in events.iter()`** is Eve working through everything that dinged while she was busy — maybe three machines finished at once; she handles all three before going quiet again.
- **the `match`** is her deciding *what kind* of ding it was: a new customer at the door (`SERVER`) versus a specific drink being ready (`token`). (That's the exact "connect vs. data" distinction — a new customer arriving is a different event from an existing order finishing.)

### Why each arm has its own inner `loop`

Look closely and you'll notice something I glossed over in the five-line pseudocode: **both match arms have a little `loop` inside them** that keeps going until the OS returns `WouldBlock` (its way of saying *"nothing left right now"*). We accept connections until there are none, and read bytes until there are none. Why not just accept one and read once?

**Because there's only one Eve.** This event loop is *not concurrent* — a single thread does everything, and nobody else is coming to pick up what she leaves behind. Picture Eve walking up to a machine that has dinged, grabbing **one** finished drink, and walking away while **two more sit there getting cold**. No other barista exists to grab them. Those customers wait forever. So when Eve goes to a ready machine, she must clear it *completely* in that one visit.

Same with sockets. When a client's socket is ready, it might have **one** command buffered — or five, glued together. If you `read()` once and move on, the other four just sit in the buffer, unserved. In a multi-threaded server you might get away with it (another thread could wrap back around), but here **there is no other thread**. One visit is all a socket gets, so you drain it dry.

And there's a second, sharper reason hiding underneath: the OS only pings you when **new** data *arrives*, not continuously while old data sits unread (this is called *edge-triggered* notification). So if you leave bytes unread, you may **never be pinged about them again** — the loop goes back to sleep and that half-read command hangs forever. (This exact mistake once made my Redis server go deaf on the *second* command a client sent — a genuinely maddening bug I wrote up separately. The fix was precisely these inner `loop`-until-`WouldBlock` drains.)

So the refined rule: **when a source is ready, fully drain it before moving on** — accept until no more connections, read until no more bytes. The single-threaded, non-concurrent nature of the loop is *exactly why* this is mandatory rather than optional.

So if you've written that `mio` loop, congratulations, you've built an event loop from scratch. You can stop being intimidated by the phrase now.

---

## The One Commandment: THOU SHALT NOT BLOCK THE LOOP

Here's the rule that, if you tattoo it on your brain, makes you understand event loops better than 90% of blog commenters:

> **Never do slow, blocking work inside the loop. Ever.**

Why? Because there's **only one Eve.** If Eve decides to personally hand-grind a bag of beans for two minutes in the middle of her shift, the *entire shop freezes.* Every machine can be dinging like crazy — nobody gets served, because the one worker is busy blocking. Orders pile up. Customers riot. Yelp reviews are written.

In code terms, imagine sticking this in your event loop:

```rust
loop {
    poll.poll(&mut events, None)?;
    for event in events.iter() {
        std::thread::sleep(Duration::from_secs(5)); // 😱 THE CRIME
        // ...
    }
}
```

That `sleep` is Eve deciding to take a 5-second nap for *every single event.* One slow customer, and all 10,000 others are frozen behind them. You've reinvented the blocking server you were trying to escape — but worse, because now they all share one thread.

This is why event-loop code is **allergic to blocking calls**: no blocking `read()`, no synchronous file reads, no `sleep`, no "just quickly query the database" that takes 200ms. Everything slow must be handed to a machine (the OS, a thread pool, an async task) so Eve can keep spinning. This single constraint is the source of *all* the "why is my Node.js server frozen?" Stack Overflow questions in history.

---

## "So where does async/await come in?"

Great question, and here's the demystification: **async/await is just a nicer way to write the handlers so you don't accidentally block.**

Remember, the loop only works if every handler is quick and non-blocking. But writing non-blocking code by hand is *miserable* — it turns into a spaghetti of callbacks ("when this is ready, do that, and when *that's* ready, do this other thing..."). This is the infamous **callback hell**.

`async`/`await` is syntax that lets you *write* code that looks blocking and sequential:

```rust
let data = socket.read().await;   // looks like it blocks...
process(data).await;              // ...but it doesn't!
```

...while the compiler secretly rewrites it into "register interest, yield control back to the loop, and resume here later when ready." The `.await` is you telling Eve: *"start this, and feel free to go serve other customers; wake me up here when it's done."* It's the espresso machine ticket, in language form.

So the stack, bottom to top:

1. **OS readiness API** (`epoll`/`kqueue`/IOCP) — the machines that ding.
2. **The event loop** — Eve, spinning forever, reacting to dings.
3. **async/await** — a pleasant language for writing Eve's reactions without callback spaghetti.

People online smash all three into the phrase "the event loop" and that's *why* it's confusing. They're three different floors of the same building.

---

## Where you've been standing in event loops this whole time

Event loops aren't exotic. You are marinating in them:

- **Your browser** runs one. Every click, scroll, `setTimeout`, and network response is an event dispatched by the loop. That's why one heavy `for` loop in JavaScript freezes the whole page — you blocked Eve, and now the UI can't even scroll.
- **Node.js** is famously "a runtime built around an event loop" (using a C library called libuv). One thread, tons of connections. Very Eve.
- **Nginx** serves ridiculous traffic on few threads by — you guessed it — event loops, one per CPU core.
- **Redis** — the whole reason I started this rabbit hole — is (mostly) single-threaded and runs an event loop. Its commands are so fast (microseconds) that Eve can serve thousands of clients without ever needing a second barista. Adding threads would just mean baristas fighting over the same espresso machine (locks). One fast Eve is simpler *and* quicker.
- **Tokio**, Rust's async runtime, is a production-grade event loop (well, one per core) with a beautiful `async`/`await` front end. When you write `async fn` in Rust, you're writing handlers for Eve.

---

## The tiny mental model to keep forever

If you remember nothing else, remember this:

> An **event loop** is one worker in an infinite loop, who asks the OS *"what's ready?"*, does only the work that's ready, and **never stands around waiting**. It scales to thousands of connections because waiting is offloaded to the OS, not to the worker.

And the two rules:

1. **The only blocking allowed is "wait for *anything* to be ready"** (`poll.poll`). That's Eve listening to the whole room.
2. **Never block on *one specific* thing inside the loop.** That's Eve napping while the shop burns.

That's genuinely the whole concept. Everything else — microtask queues, `setImmediate` vs `setTimeout`, executors, reactors, wakers — is just detail bolted onto these five lines:

```
loop {
    let ready = ask_os_who_is_ready();
    for thing in ready { handle(thing); }
}
```

The next time someone online makes event loops sound like arcane sorcery, just picture one very tired barista named Eve, refusing to stand still. You now understand them better than the diagram with seventeen boxes.

*Thanks for reading. Go tip your barista — she's running your entire internet.* ☕🦀
