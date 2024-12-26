use packets::Packet;
use server::Server;
use std::io;

mod messages;
mod packets;
mod server;
mod connection;

#[tokio::main]
async fn main() -> io::Result<()> {
    let p = Packet::Login("John".to_owned(), "123".to_owned());
    let s = serde_json::to_string(&p);
    println!("{}", s.unwrap());
    simple_logger::SimpleLogger::new().env().init().unwrap();
    let mut server = Server::new();
    server.start(42523).await
}
