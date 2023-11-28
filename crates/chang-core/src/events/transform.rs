use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize, Debug)]
pub struct EventData {
    pub events: Vec<EventRecord>,
}

#[derive(Serialize, Clone, Debug)]
pub struct EventRecord {
    pub id: Uuid,
    pub kind: String,
    pub body: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

impl EventRecord {
    pub fn new(kind: &str, body: serde_json::Value) -> Self {
        EventRecord {
            id: Uuid::new_v4(),
            kind: kind.to_string(),
            body,
            created_at: Utc::now(),
        }
    }
}

impl From<Vec<EventRecord>> for EventData {
    fn from(events: Vec<EventRecord>) -> Self {
        EventData { events }
    }
}
