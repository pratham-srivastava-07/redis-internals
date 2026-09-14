use crate::cmd::RedisCmd;
use crate::helpers::utils::{DecodeError, MAX_FRAME_BYTES};
use crate::resp::decode_array_string;
use std::io::{ErrorKind, Read};

pub enum Fill {
    Ok,
    Disconnected,
    Invalid,
}

pub fn fill_box<S: Read>(conn: &mut S, inbox: &mut Vec<u8>) -> Fill {
    let mut buffer = [0; 4096];
    loop {
        match conn.read(&mut buffer) {
            Ok(0) => return Fill::Disconnected,
            Ok(n) => {
                if inbox.len() + n > MAX_FRAME_BYTES {
                    return Fill::Invalid;
                }
                inbox.extend_from_slice(&buffer[..n]);
            }
            Err(e) if e.kind() == ErrorKind::WouldBlock => return Fill::Ok,
            Err(e) if e.kind() == ErrorKind::Interrupted => continue,
            Err(_) => return Fill::Disconnected,
        }
    }
}

pub fn parse_commands(inbox: &mut Vec<u8>) -> Result<Vec<RedisCmd>, DecodeError> {
    let mut commands = Vec::new();
    let mut offset = 0;
    while offset < inbox.len() {
        match decode_array_string(&inbox[offset..]) {
            Ok((tokens, consumed)) => {
                if consumed > MAX_FRAME_BYTES {
                    return Err(DecodeError::Invalid);
                }
                commands.push(RedisCmd::from_tokens(tokens)?);
                offset += consumed;
            }
            Err(DecodeError::Incomplete) => break,
            Err(error) => return Err(error),
        }
    }
    inbox.drain(..offset);
    Ok(commands)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_can_be_split_at_every_byte() {
        let wire = crate::aof::encode_command("SET", &[b"key".to_vec(), b"\xff\0\r\n".to_vec()]);
        for split in 0..wire.len() {
            let mut inbox = wire[..split].to_vec();
            assert!(parse_commands(&mut inbox).unwrap().is_empty());
            inbox.extend_from_slice(&wire[split..]);
            let commands = parse_commands(&mut inbox).unwrap();
            assert_eq!(commands.len(), 1);
            assert_eq!(commands[0].args[1], b"\xff\0\r\n");
            assert!(inbox.is_empty());
        }
    }

    #[test]
    fn oversized_lengths_and_deep_arrays_are_rejected() {
        let mut oversized = b"*1\r\n$9223372036854775807\r\n".to_vec();
        assert!(matches!(
            parse_commands(&mut oversized),
            Err(DecodeError::Invalid)
        ));
        let mut nested = b"*1\r\n".repeat(40);
        nested.extend_from_slice(b"$4\r\nPING\r\n");
        assert!(matches!(
            parse_commands(&mut nested),
            Err(DecodeError::Invalid)
        ));
    }
}
