mod message;
mod broker;
mod protocol;

use std::sync::Arc;
use tokio::sync::Mutex;
use broker::Broker;

#[tokio::main]
async fn main() {
    // Create a shared broker instance
    let broker = Arc::new(Mutex::new(Broker::new()));

    // Spawn a producer task
    let producer_broker = broker.clone();
    // The producer will publish messages to the broker
    let producer = tokio::spawn(async move {
        for i in 0..5 {
            let mut b = producer_broker.lock().await;
            b.publish(format!("task-{}", i));
            drop(b);
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    });

    // Spawn a consumer task
    let consumer_broker = broker.clone();
    let consumer = tokio::spawn(async move {
        loop {
            let mut b = consumer_broker.lock().await;
            if let Some(msg) = b.consume() {
                println!("[consumer] got message {}: {}", msg.id, msg.payload);
                drop(b);
            } else {
                drop(b);
                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
            }
        }
    });

    // Wait for the producer to finish
    producer.await.unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    consumer.abort();
    println!("[main] done");
}
