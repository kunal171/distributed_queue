//! Consumer client — connects to the broker, receives messages, and sends ACKs.
//!
//! After registering, the consumer enters a receive loop. For each message it
//! processes, it sends an ACK back to the broker. If the consumer crashes before
//! ACKing, the broker's sweep task will requeue the message for redelivery.

use tokio::net::TcpStream;
use crate::message::{ClientMessage, ServerMessage};
use crate::protocol::{read_frame, write_frame};

/// Connects to the broker at `addr`, registers as a consumer, and processes
/// messages in a loop — sending an ACK for each one.
pub async fn run_consumer(addr: &str) {
    let mut stream = TcpStream::connect(addr).await.expect("failed to connect");
    println!("[consumer] connected to {}", addr);

    // First frame must be a Register message so the broker knows our role
    let register = serde_json::to_vec(&ClientMessage::Register {
        role: "consumer".to_string(),
    }).unwrap();
    write_frame(&mut stream, &register).await.unwrap();

    while let Ok(Some(frame)) = read_frame(&mut stream).await {
        if let Ok(ServerMessage::Message { id, payload }) = serde_json::from_slice(&frame) {
            println!("[consumer] received message {}: {}", id, payload);

            // Send ACK back to the broker so it removes this message from in-flight
            let ack = serde_json::to_vec(&ClientMessage::Ack { id }).unwrap();
            write_frame(&mut stream, &ack).await.unwrap();
            println!("[consumer] acknowledged message {}", id);
        }
    }

    println!("[consumer] disconnected");
}
