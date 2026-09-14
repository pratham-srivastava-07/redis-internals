use super::*;

fn command(args: &[&[u8]]) -> RedisCmd {
    RedisCmd {
        cmd: "CACHE.PUT".into(),
        args: args.iter().map(|arg| arg.to_vec()).collect(),
    }
}

fn put(
    admission: &mut Admission,
    store: &mut Store,
    lru: &mut ApproxLru,
    args: &[&[u8]],
) -> Vec<u8> {
    let mut output = Vec::new();
    admission
        .put(&command(args), store, lru, &mut output)
        .unwrap();
    output
}

fn full_store(admission: &mut Admission, lru: &mut ApproxLru) -> Store {
    let mut store = Store::new();
    for i in 0..MAX_KEY_LIMIT {
        let key = format!("hot:{i}").into_bytes();
        assert_eq!(
            put(admission, &mut store, lru, &[&key, b"v"]),
            b"+STORED\r\n"
        );
        for _ in 0..5 {
            admission.record_get(&key, true);
        }
    }
    store
}

#[test]
fn one_off_requests_do_not_displace_reused_keys() {
    let mut admission = Admission::new();
    let mut lru = ApproxLru::new();
    let mut store = full_store(&mut admission, &mut lru);
    for i in 0..100 {
        let key = format!("scan:{i}").into_bytes();
        admission.record_get(&key, false);
        assert_eq!(
            put(&mut admission, &mut store, &mut lru, &[&key, b"v"]),
            b"+REJECTED\r\n"
        );
    }
    assert_eq!(store.len(), MAX_KEY_LIMIT as usize);
    assert!(store.keys().all(|key| key.starts_with(b"hot:")));
    assert_eq!(admission.rejected, 100);
    assert_eq!(admission.evictions, 0);
}

#[test]
fn repeated_misses_can_replace_one_victim() {
    let mut admission = Admission::new();
    let mut lru = ApproxLru::new();
    let mut store = full_store(&mut admission, &mut lru);
    for _ in 0..20 {
        admission.record_get(b"new", false);
    }
    assert_eq!(
        put(&mut admission, &mut store, &mut lru, &[b"new", b"value"]),
        b"+STORED\r\n"
    );
    assert_eq!(store.len(), MAX_KEY_LIMIT as usize);
    assert!(store.contains_key(b"new".as_slice()));
    assert_eq!(admission.evictions, 1);
}

#[test]
fn validation_and_overwrite_preserve_capacity() {
    let mut admission = Admission::new();
    let mut lru = ApproxLru::new();
    let mut store = full_store(&mut admission, &mut lru);
    assert!(
        put(
            &mut admission,
            &mut store,
            &mut lru,
            &[b"hot:0", b"bad", b"EX", b"0"]
        )
        .starts_with(b"-ERR")
    );
    assert_eq!(admission.admitted, MAX_KEY_LIMIT as u64);
    assert_eq!(
        put(&mut admission, &mut store, &mut lru, &[b"hot:0", b"new"]),
        b"+STORED\r\n"
    );
    assert_eq!(store.len(), MAX_KEY_LIMIT as usize);
    assert_eq!(admission.evictions, 0);
}

#[test]
fn doorkeeper_and_aging_bound_frequency_history() {
    let mut frequency = Frequency::new();
    frequency.record(b"old");
    assert_eq!(frequency.estimate(b"old"), 1);
    assert!(frequency.counters.iter().all(|count| *count == 0));
    for _ in 0..99 {
        frequency.record(b"old");
    }
    assert_eq!(frequency.estimate(b"old"), 100);
    for _ in 0..10_000 {
        frequency.record(b"new");
    }
    assert!(frequency.estimate(b"old") < frequency.estimate(b"new"));
    assert!(frequency.resets > 0);
    assert_eq!(frequency.counters.len(), ROWS * WIDTH);
    assert_eq!(frequency.door.len(), DOOR_BITS / 64);
}

#[test]
fn ties_reject_and_expired_entries_make_room() {
    let mut admission = Admission::new();
    let mut lru = ApproxLru::new();
    let mut store = Store::new();
    for i in 0..MAX_KEY_LIMIT {
        put(
            &mut admission,
            &mut store,
            &mut lru,
            &[format!("k{i}").as_bytes(), b"v"],
        );
    }
    assert_eq!(
        put(&mut admission, &mut store, &mut lru, &[b"new", b"v"]),
        b"+REJECTED\r\n"
    );
    store.get_mut(b"k0".as_slice()).unwrap().expires_at = Some(Instant::now());
    assert_eq!(
        put(&mut admission, &mut store, &mut lru, &[b"new", b"v"]),
        b"+STORED\r\n"
    );
    assert_eq!(admission.evictions, 0);
    assert_eq!(store.len(), MAX_KEY_LIMIT as usize);
}
