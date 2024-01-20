use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::otel::common::{AttributeSet, Resource, Scope};

#[derive(Debug, Deserialize, Serialize)]
pub struct LogData {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub logs: Vec<LogRecord>,
}

impl From<Vec<opentelemetry_sdk::export::logs::LogData>> for LogData {
    fn from(sdk_logs: Vec<opentelemetry_sdk::export::logs::LogData>) -> LogData {
        let logs = sdk_logs
            .into_iter()
            .map(LogRecord::from)
            .collect();

        LogData { logs }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LogRecord {
    id: Uuid,
    time: Option<DateTime<Utc>>,
    observed_time: DateTime<Utc>,
    severity_number: u32,
    severity_text: Option<String>,
    body: Option<serde_json::Value>,
    attributes: AttributeSet,
    dropped_attributes_count: u32,
    flags: Option<u8>,
    span_id: Option<String>,
    trace_id: Option<String>,

    resource: Resource,
    scope: Scope,
}

impl From<opentelemetry_sdk::export::logs::LogData> for LogRecord {
    fn from(value: opentelemetry_sdk::export::logs::LogData) -> Self {
        LogRecord {
            id: Uuid::new_v4(),
            trace_id: value
                .record
                .trace_context
                .as_ref()
                .map(|c| c.trace_id.to_string()),
            span_id: value
                .record
                .trace_context
                .as_ref()
                .map(|c| c.span_id.to_string()),
            flags: value
                .record
                .trace_context
                .map(|c| c.trace_flags.map(|f| f.to_u8()))
                .unwrap_or_default(),
            time: value.record.timestamp.map(|st| st.into()),
            observed_time: value.record.observed_timestamp.into(),
            severity_number: value
                .record
                .severity_number
                .map(|u| u as u32)
                .unwrap_or_default(),
            attributes: value
                .record
                .attributes
                .map(AttributeSet::from)
                .unwrap_or_default(),
            dropped_attributes_count: 0,
            severity_text: value.record.severity_text.map(String::from),
            body: value.record.body.map(to_serde_value),
            resource: value.resource.as_ref().into(),
            scope: value.instrumentation.clone().into(),
        }
    }
}

fn to_serde_value(value: opentelemetry::logs::AnyValue) -> serde_json::Value {
    match value {
        opentelemetry::logs::AnyValue::Boolean(b) => serde_json::Value::Bool(b),
        opentelemetry::logs::AnyValue::Int(i) => serde_json::Value::Number(i.into()),
        opentelemetry::logs::AnyValue::Double(d) => serde_json::Value::Number(
            serde_json::Number::from_f64(d).unwrap_or(serde_json::Number::from(0)),
        ),
        opentelemetry::logs::AnyValue::String(s) => serde_json::Value::String(s.into()),
        opentelemetry::logs::AnyValue::ListAny(a) => {
            serde_json::Value::Array(a.into_iter().map(to_serde_value).collect())
        }
        opentelemetry::logs::AnyValue::Map(m) => serde_json::Value::Array(
            m.into_iter()
                .map(|(key, value)| {
                    let mut kv = serde_json::Map::new();
                    kv.insert(key.into(), to_serde_value(value));
                    serde_json::Value::Object(kv)
                })
                .collect(),
        ),
        opentelemetry::logs::AnyValue::Bytes(b) => serde_json::Value::Array(
            b.into_iter()
                .map(|num| serde_json::Value::Number(num.into()))
                .collect(),
        ),
    }
}
