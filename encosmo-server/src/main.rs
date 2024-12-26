use server::Server;
use std::io;

mod messages;
mod packets;
mod states;
mod server;
mod connection;

#[tokio::main]
async fn main() -> io::Result<()> {
    simple_logger::SimpleLogger::new().env().init().unwrap();
    let mut server = Server::new();
    server.start(42523).await
}
