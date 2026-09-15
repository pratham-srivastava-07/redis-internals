mod admission;
mod aof;
mod client;
mod cmd;
mod commands;
mod config;
mod events;
mod eviction;
pub mod helpers;
mod object;
mod pipeline;
mod resp;
mod stats;
mod sync_tcp;
mod types_encoding;

use crate::events::run_event_loop;

fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [] => {}
        [arg] if arg == "--version" || arg == "-V" => {
            println!("vynk {}", env!("CARGO_PKG_VERSION"));
            return Ok(());
        }
        [arg] if arg == "--help" || arg == "-h" => {
            println!(
                "Vynk - the Redis idea, rewritten in Rust\n\nUsage: vynk [--help | --version]\n\nListens on 127.0.0.1:7379. Stores appendonly.aof in the current directory.\nConnect with: redis-cli -p 7379"
            );
            return Ok(());
        }
        _ => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "unknown arguments; use vynk --help",
            ));
        }
    }
    println!("vynk {} | 127.0.0.1:7379", env!("CARGO_PKG_VERSION"));
    run_event_loop()
}
