# Distributed Queue

A message broker built in Rust with TCP networking, supporting multiple producers
and consumers with at-least-once delivery guarantees.

## What It Will Do

- Accept messages from producers over TCP
- Store messages in an in-memory queue
- Dispatch messages to consumers with round-robin distribution
- Track acknowledgments and requeue unacknowledged messages
- Handle consumer failures gracefully with timeouts
- Support graceful shutdown with in-flight message draining

## Architecture

```text
Producer(s) ──TCP──→ Broker ──TCP──→ Consumer(s)
                       │
                 In-memory queue
                 + ack tracking
                 + timeout requeue
```

## Current State

Milestones 1–2 complete. Milestone 3 in progress (ACK types and in-flight tracking done, wiring pending).

Remaining: Milestone 3 steps 3–6 (split stream, consumer ACKs, sweep task), Milestone 4 (multiple consumers, graceful shutdown, tests).

## Project Structure

```text
src/
├── main.rs       — CLI dispatch: broker, producer, or consumer
├── broker.rs     — Broker struct, TCP listener, connection handling
├── producer.rs   — TCP client that registers and publishes messages
├── consumer.rs   — TCP client that registers and receives messages
├── message.rs    — Message, ClientMessage, ServerMessage types
└── protocol.rs   — Length-prefixed framing (read_frame, write_frame)
```

## Implemented So Far

- `Message` struct with id, payload, timestamp
- `ClientMessage` enum: Register, Publish (sent by clients to broker)
- `ServerMessage` enum: Message, Ok (sent by broker to clients)
- Length-prefixed JSON wire protocol (`protocol.rs`)
- Broker with in-memory `VecDeque<Message>` queue
- TCP listener with per-connection `tokio::spawn`
- Registration handshake: first frame identifies client as producer or consumer
- Producer: connects, registers, publishes messages, waits for Ok
- Consumer: connects, registers, receives messages in a loop
- Single binary with CLI args: `cargo run -- broker|producer|consumer`

## Milestone Plan

### Milestone 1: In-Memory Queue + Local Producer/Consumer — done

Message struct, in-memory queue, producer and consumer as async tasks in one process.

### Milestone 2: TCP Networking — done

Broker listens on TCP. Producers and consumers connect as separate processes. JSON wire protocol.

### Milestone 3: Acknowledgments and Retry

Consumer ACKs, broker tracks in-flight messages, timeout-based requeue, at-least-once delivery.

### Milestone 4: Multiple Consumers and Polish

Round-robin dispatch, graceful shutdown, integration tests.

## Concepts Practiced

- TCP networking with Tokio (`TcpListener`, `TcpStream`)
- Wire protocols (length-prefixed JSON framing)
- `tokio::sync::Mutex` for async-safe shared state
- `Arc` for sharing broker across spawned tasks
- `#[serde(tag = "type")]` for internally-tagged JSON enums
- Per-connection task spawning with `tokio::spawn`
- CLI arg dispatch for multi-role binary

## Usage

```bash
# Terminal 1: start broker
cargo run -- broker

# Terminal 2: start consumer
cargo run -- consumer

# Terminal 3: send messages
cargo run -- producer
```

## Useful Commands

```bash
cargo check
cargo test
cargo fmt --check
cargo clippy
```
