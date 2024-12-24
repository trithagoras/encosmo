use tokio::sync::mpsc;
use crate::messages::Message;

pub type Channel = (mpsc::Sender<Message>, mpsc::Receiver<Message>);