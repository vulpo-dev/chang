use crate::otel::common::{AttributeSet, Resource, Scope};

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx;
use uuid::Uuid;

/// Transformed trace data that can be serialized
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpanData {
    pub spans: Vec<Span>,
}

impl From<Vec<opentelemetry_sdk::export::trace::SpanData>> for SpanData {
    fn from(sdk_spans: Vec<opentelemetry_sdk::export::trace::SpanData>) -> Self {
        let spans = sdk_spans.into_iter().map(Span::from).collect();
        SpanData { spans }
    }
}

#[derive(Debug, Serialize)]
pub struct Span {
    id: Uuid,
    trace_id: String,
    span_id: String,
    trace_state: Option<String>,
    parent_span_id: String,
    name: String,
    kind: SpanKind,
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
    attributes: AttributeSet,
    dropped_attributes_count: u32,
    events: Vec<Event>,
    dropped_events_count: u32,
    links: Vec<Link>,
    dropped_links_count: u32,
    status: Status,

    resource: Resource,
    scope: Scope,
}

impl From<opentelemetry_sdk::export::trace::SpanData> for Span {
    fn from(value: opentelemetry_sdk::export::trace::SpanData) -> Self {
        Span {
            id: Uuid::new_v4(),
            trace_id: value.span_context.trace_id().to_string(),
            span_id: value.span_context.span_id().to_string(),
            trace_state: Some(value.span_context.trace_state().header()).filter(|s| !s.is_empty()),
            parent_span_id: Some(value.parent_span_id.to_string())
                .filter(|s| s != "0")
                .unwrap_or_default(),
            name: value.name.to_string(),
            kind: value.span_kind.into(),
            start_time: value.start_time.into(),
            end_time: value.end_time.into(),
            dropped_attributes_count: value.dropped_attributes_count,
            attributes: AttributeSet::from(&value.attributes),
            dropped_events_count: value.events.dropped_count(),
            events: value.events.into_iter().map(Into::into).collect(),
            dropped_links_count: value.links.dropped_count(),
            links: value.links.iter().map(Into::into).collect(),
            status: value.status.into(),
            resource: value.resource.as_ref().into(),
            scope: value.instrumentation_lib.clone().into(),
        }
    }
}

#[derive(sqlx::Type, Debug, Clone, Copy, Serialize)]
#[sqlx(type_name = "chang.span_kind")]
#[sqlx(rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
enum SpanKind {
    #[allow(dead_code)]
    Unspecified,
    Internal,
    Server,
    Client,
    Producer,
    Consumer,
}

impl From<opentelemetry::trace::SpanKind> for SpanKind {
    fn from(value: opentelemetry::trace::SpanKind) -> Self {
        match value {
            opentelemetry::trace::SpanKind::Client => SpanKind::Client,
            opentelemetry::trace::SpanKind::Server => SpanKind::Server,
            opentelemetry::trace::SpanKind::Producer => SpanKind::Producer,
            opentelemetry::trace::SpanKind::Consumer => SpanKind::Consumer,
            opentelemetry::trace::SpanKind::Internal => SpanKind::Internal,
        }
    }
}

#[derive(Debug, Serialize)]
struct Event {
    name: String,
    attributes: AttributeSet,
    dropped_attributes_count: u32,
}

impl From<opentelemetry::trace::Event> for Event {
    fn from(value: opentelemetry::trace::Event) -> Self {
        Event {
            name: value.name.to_string(),
            attributes: AttributeSet::from(&value.attributes),
            dropped_attributes_count: value.dropped_attributes_count,
        }
    }
}

#[derive(Debug, Serialize)]
struct Link {
    trace_id: String,
    span_id: String,
    trace_state: Option<String>,
    attributes: AttributeSet,
    dropped_attributes_count: u32,
}

impl From<&opentelemetry::trace::Link> for Link {
    fn from(value: &opentelemetry::trace::Link) -> Self {
        Link {
            trace_id: value.span_context.trace_id().to_string(),
            span_id: value.span_context.span_id().to_string(),
            trace_state: Some(value.span_context.trace_state().header()).filter(|s| !s.is_empty()),
            attributes: AttributeSet::from(&value.attributes),
            dropped_attributes_count: value.dropped_attributes_count,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Status {
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
    #[serde(skip_serializing_if = "is_zero")]
    code: u32,
}

fn is_zero(v: &u32) -> bool {
    *v == 0
}

impl From<opentelemetry::trace::Status> for Status {
    fn from(value: opentelemetry::trace::Status) -> Self {
        match value {
            opentelemetry::trace::Status::Unset => Status {
                message: None,
                code: 0,
            },
            opentelemetry::trace::Status::Error { description } => Status {
                message: Some(description.to_string()),
                code: 1,
            },
            opentelemetry::trace::Status::Ok => Status {
                message: None,
                code: 2,
            },
        }
    }
}
