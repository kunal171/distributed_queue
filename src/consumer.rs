use tokio::net::TcpStream;
use crate::message::{ClientMessage, ServerMessage};
use crate::protocol::{read_frame, write_frame};

pub async fn run_consumer(addr: &str) {
    let mut stream = TcpStream::connect(addr).await.expect("failed to connect");
    println!("[consumer] connected to {}", addr);

    let register = serde_json::to_vec(&ClientMessage::Register {
        role: "consumer".to_string(),
    }).unwrap();
    write_frame(&mut stream, &register).await.unwrap();

    while let Ok(Some(frame)) = read_frame(&mut stream).await {
        if let Ok(ServerMessage::Message { id, payload }) = serde_json::from_slice(&frame) {
            println!("[consumer] received message {}: {}", id, payload);
        }
    }

    println!("[consumer] disconnected");
}
