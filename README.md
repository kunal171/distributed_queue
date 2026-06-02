# Distributed Queue

A message broker built in Rust with TCP networking, supporting multiple producers
and consumers with at-least-once delivery guarantees.

## What It Does

- Accepts messages from producers over TCP
- Stores messages in an in-memory queue
- Dispatches messages to consumers with round-robin distribution
- Tracks acknowledgments and requeues unacknowledged messages after timeout
- Handles consumer failures gracefully with timeout-based requeue
- Supports graceful shutdown with in-flight message draining (Ctrl+C)

## Architecture

```text
Producer(s) ──TCP──→ Broker ──TCP──→ Consumer(s)
                       │
                 In-memory queue
                 + round-robin dispatch
                 + ack tracking
                 + timeout requeue
                 + graceful shutdown
```

## Project Structure

```text
src/
├── main.rs       — CLI dispatch: broker, producer, or consumer
├── lib.rs        — Public module re-exports for integration tests
├── broker.rs     — Broker struct, dispatch loop, sweep task, connection handling
├── producer.rs   — TCP client that registers and publishes messages
├── consumer.rs   — TCP client that registers, receives messages, and sends ACKs
├── message.rs    — Message, ClientMessage, ServerMessage types
└── protocol.rs   — Length-prefixed framing (read_frame, write_frame)
tests/
└── integration.rs — 5 integration tests covering the full message flow
```

## Wire Protocol

Length-prefixed JSON frames over TCP: `[4-byte big-endian length][JSON payload]`.

Client-to-broker messages (`ClientMessage`):
- `Register { role }` — first frame, identifies client as producer or consumer
- `Publish { payload }` — producer enqueues a message
- `Ack { id }` — consumer confirms processing of a message

Broker-to-client messages (`ServerMessage`):
- `Message { id, payload }` — delivers a message to a consumer
- `Ok` — success response (e.g. after publish)
- `Error { message }` — error response

## Delivery Guarantees

**At-least-once delivery**: every message is delivered at least once.

1. Broker sends a message to a consumer and moves it to an in-flight set
2. Consumer sends an ACK after processing — broker removes from in-flight
3. If no ACK arrives within 5 seconds, the sweep task requeues the message
4. Requeued messages go to the front of the queue for priority retry

## Usage

```bash
# Terminal 1: start broker
cargo run -- broker

# Terminal 2: start consumer (connect before producer to see messages)
cargo run -- consumer

# Terminal 3: send messages
cargo run -- producer
```

Multiple consumers can connect simultaneously — messages are distributed via round-robin.

## Tests

```bash
cargo test
```

5 integration tests:
- `test_publish_and_consume` — basic end-to-end delivery and ACK
- `test_producer_gets_ok` — producer receives Ok for each publish
- `test_round_robin_two_consumers` — 4 messages split across 2 consumers
- `test_messages_queued_before_consumer` — messages wait in queue until a consumer connects
- `test_multiple_producers_single_consumer` — 2 producers, 1 consumer, all messages delivered

## Dependencies

```toml
[dependencies]
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

## Commands

```bash
cargo check
cargo test
cargo fmt --check
cargo clippy
```
