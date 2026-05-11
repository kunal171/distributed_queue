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

Project just initialized. Planning milestones.

## Milestone Plan

### Milestone 1: In-Memory Queue + Local Producer/Consumer

Message struct, in-memory queue, producer and consumer as async tasks in one process.

### Milestone 2: TCP Networking

Broker listens on TCP. Producers and consumers connect as separate processes. JSON wire protocol.

### Milestone 3: Acknowledgments and Retry

Consumer ACKs, broker tracks in-flight messages, timeout-based requeue, at-least-once delivery.

### Milestone 4: Multiple Consumers and Polish

Round-robin dispatch, graceful shutdown, integration tests.

## Planned Concepts

- TCP networking with Tokio (`TcpListener`, `TcpStream`)
- Wire protocols (framing, JSON serialization over TCP)
- Fault tolerance (ack/nack, retry, timeouts)
- At-least-once delivery semantics
- Graceful shutdown (`tokio::signal`, draining in-flight work)
- Multi-client connection handling
- CAP theorem trade-offs in practice

## Useful Commands

```bash
cargo run
cargo check
cargo test
cargo fmt --check
cargo clippy
```
