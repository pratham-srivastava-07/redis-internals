for redis to understand what a client wants, it uses RESP protocol, it supports data types such as int, string, arrays and ways of conveying errors.

Resp is a req-res protocol that when client sends request, receives res, is a RESP. (every data type starts with special char and ends with \r\n)

RESP encoding for diff datat types:

for strings: starts with '+' e.g. +PING\r\n

integers: starts with ':' e.g. :15\r\n

Bulk strings: starts with '$' e.g. $4\r\nPONG\r\n

Arrays: starts with '*' e.g. if we have ["i", 500, "newone"]
                          the encoding:  *3\r\n
                                         $1\r\ni\r\n
                                         :200\r\n
                                         $6\r\newone\r\n
    
Empty starts with whatever datatype we have first and then 0, CRLF. e.g. empty string -> +0\r\n
                                                                         empty  array -> *0\r\n
                                                                         null values -> *-1\r\n

Simple strings are not binary safe, they can contain any byte, cannot contain \r\n, can be used to store any binary data, even an image


Anything that  starts with a - is an error being represented in RESP. e.g: -{msg}\r\n


---

## TTL, EXPIRE, DEL and how cleanup works

once we can SET and GET, the next real redis behaviour is: keys don't have to live forever. a key can carry an expiry, and redis has to eventually get rid of expired keys. this is the "time to live" (TTL) machinery.

### how expiry is stored

every value in the store is wrapped in an Entry:

    struct Entry {
        value: RedisValue,
        expires_at: Option<Instant>,   // None = lives forever, Some(t) = dies at t
    }

so expiry is just "an absolute point in time this key is allowed to exist until". None means no expiry. we never store "5 seconds" — we store now() + 5s and compare against it later.

### SET with EX / PX

SET can take an optional expiry option:

    SET key val EX 10     -> expire in 10 seconds
    SET key val PX 10000  -> expire in 10000 milliseconds

- EX = seconds
- PX = milliseconds

on the wire it's just an array of bulk strings like everything else:

    *5\r\n$3\r\nSET\r\n$3\r\nkey\r\n$3\r\nval\r\n$2\r\nEX\r\n$2\r\n10\r\n

internally EX/PX just decides whether we do `Instant::now() + Duration::from_secs(n)` or `from_millis(n)`, and that goes into expires_at. reply is a simple string `+OK\r\n`.

if the number isn't a valid integer -> `-ERR value is not an integer or out of range\r\n`.
if the option isn't EX/PX -> `-ERR syntax error\r\n`.

### EXPIRE

EXPIRE sets a TTL on a key that already exists (without touching its value):

    EXPIRE key 10   -> give key a 10 second expiry

reply is an integer:

- `:1\r\n` -> expiry was set (key existed)
- `:0\r\n` -> key doesn't exist, nothing to expire

so EXPIRE is really just "reach into the existing Entry and overwrite expires_at with now() + secs".

### TTL

TTL asks "how much longer does this key live?". reply is an integer with three meanings:

- `:-2\r\n` -> key doesn't exist (or already expired and got removed)
- `:-1\r\n` -> key exists but has NO expiry (immortal)
- `:N\r\n`  -> key exists and has N seconds left

the -2 / -1 convention is real redis. it lets a client tell apart "gone", "no ttl", and "N seconds left" using one integer field.

### DEL

DEL removes one or more keys explicitly (nothing to do with time — this is manual deletion):

    DEL k1 k2 k3

reply is the *count of keys that were actually deleted*, not how many you asked for:

- ask to delete 3 keys, only 2 existed -> `:2\r\n`
- none existed -> `:0\r\n`

### cleanup: how expired keys actually die

writing an expiry is easy. the interesting part is: *when* does an expired key actually get removed from memory? redis uses TWO strategies together.

**1. lazy (passive) expiration** — checked on read

whenever we touch a key (GET, TTL, etc.), before returning it we check:

    if expires_at is Some(t) and now() >= t:
        remove the key
        behave as if it never existed

so GET on an expired key returns `$-1\r\n` (nil) and quietly deletes it. TTL returns -2. the key dies *at the moment someone looks at it*.

problem: what about a key that expired but nobody ever reads again? with lazy expiration alone it sits in memory forever — a leak.

**2. active expiration** — a background sweep

so redis also periodically hunts down expired keys on its own, without waiting for a read. but it can't scan every key each time (millions of keys = big stall). instead it samples, probabilistically:

    loop:
      1. take a random sample of ~20 keys that have a TTL
      2. delete the ones that are expired
      3. if MORE THAN 25% of the sample was expired -> go again (step 1)
         else -> stop

the idea: if a random sample is full of expired keys, there are probably lots more, so keep sweeping. if the sample is mostly clean, stop — we've kept the pile of "expired but not-yet-deleted" keys under ~25% of TTL keys, which is good enough. it's a statistical bet, not a full scan.

this runs on a timer (real redis fires it ~10x/sec). in the event loop it hangs off the poll timeout — when poll wakes up (whether or not a client sent anything) we check if it's time to sweep, and if so run one active-expiration cycle.

**lazy + active together** is the whole design:
- lazy catches keys people actually read
- active catches the abandoned ones that would otherwise leak

neither alone is enough.

## Eviction Strategies 

Now Redis here is RAM bound and we cant have unbounded memory just to accomodate any number of keys.

Hence for newer data, we evict those keys / old data.

Redis is a maxmemory config that limits max amount of data it could hold.

Some of the eviction strategies are:

noeviction: no values aren't saved when new value comes after limit is reached.

allkeys-lru: removes least-recently used keys 

allkeys lfu: removes least frequently used keys.

volatile-lru: removes keys in lru with EXPIRE set

volatile-lfu: removes lfu keys with EXPIRE set


