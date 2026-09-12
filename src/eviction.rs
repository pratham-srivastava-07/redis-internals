use rand::{seq::IteratorRandom, Rng};
use std::collections::HashMap;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use crate::cmd::Entry;
use crate::config::{EVICTION_RATIO, MAX_KEY_LIMIT};

const SAMPLE_SIZE: usize = 20;
const EXPIRED_THRESHOLD: f64 = 0.25;
const LRU_CLOCK_MAX: u32 = 0x00FF_FFFF;
const LRU_SAMPLE_SIZE: usize = 5;
const EVICTION_POOL_SIZE: usize = 16;

pub fn get_current_clock() -> u32 {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is before Unix epoch")
        .as_secs();

    (seconds & u64::from(LRU_CLOCK_MAX)) as u32
}

pub fn get_idle_time(last_accessed_at: u32) -> u32 {
    idle_time_at(last_accessed_at, get_current_clock())
}

fn idle_time_at(last_accessed_at: u32, current: u32) -> u32 {
    current.wrapping_sub(last_accessed_at) & LRU_CLOCK_MAX
}

pub fn evict_keys(store: &mut HashMap<String, Entry>) {
    let now = Instant::now();

    // Only keys that actually carry a TTL are candidates for expiry.
    let mut candidates: Vec<String> = store
        .iter()
        .filter(|(_, e)| e.expires_at.is_some())
        .map(|(k, _)| k.clone())
        .collect();

    let mut rng = rand::thread_rng();

    loop {
        if candidates.is_empty() {
            break;
        }

        let mut expired = 0;
        let sample_n = SAMPLE_SIZE.min(candidates.len());

        for _ in 0..sample_n {
            // swap_remove keeps sampling O(1) and never picks the same key twice
            let idx = rng.gen_range(0..candidates.len());
            let key = candidates.swap_remove(idx);

            if let Some(entry) = store.get(&key) {
                if matches!(entry.expires_at, Some(exp) if now >= exp) {
                    store.remove(&key);
                    expired += 1;
                }
            }
        }

        // Sample mostly clean? Stop. Still dirty? There are probably more.
        if (expired as f64) / (sample_n as f64) <= EXPIRED_THRESHOLD {
            break;
        }
    }
}

#[derive(Debug)]
struct Candidate {
    key: String,
    idle: u32,
}

pub struct ApproxLru {
    pool: Vec<Candidate>,
}

impl ApproxLru {
    pub fn new() -> Self {
        Self {
            pool: Vec::with_capacity(EVICTION_POOL_SIZE),
        }
    }

    fn insert_candidate(&mut self, candidate: Candidate) {
        if let Some(index) = self.pool.iter().position(|entry| entry.key == candidate.key) {
            self.pool.remove(index);
        }

        let mut position = self.pool
            .iter()
            .position(|entry| entry.idle >= candidate.idle)
            .unwrap_or(self.pool.len());

        if self.pool.len() == EVICTION_POOL_SIZE {
            if position == 0 {
                return;
            }

            self.pool.remove(0);
            position -= 1;
        }

        self.pool.insert(position, candidate);
    }

    fn populate(&mut self, store: &HashMap<String, Entry>, current: u32) {
        self.pool.retain_mut(|candidate| {
            let Some(entry) = store.get(&candidate.key) else {
                return false;
            };
            candidate.idle = idle_time_at(entry.last_accessed_at, current);
            true
        });
        self.pool.sort_by_key(|candidate| candidate.idle);

        let mut rng = rand::thread_rng();
        // HashMap sampling traverses the map but retains only five entries.
        let samples = store.iter().choose_multiple(&mut rng, LRU_SAMPLE_SIZE);

        for (key, entry) in samples {
            self.insert_candidate(Candidate {
                key: key.clone(),
                idle: idle_time_at(entry.last_accessed_at, current),
            });
        }
    }

    pub fn enforce_limit(&mut self, store: &mut HashMap<String, Entry>) -> Vec<String> {
        let limit = usize::try_from(MAX_KEY_LIMIT)
            .expect("MAX_KEY_LIMIT must be nonnegative");
        let mut removed = Vec::new();

        if store.len() <= limit {
            return removed;
        }

        let now = Instant::now();
        store.retain(|key, entry| {
            let expired = entry.expires_at.is_some_and(|deadline| now >= deadline);
            if expired {
                removed.push(key.clone());
            }
            !expired
        });

        if store.len() <= limit {
            return removed;
        }

        assert!(
            EVICTION_RATIO.is_finite() && (0.0..=1.0).contains(&EVICTION_RATIO),
            "EVICTION_RATIO must be between 0 and 1"
        );
        let batch_size = (EVICTION_RATIO * limit as f64) as usize;
        let eviction_count = batch_size.max(store.len() - limit).min(store.len());
        let target_len = store.len() - eviction_count;

        while store.len() > target_len {
            self.populate(store, get_current_clock());
            let candidate = self.pool.pop()
                .expect("a nonempty store must produce an eviction candidate");

            if store.remove(&candidate.key).is_some() {
                removed.push(candidate.key);
            }
        }

        removed
    }
}

#[cfg(test)]
mod tests;
