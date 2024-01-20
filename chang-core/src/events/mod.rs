use serde::Serialize;

pub mod channels;
pub mod collector;
pub mod exporter;
pub mod transform;

pub use collector::ChangEventCollector;

use crate::events::transform::EventRecord;

pub trait Event {
    fn kind() -> String;
    fn from_event(value: &serde_json::Value) -> serde_json::Result<Self>
    where
        Self: Sized;
}

pub fn publish<E: Serialize + Event>(value: E) {
    let body = serde_json::to_value(value).unwrap();
    let sender = channels::sender();
    let event = EventRecord::new(&E::kind(), body);
    sender.send(event).expect("failed to send event");
}
