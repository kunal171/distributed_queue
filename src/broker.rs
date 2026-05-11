use std::collections::VecDeque;
use crate::message::Message;

pub struct Broker {
    queue: VecDeque<Message>,
    next_id: u64,

}

impl Broker {
    pub fn new() -> Self {
        Broker {
            queue: VecDeque::new(),
            next_id: 1,
        }
    }

    pub fn publish(&mut self, payload:String) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let message = Message::new(id, payload);
        println!("[broker] queued message {}", id);
        self.queue.push_back(message);
        id
    }

    pub fn consume(&mut self) -> Option<Message> {
        self.queue.pop_front()
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }
}