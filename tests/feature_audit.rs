mod support;
#[path = "admission/mod.rs"]
mod admission;

use std::fs;
use std::io::Write;
use std::net::Shutdown;
use std::thread;
use std::time::Duration;
use support::{Client, Reply, Server, bulk, encode};

fn ok() -> Reply {
    Reply::Simple(b"OK".to_vec())
}
fn nil() -> Reply {
    Reply::Bulk(None)
}

fn expect_error(reply: Reply) {
    assert!(
        matches!(reply, Reply::Error(_)),
        "expected RESP error, got {reply:?}"
    );
}

fn commands(input: &[&[&str]]) -> Vec<Vec<String>> {
    input
        .iter()
        .map(|args| args.iter().map(|arg| (*arg).to_owned()).collect())
        .collect()
}

#[test]
fn ping_basic_message_and_arity() {
    let _server = Server::new();
    let mut client = Client::connect();
    assert_eq!(client.cmd(&["PING"]), Reply::Simple(b"PONG".to_vec()));
    assert_eq!(client.cmd(&["ping", "hello"]), bulk("hello"));
    expect_error(client.cmd(&["PING", "a", "b"]));
}

#[test]
fn unknown_command() {
    let _server = Server::new();
    let mut client = Client::connect();
    expect_error(client.cmd(&["NO_SUCH_COMMAND"]));
}

#[test]
fn set_get_overwrite_empty_and_unicode() {
    let _server = Server::new();
    let mut client = Client::connect();
    assert_eq!(client.cmd(&["SET", "k", "first"]), ok());
    assert_eq!(client.cmd(&["GET", "k"]), bulk("first"));
    assert_eq!(client.cmd(&["SET", "k", ""]), ok());
    assert_eq!(client.cmd(&["GET", "k"]), bulk(""));
    assert_eq!(client.cmd(&["SET", "", "namaste ☃"]), ok());
    assert_eq!(client.cmd(&["GET", ""]), bulk("namaste ☃"));
    assert_eq!(client.cmd(&["GET", "missing"]), nil());
}

#[test]
fn set_missing_arguments() {
    let _server = Server::new();
    let mut client = Client::connect();
    expect_error(client.cmd(&["SET"]));
    expect_error(client.cmd(&["SET", "k"]));
}

#[test]
fn get_missing_arguments() {
    let _server = Server::new();
    let mut client = Client::connect();
    expect_error(client.cmd(&["GET"]));
}

#[test]
fn get_rejects_extra_arguments() {
    let _server = Server::new();
    let mut client = Client::connect();
    expect_error(client.cmd(&["GET", "k", "extra"]));
}

#[test]
fn set_rejects_trailing_garbage() {
    let _server = Server::new();
    let mut client = Client::connect();
    assert_eq!(client.cmd(&["SET", "k", "old"]), ok());
    expect_error(client.cmd(&["SET", "k", "new", "EX", "60", "garbage"]));
    assert_eq!(client.cmd(&["GET", "k"]), bulk("old"));
}

#[test]
fn set_rejects_zero_expiry() {
    let _server = Server::new();
    let mut client = Client::connect();
    expect_error(client.cmd(&["SET", "k", "v", "EX", "0"]));
}

#[test]
fn set_rejects_invalid_expiry() {
    let _server = Server::new();
    let mut client = Client::connect();
    expect_error(client.cmd(&["SET", "k", "v", "PX", "-1"]));
    expect_error(client.cmd(&["SET", "k", "v", "EX", "abc"]));
    expect_error(client.cmd(&["SET", "k", "v", "INVALID", "1"]));
}

#[test]
fn ttl_missing_and_persistent() {
    let _server = Server::new();
    let mut client = Client::connect();
    assert_eq!(client.cmd(&["TTL", "missing"]), Reply::Integer(-2));
    assert_eq!(client.cmd(&["SET", "k", "v"]), ok());
    assert_eq!(client.cmd(&["TTL", "k"]), Reply::Integer(-1));
}

#[test]
fn ttl_missing_arguments() {
    let _server = Server::new();
    let mut client = Client::connect();
    expect_error(client.cmd(&["TTL"]));
}

#[test]
fn ttl_rejects_extra_arguments() {
    let _server = Server::new();
    let mut client = Client::connect();
    expect_error(client.cmd(&["TTL", "k", "extra"]));
}

