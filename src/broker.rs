//! Broker — the central message queue server.
//!
//! Listens for TCP connections from producers and consumers. Producers push
//! messages into an in-memory queue; consumers receive them with at-least-once
//! delivery guaranteed by ACK tracking and timeout-based requeue.

use std::collections::{VecDeque, HashMap};
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};
use tokio::time::Instant;
use tokio::net::{TcpListener, TcpStream};
use crate::message::{Message, ClientMessage, ServerMessage};
use crate::protocol::{write_frame, read_frame};

/// In-memory message broker with at-least-once delivery.
///
/// Messages flow through two stages:
/// 1. `queue` — waiting to be sent to a consumer
/// 2. `in_flight` — sent but not yet acknowledged (tracked with a timestamp)
///
/// If an in-flight message isn't ACKed within the timeout, the sweep task
/// moves it back to the front of the queue for redelivery.
pub struct Broker {
    /// FIFO queue of messages waiting to be consumed.
    queue: VecDeque<Message>,
    /// Messages sent to consumers but not yet acknowledged.
    /// Key: message id, Value: (message, time it was sent).
    in_flight: HashMap<u64, (Message, Instant)>,
    /// Auto-incrementing counter for assigning unique message ids.
    next_id: u64,
    /// List of connected consumers (for future enhancements like push-based delivery).
    consumers: Vec<mpsc::Sender<Message>>,
    // Round-robin index for distributing messages to consumers (if we implement push-based delivery).
    next_consumer: usize,
    
}

impl Broker {
    pub fn new() -> Self {
        Broker {
            queue: VecDeque::new(),
            in_flight: HashMap::new(),
            next_id: 1,
            consumers: Vec::new(),
            next_consumer: 0,
        }
    }

    /// Enqueues a new message with the given payload. Returns the assigned id.
    pub fn publish(&mut self, payload: String) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let message = Message::new(id, payload);
        println!("[broker] queued message {}", id);
        self.queue.push_back(message);
        id
    }

    /// Removes a message from in-flight after the consumer confirms processing.
    pub fn ack(&mut self, id: u64) {
        if self.in_flight.remove(&id).is_some() {
            println!("[broker] acknowledged message {}", id);
        }
    }

    /// Moves any in-flight messages that have exceeded the timeout back to the queue.
    ///
    /// This is the core of at-least-once delivery: if a consumer dies before
    /// ACKing, the message will be redelivered to another consumer.
    pub fn requeue_expired(&mut self, timeout: std::time::Duration) {
        let now = Instant::now();
        let expired: Vec<u64> = self.in_flight
            .iter()
            .filter(|(_, (_, sent_at))| now.duration_since(*sent_at) > timeout)
            .map(|(id, _)| *id)
            .collect();

        for id in expired {
            if let Some((msg, _)) = self.in_flight.remove(&id) {
                println!("[broker] requeuing expired message {}", id);
                // Push to front so expired messages get retried before new ones
                self.queue.push_front(msg);
            }
        }
    }

    pub fn add_consumer(&mut self) -> mpsc::Receiver<Message> {
        let (tx, rx) = mpsc::channel(32);
        self.consumers.push(tx);
        rx
    }

    /// Returns the number of messages currently in-flight (sent but not yet ACKed).
    pub fn in_flight_count(&self) -> usize {
        self.in_flight.len()
    }

    pub fn remove_dead_consumer(&mut self) {
        self.consumers.retain(|tx| !tx.is_closed());
    }

    pub async fn dispatch_one(&mut self) -> bool {
        // If we have no consumers or no messages, we can't dispatch anything
        if self.consumers.is_empty() || self.queue.is_empty()  {
            return false;
        }

        // Clean up any dead consumers before trying to dispatch
        self.remove_dead_consumer();
        if self.consumers.is_empty() {
            return false;
        }

        let msg = match self.queue.pop_front() {
            Some(msg) => msg,
            None => return false,
        };

        let count = self.consumers.len();

        for _ in 0..count {
            let idx = self.next_consumer % self.consumers.len();
            self.next_consumer =  idx + 1;

            if self.consumers[idx].send(msg.clone()).await.is_ok() {
                // Track in-flight — same as before
                self.in_flight.insert(msg.id, (msg, Instant::now()));
                return true;
            }
        }

        // All consumers dead — put message back
        self.queue.push_front(msg);
        false
    }
}

/// Starts the broker: binds to `addr`, spawns a sweep task, and accepts connections.
pub async fn run_broker(addr: &str) {
    let listener = TcpListener::bind(addr).await.expect("failed to bind");
    run_broker_on(listener).await;
}

