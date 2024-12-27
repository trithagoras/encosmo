use server::Server;
use std::{env, io};
use anyhow::Result;

mod messages;
mod packets;
mod states;
mod server;
mod connection;

#[tokio::main]
async fn main() -> Result<()> {
    simple_logger::SimpleLogger::new().env().init().unwrap();

    // get port from command line:
    let port: u16 = match env::args().nth(1) {
        None => 42523,
        Some (p) => p.parse()?
    };

    let mut server = Server::new();
    server.start(port).await
}
