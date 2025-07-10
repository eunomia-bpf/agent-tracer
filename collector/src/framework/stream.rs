use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::Stream;
use std::pin::Pin;
use crate::framework::types::ObservabilityEvent;

pub type EventStream = Pin<Box<dyn Stream<Item = ObservabilityEvent> + Send>>;
pub type EventSender = mpsc::UnboundedSender<ObservabilityEvent>;
pub type EventReceiver = mpsc::UnboundedReceiver<ObservabilityEvent>;

pub fn create_event_channel() -> (EventSender, EventReceiver) {
    mpsc::unbounded_channel()
}

pub fn receiver_to_stream(receiver: EventReceiver) -> EventStream {
    Box::pin(ReceiverStream::new(receiver))
}

#[derive(Clone)]
pub struct EventBroadcaster {
    senders: Vec<EventSender>,
}

impl EventBroadcaster {
    pub fn new() -> Self {
        Self {
            senders: Vec::new(),
        }
    }

    pub fn add_subscriber(&mut self) -> EventReceiver {
        let (tx, rx) = create_event_channel();
        self.senders.push(tx);
        rx
    }

    pub fn broadcast(&self, event: ObservabilityEvent) {
        for sender in &self.senders {
            let _ = sender.send(event.clone());
        }
    }

    pub fn subscriber_count(&self) -> usize {
        self.senders.len()
    }
}