mod resp;
pub mod helpers;
mod client;
mod cmd;
mod sync_tcp;
mod commands;
mod events;
mod eviction;

use crate::{events::run_event_loop};


fn main() -> std::io::Result<()> {
    println!("Hello world");
    run_event_loop() 
}
