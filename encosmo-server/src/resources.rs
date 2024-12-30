
use std::sync::mpsc;

use crate::messages::Message;


pub struct ServerTx(pub mpsc::Sender<Message>);
