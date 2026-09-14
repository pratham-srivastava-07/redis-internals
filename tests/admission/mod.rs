use crate::support::{Client, Reply, Server, bulk};
use std::fs;

fn stored() -> Reply {
    Reply::Simple(b"STORED".to_vec())
}

fn stats(client: &mut Client) -> String {
    let Reply::Bulk(Some(body)) = client.cmd(&["CACHE.STATS"]) else {
        panic!("expected cache stats as a bulk reply");
    };
    String::from_utf8(body).unwrap()
}

fn warm(client: &mut Client) {
    let mut commands = Vec::new();
    for i in 0..100 {
        let key = format!("hot:{i}");
        commands.push(vec!["CACHE.PUT".into(), key.clone(), "v".into()]);
        for _ in 0..5 {
            commands.push(vec!["GET".into(), key.clone()]);
        }
    }
    for chunk in client.pipeline(&commands).chunks(6) {
        assert_eq!(chunk[0], stored());
        assert!(chunk[1..].iter().all(|reply| *reply == bulk("v")));
    }
}

#[test]
fn admission_rejects_scan_and_persists_one_replacement() {
    let mut server = Server::new();
    let mut client = Client::connect();
    warm(&mut client);
    let path = server.directory.join("appendonly.aof");
    let before_log = fs::read(&path).unwrap();
    let mut scan = Vec::new();
    for i in 0..100 {
        scan.push(vec!["GET".into(), format!("scan:{i}")]);
        scan.push(vec!["CACHE.PUT".into(), format!("scan:{i}"), "v".into()]);
    }
    for pair in client.pipeline(&scan).chunks(2) {
        assert_eq!(pair[0], Reply::Bulk(None));
        assert_eq!(pair[1], Reply::Simple(b"REJECTED".to_vec()));
    }
    assert_eq!(fs::read(&path).unwrap(), before_log);
    let report = stats(&mut client);
    assert!(report.contains(
        "hits:500\r\nmisses:100\r\nadmitted:100\r\nrejected:100\r\nadmission_evictions:0\r\n"
    ));

    for _ in 0..20 {
        assert_eq!(client.cmd(&["GET", "new"]), Reply::Bulk(None));
    }
    assert_eq!(
        client.cmd(&["CACHE.PUT", "new", "value", "EX", "60"]),
        stored()
    );
    let gets: Vec<_> = (0..100)
        .map(|i| vec!["GET".into(), format!("hot:{i}")])
        .collect();
    let survivors = client.pipeline(&gets);
    assert_eq!(
        survivors
            .iter()
            .filter(|reply| **reply == bulk("v"))
            .count(),
        99
    );
    assert!(stats(&mut client).contains("admission_evictions:1\r\n"));
    drop(client);
    server.restart();
    let mut client = Client::connect();
    assert_eq!(client.pipeline(&gets), survivors);
    assert_eq!(client.cmd(&["GET", "new"]), bulk("value"));
    assert!(matches!(client.cmd(&["TTL", "new"]), Reply::Integer(n) if n > 0));
    assert!(
        stats(&mut client).contains(
            "admitted:0\r\nrejected:0\r\nadmission_evictions:0\r\nfrequency_resets:0\r\n"
        )
    );
    server.assert_alive();
}

#[test]
fn admission_validates_and_preserves_binary_values_and_set_behavior() {
    let mut server = Server::new();
    let mut client = Client::connect();
    let key = b"\xff\0key";
    let value = b"\xfe\0\r\nvalue";
    assert_eq!(client.bytes(&[b"cache.put", key, value]), stored());
    assert!(matches!(
        client.bytes(&[b"CACHE.PUT", key, b"bad", b"EX", b"0"]),
        Reply::Error(_)
    ));
    assert!(matches!(client.cmd(&["CACHE.PUT"]), Reply::Error(_)));
    assert!(matches!(
        client.cmd(&["CACHE.STATS", "extra"]),
        Reply::Error(_)
    ));
    assert!(matches!(
        client.cmd(&["GET", "missing", "extra"]),
        Reply::Error(_)
    ));
    assert!(stats(&mut client).contains("hits:0\r\nmisses:0\r\nadmitted:1\r\nrejected:0\r\n"));
    drop(client);
    server.restart();
    let mut client = Client::connect();
    assert_eq!(
        client.bytes(&[b"GET", key]),
        Reply::Bulk(Some(value.to_vec()))
    );
    assert_eq!(
        client.cmd(&["SET", "regular", "v"]),
        Reply::Simple(b"OK".to_vec())
    );
    assert_eq!(client.cmd(&["GET", "regular"]), bulk("v"));
}

#[test]
fn direct_fill_ties_reject_but_overwrites_are_allowed() {
    let _server = Server::new();
    let mut client = Client::connect();
    for i in 0..100 {
        assert_eq!(client.cmd(&["CACHE.PUT", &format!("k{i}"), "v"]), stored());
    }
    assert_eq!(
        client.cmd(&["CACHE.PUT", "new", "v"]),
        Reply::Simple(b"REJECTED".to_vec())
    );
    assert_eq!(client.cmd(&["CACHE.PUT", "k0", "replacement"]), stored());
    assert_eq!(client.cmd(&["GET", "k0"]), bulk("replacement"));
    assert_eq!(
        client.cmd(&["SET", "new", "forced"]),
        Reply::Simple(b"OK".to_vec())
    );
    let Reply::Bulk(Some(info)) = client.cmd(&["INFO"]) else {
        panic!("expected INFO");
    };
    assert!(String::from_utf8(info).unwrap().contains("keys=61,"));
}
