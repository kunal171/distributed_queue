use tokio::net::TcpStream;
use crate::message::ClientMessage;
use crate::protocol::{read_frame, write_frame};

pub async fn run_producer(addr: &str, messages: Vec<String>) {
    let mut stream = TcpStream::connect(addr).await.expect("failed to connect");
    println!("[producer] connected to {}", addr);

    let register = serde_json::to_vec(&ClientMessage::Register {
        role: "producer".to_string(),
    }).unwrap();
    write_frame(&mut stream, &register).await.unwrap();

    for payload in messages {
        let msg = serde_json::to_vec(&ClientMessage::Publish { payload: payload.clone() }).unwrap();
        write_frame(&mut stream, &msg).await.unwrap();

        if let Ok(Some(_)) = read_frame(&mut stream).await {
            println!("[producer] sent: {}", payload);
        }
    }

    println!("[producer] done");
}
