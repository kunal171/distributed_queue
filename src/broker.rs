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
}