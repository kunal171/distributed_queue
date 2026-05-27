//! Producer client — connects to the broker and publishes messages.
//!
//! Sends a registration frame to identify as a producer, then publishes
//! each message and waits for an Ok response before sending the next one.

use tokio::net::TcpStream;
use crate::message::ClientMessage;
use crate::protocol::{read_frame, write_frame};

/// Connects to the broker at `addr`, registers as a producer, and publishes
/// each message in `messages` sequentially.
pub async fn run_producer(addr: &str, messages: Vec<String>) {
    let mut stream = TcpStream::connect(addr).await.expect("failed to connect");
    println!("[producer] connected to {}", addr);

    // First frame must be a Register message so the broker knows our role
    let register = serde_json::to_vec(&ClientMessage::Register {
        role: "producer".to_string(),
    }).unwrap();
    write_frame(&mut stream, &register).await.unwrap();

    for payload in messages {
        let msg = serde_json::to_vec(&ClientMessage::Publish { payload: payload.clone() }).unwrap();
        write_frame(&mut stream, &msg).await.unwrap();

        // Wait for the broker's Ok response before sending the next message
        if let Ok(Some(_)) = read_frame(&mut stream).await {
            println!("[producer] sent: {}", payload);
        }
    }

    println!("[producer] done");
}
