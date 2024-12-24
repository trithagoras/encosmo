use server::Server;
use std::io;

mod messages;
mod packets;
mod server;
mod connection;
mod utils;

#[tokio::main]
async fn main() -> io::Result<()> {
    let mut server = Server::new();
    server.start(42523).await
}