#[test]
fn set_overwrite_clears_ttl() {
    let _server = Server::new();
    let mut client = Client::connect();
    assert_eq!(client.cmd(&["SET", "k", "v", "EX", "60"]), ok());
    assert_eq!(client.cmd(&["SET", "k", "new"]), ok());
    assert_eq!(client.cmd(&["TTL", "k"]), Reply::Integer(-1));
}

#[test]
fn del_multiple_duplicate_and_missing() {
    let _server = Server::new();
    let mut client = Client::connect();
    assert_eq!(client.cmd(&["SET", "a", "v"]), ok());
    assert_eq!(client.cmd(&["SET", "b", "v"]), ok());
    assert_eq!(
        client.cmd(&["DEL", "a", "a", "b", "missing"]),
        Reply::Integer(2)
    );
    assert_eq!(client.cmd(&["GET", "a"]), nil());
    expect_error(client.cmd(&["DEL"]));
}

#[test]
fn expire_existing() {
    let _server = Server::new();
    let mut client = Client::connect();
    assert_eq!(client.cmd(&["SET", "k", "v"]), ok());
    assert_eq!(client.cmd(&["EXPIRE", "k", "60"]), Reply::Integer(1));
    assert_eq!(client.cmd(&["GET", "k"]), bulk("v"));
}

#[test]
fn expire_missing_returns_zero() {
    let _server = Server::new();
    let mut client = Client::connect();
    assert_eq!(client.cmd(&["EXPIRE", "missing", "60"]), Reply::Integer(0));
}

#[test]
fn expire_one_argument_returns_resp_error() {
    let _server = Server::new();
    let mut client = Client::connect();
    expect_error(client.cmd(&["EXPIRE", "k"]));
}

#[test]
fn expire_missing_arguments() {
    let _server = Server::new();
    let mut client = Client::connect();
    expect_error(client.cmd(&["EXPIRE"]));
}

#[test]
fn expire_rejects_extra_arguments() {
    let _server = Server::new();
    let mut client = Client::connect();
    assert_eq!(client.cmd(&["SET", "k", "v"]), ok());
    expect_error(client.cmd(&["EXPIRE", "k", "60", "garbage"]));
}

#[test]
fn expire_negative_deletes() {
    let _server = Server::new();
    let mut client = Client::connect();
    assert_eq!(client.cmd(&["SET", "k", "v"]), ok());
    assert_eq!(client.cmd(&["EXPIRE", "k", "-1"]), Reply::Integer(1));
    assert_eq!(client.cmd(&["GET", "k"]), nil());
}

#[test]
fn incr_creation_negative_invalid_and_overflow() {
    let _server = Server::new();
    let mut client = Client::connect();
    assert_eq!(client.cmd(&["INCR", "n"]), Reply::Integer(1));
    assert_eq!(client.cmd(&["INCR", "n"]), Reply::Integer(2));
    assert_eq!(client.cmd(&["SET", "n", "-2"]), ok());
    assert_eq!(client.cmd(&["INCR", "n"]), Reply::Integer(-1));
    assert_eq!(client.cmd(&["SET", "n", "abc"]), ok());
    expect_error(client.cmd(&["INCR", "n"]));
    assert_eq!(client.cmd(&["GET", "n"]), bulk("abc"));
    assert_eq!(client.cmd(&["SET", "n", "9223372036854775807"]), ok());
    expect_error(client.cmd(&["INCR", "n"]));
    assert_eq!(client.cmd(&["GET", "n"]), bulk("9223372036854775807"));
    expect_error(client.cmd(&["INCR"]));
    expect_error(client.cmd(&["INCR", "n", "extra"]));
}

#[test]
fn object_encoding_and_missing() {
    let _server = Server::new();
    let mut client = Client::connect();
    assert_eq!(client.cmd(&["SET", "k", "42"]), ok());
    assert_eq!(client.cmd(&["OBJECT", "ENCODING", "k"]), bulk("int"));
    assert_eq!(
        client.cmd(&["SET", "k", "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"]),
        ok()
    );
    assert_eq!(client.cmd(&["OBJECT", "ENCODING", "k"]), bulk("embstr"));
    assert_eq!(
        client.cmd(&["SET", "k", "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"]),
        ok()
    );
    assert_eq!(client.cmd(&["OBJECT", "ENCODING", "k"]), bulk("raw"));
    assert_eq!(client.cmd(&["OBJECT", "ENCODING", "missing"]), nil());
    expect_error(client.cmd(&["OBJECT"]));
    expect_error(client.cmd(&["OBJECT", "ENCODING", "k", "extra"]));
}

