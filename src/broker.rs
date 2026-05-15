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

pub async fn run_broker(addr: &str) {
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

    // Handle the initial registration message to determine if this is a producer or consumer
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

async fn handle_producer(mut stream: TcpStream, broker: Arc<Mutex<Broker>>) {
    while let Ok(Some(frame)) = read_frame(&mut stream).await {
        if let Ok(ClientMessage::Publish { payload }) = serde_json::from_slice(&frame) {
            let mut b = broker.lock().await;
            b.publish(payload);
            drop(b);

            let resp = serde_json::to_vec(&ServerMessage::Ok).unwrap();
            if write_frame(&mut stream, &resp).await.is_err() {
                break;
            }
        }
    }
    println!("[broker] producer disconnected");
}

async fn handle_consumer(mut stream: TcpStream, broker: Arc<Mutex<Broker>>) {
     loop {
        let msg = {
            let mut b = broker.lock().await;
            b.consume()
        };

        if let Some(msg) = msg {
            let resp = serde_json::to_vec(&ServerMessage::Message {
                id: msg.id,
                payload: msg.payload,
            }).unwrap();
            if write_frame(&mut stream, &resp).await.is_err() {
                break;
            }
        } else {
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }
    }
}