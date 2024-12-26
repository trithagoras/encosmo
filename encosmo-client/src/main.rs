use std::{error::Error, io::{self, Read, Write}, net::TcpStream, sync::mpsc, thread};

use macroquad::prelude::*;
use miniquad::window::order_quit;
use packets::Packet;

mod packets;

#[macroquad::main("MyGame")]
async fn main() -> Result<(), Box<dyn Error>> {
    let (net_tx, net_rx) = mpsc::channel::<Packet>();
    let mut stream = TcpStream::connect("127.0.0.1:42523")?;

    let p = Packet::Login("John".to_owned());
    let s = serde_json::to_string(&p)?;
    let clone = stream.try_clone()?;

    thread::spawn(move || {
        let res = recv_loop(clone, net_tx);
        match res {
            Err (e) => {
                eprintln!("Error in recv_loop: {}", e);
                order_quit();
            },
            _ => {}
        }
    });
    
    loop {
        // get any available packet
        if let Ok (p) = (&net_rx).try_recv() {
            println!("Packet received from server! {:?}", p);

            if let Packet::Id(id) = p {
                println!("Our ID has been set to {}", id);
                println!("Sending login packet: {:?} ...", s);
                stream.write(s.as_bytes())?;
            }
        }

        // draw
        clear_background(RED);

        draw_line(40.0, 40.0, 100.0, 200.0, 15.0, BLUE);
        draw_rectangle(screen_width() / 2.0 - 60.0, 100.0, 120.0, 60.0, GREEN);

        draw_text("Hello, Macroquad!", 20.0, 20.0, 30.0, DARKGRAY);

        next_frame().await
    }
}

fn recv_loop(mut stream: TcpStream, net_tx: mpsc::Sender<Packet>) -> Result<(), Box<dyn Error>> {
    loop {
        let mut buf = [0u8; 1024];
        let read = stream.read(&mut buf)?;
        if read == 0 {
            return Err (Box::new(io::Error::new(io::ErrorKind::BrokenPipe, "No more bytes")));
        }

        let v = buf[0..read].to_vec();
        println!("{:?}", v);
        let s = String::from_utf8(v)?;
        println!("{}", s);
        let packet = serde_json::from_str(&s)?;
        net_tx.send(packet)?;
    }
}