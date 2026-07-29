use rand::Rng;
use std::collections::HashMap;
use crate::cmd::Entry;
use std::time::Instant;

const SAMPLE_SIZE: usize = 20;
const EXPIRED_THRESHOLD: f64 = 0.25;

pub fn evict_keys(store: &mut HashMap<String, Entry>) {
    let now = Instant::now();
    
    let mut candidates: Vec<String> = store.iter().filter(|(_, e)| e.expires_at.is_some()).map(|(k, _)| k.clone()).collect();

    let mut rng = rand::thread_rng();

    loop {
        if candidates.is_empty() {
            break;
        }

        let mut expired = 0;

        let sample_n = SAMPLE_SIZE.min(candidates.len());

        for _ in 0..sample_n {
            let idx = rng.gen_range(0..candidates.len());
            let key = candidates.swap_remove(idx);

            if let Some(entry) = store.get(&key) {
                if matches!(entry.expires_at, Some(exp) if now >= exp) {
                    store.remove(&key);
                    expired += 1;
                }
            }

        }

        if (expired as f64) / (sample_n as f64) <= EXPIRED_THRESHOLD {
            break;
        }
    }

}
