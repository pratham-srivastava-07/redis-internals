use super::*;
use crate::cmd::{RedisCmd, RedisValue};
use crate::stats::Stat;

fn entry(last_accessed_at: u32) -> Entry {
    Entry {
        value: RedisValue::Int(1),
        expires_at: None,
        last_accessed_at,
    }
}

fn candidate(key: &str, idle: u32) -> Candidate {
    Candidate { key: key.into(), idle }
}

fn filled_store(count: usize) -> HashMap<String, Entry> {
    let current = get_current_clock();
    (0..count).map(|index| (index.to_string(), entry(current))).collect()
}

#[test]
fn idle_time_handles_clock_wrap() {
    assert_eq!(idle_time_at(10, 25), 15);
    assert_eq!(idle_time_at(25, 25), 0);
    assert_eq!(idle_time_at(LRU_CLOCK_MAX, 0), 1);
    assert_eq!(idle_time_at(LRU_CLOCK_MAX - 2, 3), 6);
}

#[test]
fn pool_orders_candidates_and_updates_duplicates() {
    let mut lru = ApproxLru::new();
    lru.insert_candidate(candidate("a", 10));
    lru.insert_candidate(candidate("b", 70));
    lru.insert_candidate(candidate("c", 30));
    assert_eq!(
        lru.pool.iter().map(|entry| entry.idle).collect::<Vec<_>>(),
        vec![10, 30, 70]
    );

    lru.insert_candidate(candidate("b", 5));
    assert_eq!(lru.pool.len(), 3);
    assert_eq!(lru.pool.first().unwrap().key, "b");
    assert_eq!(lru.pool.pop().unwrap().key, "c");
}

#[test]
fn full_pool_retains_better_candidates() {
    let mut lru = ApproxLru::new();
    for idle in 1..=EVICTION_POOL_SIZE as u32 {
        lru.insert_candidate(candidate(&idle.to_string(), idle));
    }

    lru.insert_candidate(candidate("recent", 0));
    assert_eq!(lru.pool.len(), EVICTION_POOL_SIZE);
    assert!(!lru.pool.iter().any(|entry| entry.key == "recent"));

    lru.insert_candidate(candidate("old", 100));
    assert_eq!(lru.pool.len(), EVICTION_POOL_SIZE);
    assert_eq!(lru.pool.first().unwrap().idle, 2);
    assert_eq!(lru.pool.last().unwrap().key, "old");
}

#[test]
fn population_drops_missing_keys_and_refreshes_recreated_keys() {
    let mut lru = ApproxLru::new();
    lru.insert_candidate(candidate("missing", 100));
    lru.insert_candidate(candidate("recreated", 100));

    let store = HashMap::from([
        ("recreated".into(), entry(100)),
        ("old".into(), entry(10)),
    ]);
    lru.populate(&store, 100);

    assert_eq!(lru.pool.len(), 2);
    assert_eq!(lru.pool.first().unwrap().key, "recreated");
    assert_eq!(lru.pool.first().unwrap().idle, 0);
    assert_eq!(lru.pool.pop().unwrap().key, "old");
}

#[test]
fn capacity_evicts_a_batch_and_handles_oversized_stores() {
    let limit = usize::try_from(MAX_KEY_LIMIT).unwrap();
    let batch = (EVICTION_RATIO * limit as f64) as usize;
    let mut lru = ApproxLru::new();
    assert!(lru.enforce_limit(&mut HashMap::new()).is_empty());
    let mut store = filled_store(limit);
    assert!(lru.enforce_limit(&mut store).is_empty());

    store.insert("extra".into(), entry(get_current_clock()));
    let removed = lru.enforce_limit(&mut store);
    assert_eq!(removed.len(), batch.max(1));
    assert_eq!(store.len(), limit + 1 - removed.len());
    assert!(removed.iter().all(|key| !store.contains_key(key)));

    let mut oversized = filled_store(limit * 3);
    let removed = lru.enforce_limit(&mut oversized);
    assert_eq!(oversized.len(), limit);
    assert_eq!(removed.len(), limit * 2);
}

#[test]
fn expired_keys_are_reclaimed_before_live_keys() {
    let limit = usize::try_from(MAX_KEY_LIMIT).unwrap();
    let mut store = filled_store(limit);
    let mut expired = entry(0);
    expired.expires_at = Some(Instant::now());
    store.insert("expired".into(), expired);

    let removed = ApproxLru::new().enforce_limit(&mut store);
    assert_eq!(removed, vec!["expired"]);
    assert_eq!(store.len(), limit);
}

#[test]
fn eviction_deletions_replay_after_writes() {
    let limit = usize::try_from(MAX_KEY_LIMIT).unwrap();
    let mut store = HashMap::new();
    let mut stats = Stat::new();
    let mut lru = ApproxLru::new();
    let mut log = Vec::new();

    for index in 0..=limit {
        let cmd = RedisCmd {
            cmd: "SET".into(),
            args: vec![index.to_string(), "value".into()],
        };
        log.extend(crate::aof::aof_entry(&cmd).unwrap());
        crate::sync_tcp::respond(cmd, &mut store, &mut stats, &mut Vec::new());
        let removed = lru.enforce_limit(&mut store);
        if !removed.is_empty() {
            log.extend(crate::aof::aof_entry(&RedisCmd {
                cmd: "DEL".into(),
                args: removed,
            }).unwrap());
        }
    }

    let mut replayed = HashMap::new();
    for cmd in crate::pipeline::parse_commands(&mut log).unwrap() {
        crate::sync_tcp::respond(cmd, &mut replayed, &mut stats, &mut Vec::new());
    }
    assert!(store.len() < limit);
    assert_eq!(store.len(), replayed.len());
    assert!(store.keys().all(|key| replayed.contains_key(key)));
}