#[test]
fn binary_values_round_trip() {
    let _server = Server::new();
    let mut client = Client::connect();
    let value = b"\x00\xff\xfe\r\n";
    assert_eq!(client.bytes(&[b"SET", b"k", value]), ok());
    assert_eq!(
        client.bytes(&[b"GET", b"k"]),
        Reply::Bulk(Some(value.to_vec()))
    );
}

#[test]
fn binary_keys_remain_distinct() {
    let _server = Server::new();
    let mut client = Client::connect();
    assert_eq!(client.bytes(&[b"SET", b"\xff", b"one"]), ok());
    assert_eq!(client.bytes(&[b"SET", b"\xfe", b"two"]), ok());
    assert_eq!(client.bytes(&[b"GET", b"\xff"]), bulk("one"));
}

#[test]
fn px_expiration() {
    let _server = Server::new();
    let mut client = Client::connect();
    assert_eq!(client.cmd(&["SET", "k", "v", "PX", "30"]), ok());
    thread::sleep(Duration::from_millis(150));
    assert_eq!(client.cmd(&["GET", "k"]), nil());
    assert_eq!(client.cmd(&["TTL", "k"]), Reply::Integer(-2));
}

#[test]
fn ex_expiration() {
    let _server = Server::new();
    let mut client = Client::connect();
    assert_eq!(client.cmd(&["SET", "k", "v", "EX", "1"]), ok());
    thread::sleep(Duration::from_millis(1100));
    assert_eq!(client.cmd(&["GET", "k"]), nil());
}

#[test]
fn ttl_rounds_seconds() {
    let _server = Server::new();
    let mut client = Client::connect();
    assert_eq!(client.cmd(&["SET", "k", "v", "PX", "1800"]), ok());
    assert_eq!(client.cmd(&["TTL", "k"]), Reply::Integer(2));
}

#[test]
fn del_does_not_count_expired_keys() {
    let _server = Server::new();
    let mut client = Client::connect();
    assert_eq!(
        client.pipeline(&commands(&[
            &["SET", "k", "v"],
            &["EXPIRE", "k", "0"],
            &["DEL", "k"],
        ])),
        vec![ok(), Reply::Integer(1), Reply::Integer(0)]
    );
}

#[test]
fn expire_cannot_revive_expired_key() {
    let _server = Server::new();
    let mut client = Client::connect();
    assert_eq!(
        client.pipeline(&commands(&[
            &["SET", "k", "v"],
            &["EXPIRE", "k", "0"],
            &["EXPIRE", "k", "60"],
            &["GET", "k"],
        ])),
        vec![ok(), Reply::Integer(1), Reply::Integer(0), nil()]
    );
}

#[test]
fn info_is_resp_bulk_and_contains_keyspace_counts() {
    let _server = Server::new();
    let mut client = Client::connect();
    assert_eq!(client.cmd(&["SET", "k", "v"]), ok());
    let Reply::Bulk(Some(info)) = client.cmd(&["INFO"]) else {
        panic!("INFO must return a bulk string");
    };
    assert!(String::from_utf8_lossy(&info).contains("keys=1"));
}

#[test]
fn pipeline_preserves_order() {
    let _server = Server::new();
    let mut client = Client::connect();
    assert_eq!(
        client.pipeline(&commands(&[
            &["SET", "n", "1"],
            &["INCR", "n"],
            &["GET", "n"],
            &["DEL", "n"],
            &["GET", "n"],
        ])),
        vec![ok(), Reply::Integer(2), bulk("2"), Reply::Integer(1), nil()]
    );
}

#[test]
fn fragmented_payload_resumes() {
    let _server = Server::new();
    let mut client = Client::connect();
    let wire = encode(&[b"SET", b"k", b"hello"]);
    client.stream.write_all(&wire[..wire.len() - 4]).unwrap();
    thread::sleep(Duration::from_millis(40));
    client.stream.write_all(&wire[wire.len() - 4..]).unwrap();
    assert_eq!(client.read().unwrap(), ok());
    assert_eq!(client.cmd(&["GET", "k"]), bulk("hello"));
}

#[test]
fn fragmented_header_does_not_crash() {
    let mut server = Server::new();
    let mut client = Client::connect();
    client.stream.write_all(b"*1\r").unwrap();
    thread::sleep(Duration::from_millis(100));
    server.assert_alive();
    client.stream.write_all(b"\n$4\r\nPING\r\n").unwrap();
    assert_eq!(client.read().unwrap(), Reply::Simple(b"PONG".to_vec()));
}

