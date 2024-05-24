use log::{error, info};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio::time;
use tokio_util::sync::CancellationToken;

use crate::events::channels;
use crate::events::exporter::EventExporter;
use crate::events::transform::EventRecord;

type Channels = (UnboundedSender<EventRecord>, UnboundedReceiver<EventRecord>);

pub struct ChangEventCollector {
    events: Arc<Mutex<Vec<EventRecord>>>,
    exporter: Arc<Box<dyn EventExporter>>,
    interval: Duration,
}

impl ChangEventCollector {
    pub fn builder() -> ChangEventCollectorBuilder {
        ChangEventCollectorBuilder::default()
    }

    pub fn new(
        exporter: Box<dyn EventExporter + 'static>,
        interval: Duration,
    ) -> ChangEventCollector {
        let events = Arc::new(Mutex::new(Vec::new()));
        ChangEventCollector {
            events,
            exporter: Arc::new(exporter),
            interval,
        }
    }

    pub fn start(&self) {
        let (tx, mut rx): Channels = mpsc::unbounded_channel();

        channels::init(&tx);

        let events = self.events.clone();
        let exporter = self.exporter.clone();
        let timeout = self.interval;
        let token = CancellationToken::new();

        let cancel_token1 = token.clone();
        tokio::spawn(async move {
            let mut interval = time::interval(timeout);

            loop {
                interval.tick().await;

                let values: Vec<_> = events
                    .lock()
                    .expect("failed to lock events for export")
                    .drain(0..)
                    .collect();

                if values.is_empty() {
                    continue;
                }

                let result = exporter.export(values).await;

                if let Err(err) = result {
                    error!(key:err = err; "Error: failed to export events");
                    // TODO: add retry
                }

                if cancel_token1.is_cancelled() {
                    break;
                }
            }
        });

        let events = self.events.clone();
        let cancel_token2 = token.clone();
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                events
                    .lock()
                    .expect("failed to lock events") // TODO: add retry?
                    .push(event.clone());

                if cancel_token2.is_cancelled() {
                    break;
                }
            }
        });

        tokio::spawn(async move {
            tokio::signal::ctrl_c().await.unwrap();
            info!("Chang Tasks Shutdown requested. Waiting for pending tasks");
            token.cancel();
        });
    }
}

pub struct ChangEventCollectorBuilder {
    exporter: Option<Box<dyn EventExporter>>,
    interval: Duration,
}

impl Default for ChangEventCollectorBuilder {
    fn default() -> Self {
        ChangEventCollectorBuilder {
            exporter: None,
            interval: Duration::from_secs(5),
        }
    }
}

impl ChangEventCollectorBuilder {
    pub fn with_exporter(
        mut self,
        exporter: impl EventExporter + 'static,
    ) -> ChangEventCollectorBuilder {
        self.exporter = Some(Box::new(exporter));
        self
    }

    pub fn with_interval(mut self, d: Duration) -> ChangEventCollectorBuilder {
        self.interval = d;
        self
    }

    pub fn start(self) -> ChangEventCollector {
        let exporter = self.exporter.expect("Missing EventExporter");
        let interval = self.interval;

        let event_collector = ChangEventCollector::new(exporter, interval);

        event_collector.start();

        event_collector
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        db::migration,
        events::{self, exporter::ChangEventExporter, Event},
        utils,
    };

    use fake::{faker::lorem::en::Word, Fake};
    use rand::Rng;
    use serde::{Deserialize, Serialize};
    use tokio::time::{sleep, Duration};

    #[derive(Deserialize, Serialize)]
    struct Nested {
        inner: i32,
    }

    #[derive(Deserialize, Serialize)]
    struct Yak {
        name: String,
        other: String,
        nested: Nested,
    }

    impl Event for Yak {
        fn kind() -> String {
            String::from("yak")
        }

        fn from_event(value: &serde_json::Value) -> serde_json::Result<Self> {
            serde_json::from_value(value.clone())
        }
    }

    #[tokio::test]
    async fn collector() {
        let prepare = utils::test::prepare().await;

        migration::base(&prepare.pool).await;
        migration::events(&prepare.pool).await;

        let exporter = ChangEventExporter::new(&prepare.pool);
        let _collector = ChangEventCollector::builder()
            .with_exporter(exporter)
            .with_interval(Duration::from_millis(100))
            .start();

        for event in get_records().into_iter() {
            events::publish(event);
        }

        sleep(Duration::from_millis(500)).await;

        // TODO: assert inserted items

        utils::test::cleanup(prepare).await;
    }

    #[tokio::test]
    #[should_panic]
    async fn panic_without_exporter() {
        let _collector = ChangEventCollector::builder()
            .with_interval(Duration::from_millis(100))
            .start();
    }

    fn get_records() -> Vec<Yak> {
        let mut records: Vec<Yak> = Vec::new();
        let mut rng = rand::thread_rng();

        for _ in 0..10 {
            records.push(Yak {
                name: Word().fake(),
                other: Word().fake(),
                nested: Nested { inner: rng.gen() },
            });
        }

        records
    }
}
