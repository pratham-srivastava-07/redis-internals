use super::*;                       

#[test]
fn ping_replies_pong() {
    let mut out: Vec<u8> = Vec::new();
    eval_ping(vec![], &mut out).unwrap();
    assert_eq!(out, b"+PONG\r\n");
}

fn run(store: &mut HashMap<String, Entry>, stats: &mut Stat,  name: &str, args: &[&str]) -> Vec<u8> {
    let mut out = Vec::new();
    crate::sync_tcp::respond(crate::cmd::RedisCmd {
        cmd: name.into(),
        args: args.iter().map(|arg| arg.to_string()).collect(),
    }, store, stats, &mut out);
    out
}

#[test]
fn set_selects_encoding_and_get_preserves_text() {
    let mut store = HashMap::new();
    let long = "x".repeat(45);
    let short = "x".repeat(44);
    for (value, encoding) in [("42", "int"), ("hello", "embstr"), (short.as_str(), "embstr"), (long.as_str(), "raw"), ("001", "embstr")] {
        assert_eq!(run(&mut store, "SET", &["k", value]), b"+OK\r\n");
        assert_eq!(run(&mut store, "GET", &["k"]), format!("${}\r\n{value}\r\n", value.len()).as_bytes());
        assert_eq!(run(&mut store, "OBJECT", &["ENCODING", "k"]), format!("${}\r\n{encoding}\r\n", encoding.len()).as_bytes());
    }
}

#[test]
fn incr_creates_counter_and_preserves_ttl() {
    let mut store = HashMap::new();
    assert_eq!(run(&mut store, "incr", &["k"]), b":1\r\n");
    let deadline = Instant::now() + Duration::from_secs(60);
    store.get_mut("k").unwrap().expires_at = Some(deadline);
    assert_eq!(run(&mut store, "INCR", &["k"]), b":2\r\n");
    assert_eq!(store["k"].expires_at, Some(deadline));
    assert_eq!(run(&mut store, "GET", &["k"]), b"$1\r\n2\r\n");
}

#[test]
fn incr_rejects_invalid_values_without_mutating_them() {
    let mut store = HashMap::new();
    for value in ["hello", "", "+1", "01", "-0", " 1", "1 ", "1.0", "9223372036854775808", "-9223372036854775809"] {
        run(&mut store, "SET", &["k", value]);
        assert_eq!(run(&mut store, "INCR", &["k"]), b"-ERR value is not an integer or out of range\r\n");
        assert_eq!(run(&mut store, "GET", &["k"]), format!("${}\r\n{value}\r\n", value.len()).as_bytes());
    }
    run(&mut store, "SET", &["k", "9223372036854775807"]);
    assert_eq!(run(&mut store, "INCR", &["k"]), b"-ERR increment or decrement would overflow\r\n");
    assert!(matches!(store["k"].value, RedisValue::Int(i64::MAX)));
    run(&mut store, "SET", &["k", "-9223372036854775808"]);
    assert_eq!(run(&mut store, "INCR", &["k"]), b":-9223372036854775807\r\n");
}

#[test]
fn incr_converts_byte_encodings_and_rejects_other_types() {
    let mut store = HashMap::new();
    for value in [RedisValue::Raw(b"12".to_vec()), RedisValue::EmbStr(b"12".to_vec().into_boxed_slice())] {
        store.insert("k".into(), Entry { value, expires_at: None });
        assert_eq!(run(&mut store, "INCR", &["k"]), b":13\r\n");
        assert!(matches!(store["k"].value, RedisValue::Int(13)));
    }
    store.insert("k".into(), Entry { value: RedisValue::_List(vec![]), expires_at: None });
    assert_eq!(run(&mut store, "INCR", &["k"]), b"-WRONGTYPE Operation against a key holding the wrong kind of value\r\n");
    assert!(matches!(store["k"].value, RedisValue::_List(_)));
}

#[test]
fn expired_keys_are_absent_and_incr_checks_arity() {
    let mut store = HashMap::new();
    assert!(run(&mut store, "INCR", &[]).starts_with(b"-ERR"));
    assert!(run(&mut store, "INCR", &["k", "extra"]).starts_with(b"-ERR"));
    assert!(store.is_empty());
    store.insert("k".into(), Entry { value: RedisValue::Int(99), expires_at: Some(Instant::now()) });
    assert_eq!(run(&mut store, "INCR", &["k"]), b":1\r\n");
    assert_eq!(store["k"].expires_at, None);
    store.get_mut("k").unwrap().expires_at = Some(Instant::now());
    assert_eq!(run(&mut store, "OBJECT", &["ENCODING", "k"]), b"$-1\r\n");
    assert!(!store.contains_key("k"));
}

#[test]
fn aof_encoded_counter_commands_replay_through_dispatch() {
    let mut bytes = Vec::new();
    for (name, args) in [("SET", vec!["k".into(), "40".into()]), ("INCR", vec!["k".into()]), ("INCR", vec!["k".into()])] {
        let cmd = crate::cmd::RedisCmd { cmd: name.into(), args };
        bytes.extend(crate::aof::aof_entry(&cmd).unwrap());
    }
    let mut store = HashMap::new();
    for cmd in crate::pipeline::parse_commands(&mut bytes).unwrap() {
        crate::sync_tcp::respond(cmd, &mut store, &mut Vec::new());
    }
    assert_eq!(run(&mut store, "GET", &["k"]), b"$2\r\n42\r\n");
}