fn malformed_is_rejected(wire: &[u8]) {
    let mut server = Server::new();
    let mut client = Client::connect();
    client.stream.write_all(wire).unwrap();
    match client.read() {
        Ok(Reply::Error(_)) => {}
        Err(error)
            if matches!(
                error.kind(),
                std::io::ErrorKind::UnexpectedEof | std::io::ErrorKind::ConnectionReset
            ) => {}
        other => panic!("expected protocol rejection, got {other:?}"),
    }
    server.assert_alive();
}

#[test]
fn malformed_bulk_terminator_is_rejected() {
    malformed_is_rejected(b"*1\r\n$4\r\nPINGxx");
}

#[test]
fn malformed_array_header_is_rejected() {
    malformed_is_rejected(b"*1xx$4\r\nPING\r\n");
}

#[test]
fn overflowing_resp_length_does_not_crash() {
    let mut server = Server::new();
    let mut client = Client::connect();
    client
        .stream
        .write_all(format!("*{}\r\n", "9".repeat(100)).as_bytes())
        .unwrap();
    thread::sleep(Duration::from_millis(100));
    server.assert_alive();
}

#[test]
fn signed_integer_command_element_does_not_crash() {
    let mut server = Server::new();
    let mut client = Client::connect();
    client.stream.write_all(b"*1\r\n:-1\r\n").unwrap();
    thread::sleep(Duration::from_millis(100));
    server.assert_alive();
}

#[test]
fn huge_expiry_is_rejected_without_crash() {
    let mut server = Server::new();
    let mut client = Client::connect();
    expect_error(client.cmd(&["SET", "k", "v", "EX", "18446744073709551615"]));
    server.assert_alive();
}

#[test]
fn concurrent_clients_do_not_lose_increments() {
    let _server = Server::new();
    let threads: Vec<_> = (0..8)
        .map(|_| {
            thread::spawn(|| {
                let mut client = Client::connect();
                for _ in 0..25 {
                    assert!(matches!(client.cmd(&["INCR", "n"]), Reply::Integer(_)));
                }
            })
        })
        .collect();
    for handle in threads {
        handle.join().unwrap();
    }
    assert_eq!(Client::connect().cmd(&["GET", "n"]), bulk("200"));
}

#[test]
fn large_value_round_trip() {
    let _server = Server::new();
    let mut client = Client::connect();
    let value = vec![b'x'; 256 * 1024];
    assert_eq!(client.bytes(&[b"SET", b"k", &value]), ok());
    let Reply::Bulk(Some(actual)) = client.cmd(&["GET", "k"]) else {
        panic!("missing value");
    };
    assert_eq!(actual.len(), value.len());
    assert!(actual == value);
}

#[test]
fn slow_reader_receives_all_replies() {
    let _server = Server::new();
    let mut client = Client::connect();
    let value = vec![b'x'; 128 * 1024];
    assert_eq!(client.bytes(&[b"SET", b"k", &value]), ok());
    let wire = encode(&[b"GET", b"k"]).repeat(512);
    client.stream.write_all(&wire).unwrap();
    thread::sleep(Duration::from_millis(500));
    for index in 0..512 {
        let reply = client
            .read()
            .unwrap_or_else(|error| panic!("reply {}/512: {error}", index + 1));
        let Reply::Bulk(Some(actual)) = reply else {
            panic!("missing value");
        };
        assert_eq!(actual.len(), value.len());
        assert!(actual == value);
    }
}

#[test]
fn half_closed_client_receives_reply() {
    let _server = Server::new();
    let mut client = Client::connect();
    client.stream.write_all(&encode(&[b"PING"])).unwrap();
    client.stream.shutdown(Shutdown::Write).unwrap();
    assert_eq!(client.read().unwrap(), Reply::Simple(b"PONG".to_vec()));
}

#[test]
fn aof_restores_set_incr_and_del() {
    let mut server = Server::new();
    let mut client = Client::connect();
    assert_eq!(
        client.pipeline(&commands(&[
            &["SET", "n", "4"],
            &["INCR", "n"],
            &["SET", "gone", "v"],
            &["DEL", "gone"],
        ])),
        vec![ok(), Reply::Integer(5), ok(), Reply::Integer(1)]
    );
    thread::sleep(Duration::from_millis(150));
    drop(client);
    server.restart();
    let mut client = Client::connect();
    assert_eq!(client.cmd(&["GET", "n"]), bulk("5"));
    assert_eq!(client.cmd(&["GET", "gone"]), nil());
}

