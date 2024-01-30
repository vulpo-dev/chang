use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize, Debug, PartialEq)]
pub struct EventData {
    pub events: Vec<EventRecord>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
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

#[cfg(test)]
mod test {
    use super::*;
    use crate::utils;

    #[test]
    fn event_data_from_event_records() {
        let records = utils::test::events::get_records();

        let expected = EventData {
            events: records.clone(),
        };

        let event_data = EventData::from(records);

        assert_eq!(expected, event_data);
    }
}
