use log::{as_error, error};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio::time;

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
        let timeout = self.interval.clone();

        tokio::spawn(async move {
            let mut interval = time::interval(timeout);

            loop {
                interval.tick().await;

                let values: Vec<_> = events
                    .lock()
                    .expect("failed to lock events for export")
                    .drain(0..)
                    .collect();

                if values.len() == 0 {
                    continue;
                }

                let result = exporter.export(values).await;

                if let Err(err) = result {
                    error!(err = as_error!(err); "Error: failed to export events");
                }
            }
        });

        let events = self.events.clone();

        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                events
                    .lock()
                    .expect("failed to lock events")
                    .push(event.clone());
            }
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
