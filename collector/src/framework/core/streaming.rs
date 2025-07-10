use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::{Stream, StreamExt};
use std::pin::Pin;
use super::events::ObservabilityEvent;
use super::error::StreamError;

pub type EventStream = Pin<Box<dyn Stream<Item = ObservabilityEvent> + Send>>;
pub type EventSender = mpsc::UnboundedSender<ObservabilityEvent>;
pub type EventReceiver = mpsc::UnboundedReceiver<ObservabilityEvent>;

pub fn create_event_channel() -> (EventSender, EventReceiver) {
    mpsc::unbounded_channel()
}

pub fn create_bounded_event_channel(capacity: usize) -> (mpsc::Sender<ObservabilityEvent>, mpsc::Receiver<ObservabilityEvent>) {
    mpsc::channel(capacity)
}

pub fn receiver_to_stream(receiver: EventReceiver) -> EventStream {
    Box::pin(ReceiverStream::new(receiver))
}

#[derive(Clone)]
pub struct EventBroadcaster {
    senders: Vec<EventSender>,
    subscriber_count: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}

impl EventBroadcaster {
    pub fn new() -> Self {
        Self {
            senders: Vec::new(),
            subscriber_count: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        }
    }

    pub fn add_subscriber(&mut self) -> EventReceiver {
        let (tx, rx) = create_event_channel();
        self.senders.push(tx);
        self.subscriber_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        rx
    }

    pub fn broadcast(&self, event: ObservabilityEvent) -> Result<(), StreamError> {
        let mut failed_senders = Vec::new();
        
        for (index, sender) in self.senders.iter().enumerate() {
            if let Err(_) = sender.send(event.clone()) {
                failed_senders.push(index);
            }
        }

        if !failed_senders.is_empty() {
            return Err(StreamError::BroadcastFailed {
                failed_count: failed_senders.len(),
                total_count: self.senders.len(),
            });
        }

        Ok(())
    }

    pub fn subscriber_count(&self) -> usize {
        self.subscriber_count.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn cleanup_closed_subscribers(&mut self) {
        self.senders.retain(|sender| !sender.is_closed());
        self.subscriber_count.store(self.senders.len(), std::sync::atomic::Ordering::Relaxed);
    }
}

impl Default for EventBroadcaster {
    fn default() -> Self {
        Self::new()
    }
}

/// Stream multiplexer for routing events to multiple analyzers
pub struct StreamMultiplexer {
    input_receiver: EventReceiver,
    broadcasters: Vec<EventBroadcaster>,
}

impl StreamMultiplexer {
    pub fn new(input_receiver: EventReceiver) -> Self {
        Self {
            input_receiver,
            broadcasters: Vec::new(),
        }
    }

    pub fn add_output(&mut self) -> EventReceiver {
        let mut broadcaster = EventBroadcaster::new();
        let receiver = broadcaster.add_subscriber();
        self.broadcasters.push(broadcaster);
        receiver
    }

    pub async fn run(mut self) -> Result<(), StreamError> {
        while let Some(event) = self.input_receiver.recv().await {
            for broadcaster in &self.broadcasters {
                broadcaster.broadcast(event.clone())?;
            }
        }
        Ok(())
    }
}

/// Filter stream that only passes events matching a predicate
pub struct FilteredStream<F> {
    inner: EventStream,
    filter: F,
}

impl<F> FilteredStream<F>
where
    F: Fn(&ObservabilityEvent) -> bool,
{
    pub fn new(inner: EventStream, filter: F) -> Self {
        Self { inner, filter }
    }
}

impl<F> Stream for FilteredStream<F>
where
    F: Fn(&ObservabilityEvent) -> bool + Unpin,
{
    type Item = ObservabilityEvent;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        loop {
            match self.inner.as_mut().poll_next(cx) {
                std::task::Poll::Ready(Some(event)) => {
                    if (self.filter)(&event) {
                        return std::task::Poll::Ready(Some(event));
                    }
                    // Continue loop to get next event
                }
                std::task::Poll::Ready(None) => return std::task::Poll::Ready(None),
                std::task::Poll::Pending => return std::task::Poll::Pending,
            }
        }
    }
} 