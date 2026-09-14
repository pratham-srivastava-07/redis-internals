use std::collections::hash_map::RandomState;
use std::hash::BuildHasher;
use std::io::{self, Write};
use std::time::Instant;

use crate::cmd::{RedisCmd, Store};
use crate::commands::{Outcome, set_command};
use crate::config::MAX_KEY_LIMIT;
use crate::eviction::ApproxLru;

const WIDTH: usize = 1024;
const ROWS: usize = 4;
const DOOR_BITS: usize = 8192;
const SAMPLE_WINDOW: usize = 1000;

struct Frequency {
    counters: Box<[u16]>,
    door: Box<[u64]>,
    hashes: [RandomState; ROWS],
    samples: usize,
    resets: u64,
}

impl Frequency {
    fn new() -> Self {
        Self {
            counters: vec![0; WIDTH * ROWS].into_boxed_slice(),
            door: vec![0; DOOR_BITS / 64].into_boxed_slice(),
            hashes: std::array::from_fn(|_| RandomState::new()),
            samples: 0,
            resets: 0,
        }
    }

    fn hashes(&self, key: &[u8]) -> [usize; ROWS] {
        std::array::from_fn(|row| self.hashes[row].hash_one(key) as usize)
    }

    fn seen(&self, hashes: &[usize; ROWS]) -> bool {
        hashes[..2].iter().all(|hash| {
            let bit = hash % DOOR_BITS;
            self.door[bit / 64] & (1 << (bit % 64)) != 0
        })
    }

    fn record(&mut self, key: &[u8]) {
        if self.samples == SAMPLE_WINDOW {
            for count in &mut self.counters {
                *count /= 2;
            }
            self.door.fill(0);
            self.samples /= 2;
            self.resets += 1;
        }
        let hashes = self.hashes(key);
        if self.seen(&hashes) {
            for (row, hash) in hashes.iter().enumerate() {
                let count = &mut self.counters[row * WIDTH + hash % WIDTH];
                *count = count.saturating_add(1);
            }
        } else {
            for hash in &hashes[..2] {
                let bit = hash % DOOR_BITS;
                self.door[bit / 64] |= 1 << (bit % 64);
            }
        }
        self.samples += 1;
    }

    fn estimate(&self, key: &[u8]) -> u16 {
        let hashes = self.hashes(key);
        let count = hashes
            .iter()
            .enumerate()
            .map(|(row, hash)| self.counters[row * WIDTH + hash % WIDTH])
            .min()
            .unwrap();
        count.saturating_add(u16::from(self.seen(&hashes)))
    }
}

pub struct Admission {
    frequency: Frequency,
    hits: u64,
    misses: u64,
    admitted: u64,
    rejected: u64,
    evictions: u64,
}

impl Admission {
    pub fn new() -> Self {
        Self {
            frequency: Frequency::new(),
            hits: 0,
            misses: 0,
            admitted: 0,
            rejected: 0,
            evictions: 0,
        }
    }

    pub fn record_get(&mut self, key: &[u8], hit: bool) {
        self.frequency.record(key);
        if hit {
            self.hits += 1;
        } else {
            self.misses += 1;
        }
    }

    pub fn put<W: Write>(
        &mut self,
        cmd: &RedisCmd,
        store: &mut Store,
        lru: &mut ApproxLru,
        output: &mut W,
    ) -> io::Result<(Outcome, Option<Vec<u8>>)> {
        // Reuse SET validation without changing the live store before admission.
        let mut prepared = Store::new();
        let mut reply = Vec::new();
        if set_command(&cmd.args, &mut prepared, &mut reply)? == Outcome::Rejected {
            output.write_all(&reply)?;
            return Ok((Outcome::Rejected, None));
        }
        let (key, entry) = prepared.into_iter().next().unwrap();
        let now = Instant::now();
        store.retain(|_, entry| entry.expires_at.is_none_or(|end| end > now));
        let expired = entry.expires_at.is_some_and(|end| end <= now);
        let limit = usize::try_from(MAX_KEY_LIMIT).expect("MAX_KEY_LIMIT must be nonnegative");
        let mut victim = None;
        if !expired && !store.contains_key(&key) && store.len() >= limit {
            victim = lru.victim(store);
            let admit = victim.as_ref().is_some_and(|victim| {
                self.frequency.estimate(&key) > self.frequency.estimate(victim)
            });
            if !admit {
                self.rejected += 1;
                output.write_all(b"+REJECTED\r\n")?;
                return Ok((Outcome::Rejected, None));
            }
        }
        if let Some(victim) = &victim {
            store.remove(victim);
            self.evictions += 1;
        }
        store.insert(key, entry);
        self.admitted += 1;
        output.write_all(b"+STORED\r\n")?;
        Ok((Outcome::Modified, victim))
    }

    pub fn stats<W: Write>(&self, args: &[Vec<u8>], output: &mut W) -> io::Result<Outcome> {
        if !args.is_empty() {
            output.write_all(b"-ERR wrong number of arguments for 'cache.stats' command\r\n")?;
            return Ok(Outcome::Rejected);
        }
        let metadata = std::mem::size_of::<Self>()
            + std::mem::size_of_val(self.frequency.counters.as_ref())
            + std::mem::size_of_val(self.frequency.door.as_ref());
        let body = format!(
            "# Cache\r\nhits:{}\r\nmisses:{}\r\nadmitted:{}\r\nrejected:{}\r\nadmission_evictions:{}\r\nfrequency_resets:{}\r\nadmission_metadata_bytes:{metadata}\r\n",
            self.hits,
            self.misses,
            self.admitted,
            self.rejected,
            self.evictions,
            self.frequency.resets
        );
        write!(output, "${}\r\n{body}\r\n", body.len())?;
        Ok(Outcome::ReadOnly)
    }
}

#[cfg(test)]
mod tests;
