use opentelemetry::{logs::AnyValue, Key, KeyValue};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
#[derive(Default)]
pub struct AttributeSet(pub BTreeMap<String, serde_json::Value>);



impl From<&opentelemetry_sdk::AttributeSet> for AttributeSet {
    fn from(value: &opentelemetry_sdk::AttributeSet) -> Self {
        AttributeSet(
            value
                .iter()
                .map(|(key, value)| (key.clone().into(), to_serde_value(value.clone())))
                .collect(),
        )
    }
}

impl From<&opentelemetry_sdk::Resource> for AttributeSet {
    fn from(value: &opentelemetry_sdk::Resource) -> Self {
        AttributeSet(
            value
                .iter()
                .map(|(key, value)| (key.clone().into(), to_serde_value(value.clone())))
                .collect(),
        )
    }
}

impl From<&Vec<KeyValue>> for AttributeSet {
    fn from(value: &Vec<KeyValue>) -> Self {
        AttributeSet(
            value
                .iter()
                .map(|KeyValue { key, value }| (key.clone().into(), to_serde_value(value.clone())))
                .collect(),
        )
    }
}

impl From<Vec<(Key, AnyValue)>> for AttributeSet {
    fn from(value: Vec<(Key, AnyValue)>) -> Self {
        AttributeSet(
            value
                .iter()
                .map(|(key, value)| (key.clone().into(), anyvalue_to_serde(value.clone())))
                .collect(),
        )
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[derive(Default)]
pub struct Resource {
    attributes: AttributeSet,
    dropped_attributes_count: u64,
    schema_url: Option<String>,
}



impl From<&opentelemetry_sdk::Resource> for Resource {
    fn from(value: &opentelemetry_sdk::Resource) -> Self {
        Resource {
            attributes: AttributeSet::from(value),
            dropped_attributes_count: 0,
            schema_url: value.schema_url().map(|su| su.to_string()),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Scope {
    name: String,
    version: Option<String>,
    attributes: AttributeSet,
    dropped_attributes_count: u64,
    schema_url: Option<String>,
}

impl From<opentelemetry_sdk::Scope> for Scope {
    fn from(value: opentelemetry_sdk::Scope) -> Self {
        Scope {
            name: value.name.to_string(),
            version: value.version.map(|v| v.to_string()),
            attributes: AttributeSet::from(&value.attributes),
            dropped_attributes_count: 0,
            schema_url: value.schema_url.map(|su| su.to_string()),
        }
    }
}

fn to_serde_value(value: opentelemetry::Value) -> serde_json::Value {
    match value {
        opentelemetry::Value::Bool(b) => serde_json::Value::Bool(b),
        opentelemetry::Value::I64(i) => serde_json::Value::Number(i.into()),
        opentelemetry::Value::F64(d) => {
            serde_json::Value::Number(serde_json::Number::from_f64(d).unwrap())
        }
        opentelemetry::Value::String(s) => serde_json::Value::String(s.into()),
        opentelemetry::Value::Array(a) => {
            let vec = match a {
                opentelemetry::Array::Bool(v) => {
                    v.into_iter().map(serde_json::Value::Bool).collect()
                }
                opentelemetry::Array::I64(v) => v
                    .into_iter()
                    .map(|num| serde_json::Value::Number(num.into()))
                    .collect(),
                opentelemetry::Array::F64(v) => v
                    .into_iter()
                    .map(|num| {
                        serde_json::Value::Number(serde_json::Number::from_f64(num).unwrap())
                    })
                    .collect(),
                opentelemetry::Array::String(v) => v
                    .into_iter()
                    .map(|s| serde_json::Value::String(s.into()))
                    .collect(),
            };

            serde_json::Value::Array(vec)
        }
    }
}

fn anyvalue_to_serde(value: opentelemetry::logs::AnyValue) -> serde_json::Value {
    match value {
        opentelemetry::logs::AnyValue::Boolean(b) => serde_json::Value::Bool(b),
        opentelemetry::logs::AnyValue::Int(i) => serde_json::Value::Number(i.into()),
        opentelemetry::logs::AnyValue::Double(d) => serde_json::Value::Number(
            serde_json::Number::from_f64(d).unwrap_or(serde_json::Number::from(0)),
        ),
        opentelemetry::logs::AnyValue::String(s) => serde_json::Value::String(s.into()),
        opentelemetry::logs::AnyValue::ListAny(a) => {
            serde_json::Value::Array(a.into_iter().map(anyvalue_to_serde).collect())
        }
        opentelemetry::logs::AnyValue::Map(m) => serde_json::Value::Array(
            m.into_iter()
                .map(|(key, value)| {
                    let mut kv = serde_json::Map::new();
                    kv.insert(key.into(), anyvalue_to_serde(value));
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
