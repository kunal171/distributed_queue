mod message;
mod broker;
mod protocol;
mod producer;
mod consumer;
use std::env;

#[tokio::main]
async fn main() {
   let args: Vec<String> = env::args().collect();

   let role = args.get(1).map(|s| s.as_str()).unwrap_or("broker");
   let addr = "127.0.0.1:8080";

    match role {
        "broker" => broker::run_broker(addr).await,
        "producer" => {
            let messages: Vec<String> = (0..5).map(|i| format!("task-{}", i)).collect();
            producer::run_producer(addr, messages).await;
        }
        "consumer" => consumer::run_consumer(addr).await,
        _ => eprintln!("Usage: distributed_queue [broker|producer|consumer]"),
    }
}
