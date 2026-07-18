use super::*;                       
use std::collections::HashMap;
use crate::cmd::RedisValue;

#[test]
fn ping_replies_pong() {
    let mut out: Vec<u8> = Vec::new();
    eval_ping(vec![], &mut out).unwrap();
    assert_eq!(out, b"+PONG\r\n");
}