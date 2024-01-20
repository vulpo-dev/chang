use log::kv::source::as_list;
use log::kv::{Source, Visitor};
use log::{Level, Metadata, Record};
use opentelemetry::logs::{AnyValue, LogRecordBuilder, Logger, LoggerProvider, Severity};
use opentelemetry::{Key, OrderMap};
use std::borrow::Cow;

pub struct ChangLogBridge<P, L>
where
    P: LoggerProvider<Logger = L> + Send + Sync,
    L: Logger + Send + Sync,
{
    logger: L,
    _phantom: std::marker::PhantomData<P>, // P is not used in this struct
}

impl<P, L> log::Log for ChangLogBridge<P, L>
where
    P: LoggerProvider<Logger = L> + Send + Sync,
    L: Logger + Send + Sync,
{
    fn enabled(&self, _metadata: &Metadata) -> bool {
        return self.logger.event_enabled(
            map_severity_to_otel_severity(_metadata.level()),
            _metadata.target(),
        );
    }

    fn log(&self, record: &Record) {
        let mut attributes = Attributes::default();
        let _ = as_list(record.key_values()).visit(&mut attributes);

        if self.enabled(record.metadata()) {
            self.logger.emit(
                LogRecordBuilder::new()
                    .with_severity_number(map_severity_to_otel_severity(record.level()))
                    .with_severity_text(record.level().as_str())
                    // Not populating ObservedTimestamp, instead relying on OpenTelemetry
                    // API to populate it with current time.
                    .with_body(AnyValue::from(record.args().to_string()))
                    .with_attributes(attributes.attributes())
                    .build(),
            );
        }
    }

    fn flush(&self) {}
}

impl<P, L> ChangLogBridge<P, L>
where
    P: LoggerProvider<Logger = L> + Send + Sync,
    L: Logger + Send + Sync,
{
    pub fn new(provider: &P) -> Self {
        ChangLogBridge {
            logger: provider.versioned_logger(
                "chang-log-appender",
                Some(Cow::Borrowed(env!("CARGO_PKG_VERSION"))),
                None,
                None,
            ),
            _phantom: Default::default(),
        }
    }
}

fn map_severity_to_otel_severity(level: Level) -> Severity {
    match level {
        Level::Error => Severity::Error,
        Level::Warn => Severity::Warn,
        Level::Info => Severity::Info,
        Level::Debug => Severity::Debug,
        Level::Trace => Severity::Trace,
    }
}

fn serde_to_anyvalue(value: serde_json::Value) -> AnyValue {
    match value {
        serde_json::Value::Null => AnyValue::Int(0),
        serde_json::Value::Bool(b) => AnyValue::Boolean(b),
        serde_json::Value::Number(n) => AnyValue::Double(n.as_f64().unwrap()),
        serde_json::Value::String(s) => AnyValue::String(s.into()),
        serde_json::Value::Array(a) => {
            AnyValue::ListAny(a.into_iter().map(serde_to_anyvalue).collect())
        }
        serde_json::Value::Object(o) => {
            let mut map = OrderMap::new();
            o.into_iter().for_each(|(key, value)| {
                map.insert(Key::from(key), serde_to_anyvalue(value));
            });

            AnyValue::Map(map)
        }
    }
}

#[derive(Debug)]
#[derive(Default)]
struct Attributes(Vec<(Key, AnyValue)>);

impl Attributes {
    pub fn attributes(self) -> Vec<(Key, AnyValue)> {
        self.0
    }
}



impl<'kvs> Visitor<'kvs> for Attributes {
    fn visit_pair(
        &mut self,
        key: log::kv::Key<'kvs>,
        value: log::kv::Value<'kvs>,
    ) -> Result<(), log::kv::Error> {
        let key = key.to_string();

        if let Ok(value) = serde_json::to_value(value) {
            let mut attributes = vec![];
            to_flattend_object(&key, &value, &mut attributes);

            attributes.into_iter().for_each(|(key, attribute)| {
                let entry = (Key::from(key), serde_to_anyvalue(attribute));
                self.0.push(entry);
            });
        }

        Ok(())
    }
}

fn to_flattend_object(
    base_key: &str,
    value: &serde_json::Value,
    acc: &mut Vec<(String, serde_json::Value)>,
) {
    match value {
        serde_json::Value::Object(o) => o.into_iter().for_each(|(o_key, value)| {
            let k = base_key.to_string() + "." + &o_key;
            to_flattend_object(&k, value, acc);
        }),
        _ => acc.push((base_key.to_string(), value.to_owned())),
    }
}
