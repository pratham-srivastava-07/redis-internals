mod resp;
pub mod helpers;
mod client;
mod cmd;
mod sync_tcp;
mod commands;
mod events;


use crate::{events::run_event_loop};


fn main() -> std::io::Result<()> {
    let args: Vec<String> = vec!["SET".to_string(), "k".to_string(), "v".to_string()];
    run_event_loop()
}