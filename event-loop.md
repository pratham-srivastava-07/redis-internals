the problem: the old server was sequential. main thread sits inside loop { read_command() } for ONE client, and read() BLOCKS (thread literally stops until that client sends bytes). while blocked, nobody is running listener.incoming(), so a second client connects (kernel does the handshake) but just sits in the accept queue, never served.

two classic fixes:
1. thread per client - simple, but one thread per connection
2. io multiplexing / event loop - ONE thread, never blocks on any single socket. this is what real redis does. this is what we built.

## the core idea

instead of "read this client (and wait)", we ask the kernel: "which of my sockets have data ready RIGHT NOW?" then only touch those, then ask again.

the kernel does all the detection for free - network card interrupt, tcp processing, filling the socket's receive buffer. epoll is just the subscription service on top:

- register(socket)  -> "kernel, flag me when this one has something"   (say once per socket)
- poll/epoll_wait   -> "ok what happened?"  (the ONLY place the program ever sleeps)

## epoll vs iocp vs mio

epoll is linux-only (readiness model: "socket 5 has data, go read it").
IOCP is windows-only (completion model: "that read you started? done, buffer's full").
redis uses epoll (kqueue on mac). i'm on windows, so i use the mio crate - it gives an epoll-shaped api (Poll, Events, Token, Interest) and translates to the right os mechanism underneath. tokio is built on mio.

the pieces:
- Poll::new()                   -> creates the epoll instance (the kernel-side subscription list)
- Events::with_capacity(n)      -> reusable basket that poll fills with "who's ready" notes
- register(&mut sock, Token(n), Interest::READABLE) -> subscribe a socket
- poll.poll(&mut events, None)  -> block until at least one socket is ready
- Token(n)                      -> just a labeled number; how you know WHICH socket fired (stands in for the raw fd)

## two kinds of sockets (this confused me)

bind() creates the LISTENER - the doorbell. it never carries data, no PING ever flows through it. its only readiness event means "a client is waiting to be accepted".

accept() creates a NEW socket per client - the private line. all actual data flows through these. you can't register client sockets upfront because they don't exist until accept() creates them.

so: running redis-cli = the dial -> listener fires -> SERVER arm accepts + registers the new socket. typing PING = that client's own socket fires -> client arm reads/replies. the server arm never sees a single data byte.


the match: SERVER arm is a comparison against the const, tok arm is a binding that catches every other token. one poll wakeup can deliver a batch - the for loop runs the match once per event.

