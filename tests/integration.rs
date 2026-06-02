use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tokio::time::{timeout, Duration};

use distributed_queue::broker::run_broker_on;
use distributed_queue::message::{ClientMessage, ServerMessage};
use distributed_queue::protocol::{read_frame, write_frame};

// --- Test helpers ---

/// Starts a broker on a random port and returns the bound address.
async fn start_broker() -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(run_broker_on(listener));
    addr
}

/// Connects to the broker and registers with the given role.
async fn connect_as(addr: SocketAddr, role: &str) -> TcpStream {
    let mut stream = TcpStream::connect(addr).await.unwrap();
    let register = serde_json::to_vec(&ClientMessage::Register {
        role: role.to_string(),
    })
    .unwrap();
    write_frame(&mut stream, &register).await.unwrap();
    stream
}

/// Publishes a message and asserts the broker responds with Ok.
async fn publish_one(stream: &mut TcpStream, payload: &str) {
    let msg = serde_json::to_vec(&ClientMessage::Publish {
        payload: payload.to_string(),
    })
    .unwrap();
    write_frame(stream, &msg).await.unwrap();

    let frame = read_frame(stream).await.unwrap().expect("expected Ok response");
    let resp: ServerMessage = serde_json::from_slice(&frame).unwrap();
    assert!(matches!(resp, ServerMessage::Ok));
}

/// Receives one message from the broker (with a 5s timeout) and sends an ACK.
async fn receive_and_ack(stream: &mut TcpStream) -> (u64, String) {
    let frame = timeout(Duration::from_secs(5), read_frame(stream))
        .await
        .expect("timed out waiting for message")
        .unwrap()
        .expect("connection closed");

    match serde_json::from_slice::<ServerMessage>(&frame).unwrap() {
        ServerMessage::Message { id, payload } => {
            let ack = serde_json::to_vec(&ClientMessage::Ack { id }).unwrap();
            write_frame(stream, &ack).await.unwrap();
            (id, payload)
        }
        other => panic!("expected Message, got {:?}", other),
    }
}

// --- Tests ---

/// Basic end-to-end: producer publishes 3 messages, consumer receives and ACKs all 3.
#[tokio::test]
async fn test_publish_and_consume() {
    let addr = start_broker().await;

    let mut consumer = connect_as(addr, "consumer").await;
    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut producer = connect_as(addr, "producer").await;
    for i in 0..3 {
        publish_one(&mut producer, &format!("msg-{}", i)).await;
    }

    let mut received = Vec::new();
    for _ in 0..3 {
        let (_, payload) = receive_and_ack(&mut consumer).await;
        received.push(payload);
    }

    assert_eq!(received, vec!["msg-0", "msg-1", "msg-2"]);
}

/// Producer gets an Ok response for each published message.
#[tokio::test]
async fn test_producer_gets_ok() {
    let addr = start_broker().await;
    let mut producer = connect_as(addr, "producer").await;

    // publish_one asserts Ok internally
    publish_one(&mut producer, "payload-a").await;
    publish_one(&mut producer, "payload-b").await;
    publish_one(&mut producer, "payload-c").await;
}

/// Two consumers receive messages via round-robin dispatch.
#[tokio::test]
async fn test_round_robin_two_consumers() {
    let addr = start_broker().await;

    let mut consumer1 = connect_as(addr, "consumer").await;
    let mut consumer2 = connect_as(addr, "consumer").await;
    tokio::time::sleep(Duration::from_millis(200)).await;

    let mut producer = connect_as(addr, "producer").await;
    for i in 0..4 {
        publish_one(&mut producer, &format!("task-{}", i)).await;
    }

    // Spawn receivers concurrently since each consumer blocks on read
    let c1 = tokio::spawn(async move {
        let mut msgs = Vec::new();
        for _ in 0..2 {
            let (_, payload) = receive_and_ack(&mut consumer1).await;
            msgs.push(payload);
        }
        msgs
    });

    let c2 = tokio::spawn(async move {
        let mut msgs = Vec::new();
        for _ in 0..2 {
            let (_, payload) = receive_and_ack(&mut consumer2).await;
            msgs.push(payload);
        }
        msgs
    });

    let c1_msgs = c1.await.unwrap();
    let c2_msgs = c2.await.unwrap();

    // Both consumers should have received messages
    assert_eq!(c1_msgs.len(), 2);
    assert_eq!(c2_msgs.len(), 2);

    // All 4 messages should be accounted for
    let mut all: Vec<String> = c1_msgs.into_iter().chain(c2_msgs).collect();
    all.sort();
    assert_eq!(all, vec!["task-0", "task-1", "task-2", "task-3"]);
}

/// Messages published before any consumer connects are still delivered.
#[tokio::test]
async fn test_messages_queued_before_consumer() {
    let addr = start_broker().await;

    let mut producer = connect_as(addr, "producer").await;
    for i in 0..3 {
        publish_one(&mut producer, &format!("early-{}", i)).await;
    }

    // Messages are sitting in the queue — now connect a consumer
    tokio::time::sleep(Duration::from_millis(100)).await;
    let mut consumer = connect_as(addr, "consumer").await;

    let mut received = Vec::new();
    for _ in 0..3 {
        let (_, payload) = receive_and_ack(&mut consumer).await;
        received.push(payload);
    }

    assert_eq!(received, vec!["early-0", "early-1", "early-2"]);
}

/// Multiple producers can publish to the same broker, single consumer gets all messages.
#[tokio::test]
async fn test_multiple_producers_single_consumer() {
    let addr = start_broker().await;

    let mut consumer = connect_as(addr, "consumer").await;
    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut producer1 = connect_as(addr, "producer").await;
    let mut producer2 = connect_as(addr, "producer").await;

    publish_one(&mut producer1, "from-p1-a").await;
    publish_one(&mut producer2, "from-p2-a").await;
    publish_one(&mut producer1, "from-p1-b").await;
    publish_one(&mut producer2, "from-p2-b").await;

    let mut received = Vec::new();
    for _ in 0..4 {
        let (_, payload) = receive_and_ack(&mut consumer).await;
        received.push(payload);
    }

    assert_eq!(received.len(), 4);
    assert!(received.contains(&"from-p1-a".to_string()));
    assert!(received.contains(&"from-p2-a".to_string()));
    assert!(received.contains(&"from-p1-b".to_string()));
    assert!(received.contains(&"from-p2-b".to_string()));
}