#[test]
fn aof_restores_set_with_expiry() {
    let mut server = Server::new();
    let mut client = Client::connect();
    assert_eq!(client.cmd(&["SET", "k", "v", "EX", "60"]), ok());
    thread::sleep(Duration::from_millis(150));
    drop(client);
    server.restart();
    let mut client = Client::connect();
    assert_eq!(client.cmd(&["GET", "k"]), bulk("v"));
    assert!(matches!(client.cmd(&["TTL", "k"]), Reply::Integer(ttl) if ttl > 0));
}

#[test]
fn aof_preserves_expire() {
    let mut server = Server::new();
    let mut client = Client::connect();
    assert_eq!(client.cmd(&["SET", "k", "v"]), ok());
    assert_eq!(client.cmd(&["EXPIRE", "k", "60"]), Reply::Integer(1));
    thread::sleep(Duration::from_millis(150));
    drop(client);
    server.restart();
    let mut client = Client::connect();
    assert_eq!(client.cmd(&["GET", "k"]), bulk("v"));
    let ttl = client.cmd(&["TTL", "k"]);
    assert!(
        matches!(ttl, Reply::Integer(n) if n > 0),
        "TTL after restart: {ttl:?}"
    );
}

#[test]
fn aof_does_not_resurrect_expired_counter() {
    let mut server = Server::new();
    let mut client = Client::connect();
    assert_eq!(client.cmd(&["SET", "n", "100"]), ok());
    assert_eq!(client.cmd(&["EXPIRE", "n", "0"]), Reply::Integer(1));
    assert_eq!(client.cmd(&["INCR", "n"]), Reply::Integer(1));
    thread::sleep(Duration::from_millis(150));
    drop(client);
    server.restart();
    assert_eq!(Client::connect().cmd(&["GET", "n"]), bulk("1"));
}

#[test]
fn truncated_aof_tail_does_not_hide_new_writes() {
    let mut server = Server::new();
    server.stop();
    let mut log = encode(&[b"SET", b"old", b"v"]);
    log.extend_from_slice(b"*3\r\n$3\r\nSET\r\n$1\r\nz\r\n$5\r\nxx");
    fs::write(server.directory.join("appendonly.aof"), log).unwrap();
    server.start();
    let mut client = Client::connect();
    assert_eq!(client.cmd(&["GET", "old"]), bulk("v"));
    assert_eq!(client.cmd(&["SET", "new", "value"]), ok());
    thread::sleep(Duration::from_millis(150));
    drop(client);
    server.restart();
    assert_eq!(Client::connect().cmd(&["GET", "new"]), bulk("value"));
}

#[test]
fn eviction_preserves_survivors_on_restart() {
    let mut server = Server::new();
    let mut client = Client::connect();
    let sets: Vec<_> = (0..101)
        .map(|i| vec!["SET".into(), format!("k{i}"), "v".into()])
        .collect();
    assert!(client.pipeline(&sets).iter().all(|reply| *reply == ok()));
    let gets: Vec<_> = (0..101)
        .map(|i| vec!["GET".into(), format!("k{i}")])
        .collect();
    let before = client.pipeline(&gets);
    assert_eq!(before.iter().filter(|reply| **reply != nil()).count(), 61);
    thread::sleep(Duration::from_millis(150));
    drop(client);
    server.restart();
    assert_eq!(Client::connect().pipeline(&gets), before);
}

#[test]
fn binary_data_survives_restart() {
    let mut server = Server::new();
    let key = b"key\xff\0\r\n";
    let value = b"value\xfe\0\r\n";
    assert_eq!(Client::connect().bytes(&[b"SET", key, value]), ok());
    server.restart();
    assert_eq!(
        Client::connect().bytes(&[b"GET", key]),
        Reply::Bulk(Some(value.to_vec()))
    );
}

#[test]
fn expiry_elapses_while_server_is_stopped() {
    let mut server = Server::new();
    assert_eq!(Client::connect().cmd(&["SET", "k", "v", "PX", "600"]), ok());
    server.stop();
    thread::sleep(Duration::from_millis(800));
    server.start();
    assert_eq!(Client::connect().cmd(&["GET", "k"]), nil());
}

#[test]
fn rejected_overwrite_does_not_change_persisted_value() {
    let mut server = Server::new();
    let mut client = Client::connect();
    assert_eq!(client.cmd(&["SET", "k", "original"]), ok());
    assert!(matches!(
        client.cmd(&["SET", "k", "bad", "EX", "0"]),
        Reply::Error(_)
    ));
    drop(client);
    server.restart();
    assert_eq!(Client::connect().cmd(&["GET", "k"]), bulk("original"));
}