/// Runs the broker on an already-bound listener.
/// Useful for tests that bind to port 0 to get a random available port.
pub async fn run_broker_on(listener: TcpListener) {
    let addr = listener.local_addr().unwrap();
    println!("[broker] listening on {}", addr);

    let broker = Arc::new(Mutex::new(Broker::new()));

    // Sweep task — periodically checks for unacknowledged messages and requeues them
    let sweep_broker = broker.clone();
    tokio::spawn(async move {
        let timeout = std::time::Duration::from_secs(5);
        loop {
            tokio::time::sleep(timeout).await;
            let mut b = sweep_broker.lock().await;
            b.requeue_expired(timeout);
        }
    });

    // Dispatch task — pulls messages from the queue and round-robins them to consumers
    let dispatch_broker = broker.clone();
    tokio::spawn(async move {
        loop {
            let dispatched = {
                let mut b = dispatch_broker.lock().await;
                b.dispatch_one().await
            };

            if !dispatched {
                // No messages or no consumers — wait briefly before checking again
                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
            }
        }
    });

    // Accept loop — races against Ctrl+C shutdown signal
    let shutdown = tokio::signal::ctrl_c();
    tokio::pin!(shutdown);

    loop {
        tokio::select! {
            result = listener.accept() => {
                let (stream, peer) = result.expect("accept failed");
                println!("[broker] connection from {}", peer);

                let broker = broker.clone();
                tokio::spawn(async move {
                    handle_connection(stream, broker).await;
                });
            }
            _ = &mut shutdown => {
                println!("[broker] shutting down...");
                break;
            }
        }
    }

    // Drain: wait for in-flight messages to be ACKed before exiting
    let drain_timeout = std::time::Duration::from_secs(10);
    let start = Instant::now();
    loop {
        let remaining = {
            let b = broker.lock().await;
            b.in_flight_count()
        };

        if remaining == 0 {
            println!("[broker] all messages acknowledged, exiting");
            break;
        }

        if Instant::now().duration_since(start) > drain_timeout {
            println!("[broker] shutdown timeout, {} messages still in-flight", remaining);
            break;
        }

        println!("[broker] waiting for {} in-flight messages...", remaining);
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
}

/// Reads the first frame to determine if this client is a producer or consumer,
/// then delegates to the appropriate handler.
async fn handle_connection(mut stream: TcpStream, broker: Arc<Mutex<Broker>>) {
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

/// Handles a producer connection: reads Publish messages and enqueues them.
async fn handle_producer(mut stream: TcpStream, broker: Arc<Mutex<Broker>>) {
    while let Ok(Some(frame)) = read_frame(&mut stream).await {
        if let Ok(ClientMessage::Publish { payload }) = serde_json::from_slice(&frame) {
            let mut b = broker.lock().await;
            b.publish(payload);
            // Drop the lock before writing to the network to avoid holding it during I/O
            drop(b);

            let resp = serde_json::to_vec(&ServerMessage::Ok).unwrap();
            if write_frame(&mut stream, &resp).await.is_err() {
                break;
            }
        }
    }
    println!("[broker] producer disconnected");
}

/// Handles a consumer connection with bidirectional communication.
///
/// The consumer registers with the broker to get an `mpsc::Receiver<Message>`.
/// The broker's dispatch loop pushes messages into this channel via round-robin.
/// A spawned task listens for ACKs on the read half concurrently.
async fn handle_consumer(stream: TcpStream, broker: Arc<Mutex<Broker>>) {
    let (mut reader, mut writer) = tokio::io::split(stream);

    // Register this consumer with the broker and get our message channel
    let mut rx = {
        let mut b = broker.lock().await;
        b.add_consumer()
    };

    // Spawn a task to listen for ACKs from the consumer
    let ack_broker = broker.clone();
    let ack_task = tokio::spawn(async move {
        while let Ok(Some(frame)) = read_frame(&mut reader).await {
            if let Ok(ClientMessage::Ack { id }) = serde_json::from_slice(&frame) {
                let mut b = ack_broker.lock().await;
                b.ack(id);
            }
        }
    });

    // Receive messages from the broker's dispatch loop via the channel
    while let Some(msg) = rx.recv().await {
        let resp = serde_json::to_vec(&ServerMessage::Message {
            id: msg.id,
            payload: msg.payload,
        }).unwrap();
        if write_frame(&mut writer, &resp).await.is_err() {
            break;
        }

        if ack_task.is_finished() {
            break;
        }
    }
}