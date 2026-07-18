mod resp;
pub mod helpers;
mod client;
mod cmd;
mod sync_tcp;
mod commands;
mod events;


use crate::{events::run_event_loop};


fn main() -> std::io::Result<()> {
    run_event_loop() 
}