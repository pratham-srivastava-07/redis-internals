use crate::cmd::{RedisCmd, Store};
use crate::commands::{self, Outcome};
use crate::stats::Stat;
use std::io::{self, Write};

pub fn respond<S: Write>(
    cmd: &RedisCmd,
    store: &mut Store,
    stats: &mut Stat,
    stream: &mut S,
) -> io::Result<Outcome> {
    match cmd.cmd.to_ascii_uppercase().as_str() {
        "PING" => commands::eval_ping(&cmd.args, stream),
        "SET" => commands::set_command(&cmd.args, store, stream),
        "GET" => commands::get_command(&cmd.args, store, stream),
        "INCR" => commands::incr_command(&cmd.args, store, stream),
        "OBJECT" => commands::object_command(&cmd.args, store, stream),
        "INFO" => commands::eval_info(&cmd.args, store, stats, stream),
        "TTL" => commands::set_ttl(&cmd.args, store, stream),
        "DEL" => commands::delete_keys(&cmd.args, store, stream),
        "EXPIRE" => commands::expire_command(&cmd.args, store, stream),
        _ => {
            stream.write_all(b"-ERR unknown command\r\n")?;
            Ok(Outcome::Rejected)
        }
    }
}
