use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::net::{ TcpListener, TcpStream};
use crate::broker;
use crate::message::{Message, ClientMessage, ServerMessage};
use crate::protocol::{write_frame, read_frame};

pub struct Broker {
    queue: VecDeque<Message>,
    next_id: u64,

}

impl Broker {
    pub fn new() -> Self {
        Broker {
            queue: VecDeque::new(),
            next_id: 1,
        }
    }

    pub fn publish(&mut self, payload:String) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let message = Message::new(id, payload);
        println!("[broker] queued message {}", id);
        self.queue.push_back(message);
        id
    }

    pub fn consume(&mut self) -> Option<Message> {
        self.queue.pop_front()
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }
}

async fn run_broker(addr: &str) {
    let listener = TcpListener::bind(addr).await.expect("failed to Bind");
    println!("[broker] listening on {}", addr);

    let broker = Arc::new(Mutex::new(Broker::new()));

    loop {
        let (stream, peer) = listener.accept().await.expect("accept failed");
        println!("[broker] connection from {}", peer);

        let broker = broker.clone();
        tokio::spawn(async move {
            handle_connection(stream, broker).await;
        });
    }
}

async fn handle_connection(mut stream: TcpStream, broker: Arc<Mutex<Broker>>) {
    // Implementation for handling individual connections
     let Some(first_frame) = read_frame(&mut stream).await.unwrap_or(None) else {
        return;
    };

    let Ok(msg) = serde_json::from_slice::<ClientMessage>(&first_frame) else {
        return;
    };

     match msg {
        ClientMessage::Register { role } if role == "producer" => {
            println!("[broker] producer registered");
            handle_producer(stream, broker).await;
        }
        ClientMessage::Register { role } if role == "consumer" => {
            println!("[broker] consumer registered");
            handle_consumer(stream, broker).await;
        }
        _ => {
            println!("[broker] unknown role, dropping connection");
        }
    }
}