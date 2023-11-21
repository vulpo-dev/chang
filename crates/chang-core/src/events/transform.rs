use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize, Debug)]
pub struct EventData {
    pub events: Vec<Event>,
}

#[derive(Serialize, Clone, Debug)]
pub struct Event {
    pub id: Uuid,
    pub kind: String,
    pub body: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

impl Event {
    pub fn new(kind: &str, body: serde_json::Value) -> Self {
        Event {
            id: Uuid::new_v4(),
            kind: kind.to_string(),
            body,
            created_at: Utc::now(),
        }
    }
}

impl From<Vec<Event>> for EventData {
    fn from(events: Vec<Event>) -> Self {
        EventData { events }
    }
}
