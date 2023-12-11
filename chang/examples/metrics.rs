use chang::otel::metrics::exporter::ChangMetricsExporter;
use dotenv::dotenv;
use opentelemetry::metrics::Unit;
use opentelemetry::{metrics::MeterProvider as _, KeyValue};
use opentelemetry_sdk::metrics::{MeterProvider, PeriodicReader};
use opentelemetry_sdk::{runtime, Resource};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::error::Error;
use std::{env, time};
use tokio::time::{sleep, Duration};

fn init_meter_provider(db: PgPool) -> MeterProvider {
    let exporter = ChangMetricsExporter::new(&db);

    let reader = PeriodicReader::builder(exporter, runtime::Tokio)
        .with_interval(time::Duration::from_millis(0))
        .build();

    MeterProvider::builder()
        .with_reader(reader)
        .with_resource(Resource::new(vec![KeyValue::new(
            "service.name",
            "chagn-metrics-basic-example",
        )]))
        .build()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL environment variable");

    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await
        .expect("Valid DB connection");

    // Initialize the MeterProvider with the stdout Exporter.
    let meter_provider = init_meter_provider(pool);

    // Create a meter from the above MeterProvider.
    let meter = meter_provider.meter("mylibraryname");

    // Create a Counter Instrument.
    let counter = meter.u64_counter("my_counter").init();

    // Record measurements using the Counter instrument.
    counter.add(
        10,
        [
            KeyValue::new("mykey1", "myvalue1"),
            KeyValue::new("mykey2", "myvalue2"),
        ]
        .as_ref(),
    );

    // Create a ObservableCounter instrument and register a callback that reports the measurement.
    let observable_counter = meter
        .u64_observable_counter("my_observable_counter")
        .with_description("My observable counter example description")
        .with_unit(Unit::new("myunit"))
        .init();

    meter.register_callback(&[observable_counter.as_any()], move |observer| {
        observer.observe_u64(
            &observable_counter,
            100,
            [
                KeyValue::new("mykey1", "myvalue1"),
                KeyValue::new("mykey2", "myvalue2"),
            ]
            .as_ref(),
        )
    })?;

    // Create a UpCounter Instrument.
    let updown_counter = meter.i64_up_down_counter("my_updown_counter").init();

    // Record measurements using the UpCounter instrument.
    updown_counter.add(
        -10,
        [
            KeyValue::new("mykey1", "myvalue1"),
            KeyValue::new("mykey2", "myvalue2"),
        ]
        .as_ref(),
    );

    // Create a Observable UpDownCounter instrument and register a callback that reports the measurement.
    let observable_up_down_counter = meter
        .i64_observable_up_down_counter("my_observable_updown_counter")
        .with_description("My observable updown counter example description")
        .with_unit(Unit::new("myunit"))
        .init();

    meter.register_callback(&[observable_up_down_counter.as_any()], move |observer| {
        observer.observe_i64(
            &observable_up_down_counter,
            100,
            [
                KeyValue::new("mykey1", "myvalue1"),
                KeyValue::new("mykey2", "myvalue2"),
            ]
            .as_ref(),
        )
    })?;

    // Create a Histogram Instrument.
    let histogram = meter
        .f64_histogram("my_histogram")
        .with_description("My histogram example description")
        .init();

    // Record measurements using the histogram instrument.
    histogram.record(
        10.5,
        [
            KeyValue::new("mykey1", "myvalue1"),
            KeyValue::new("mykey2", "myvalue2"),
        ]
        .as_ref(),
    );

    // Note that there is no ObservableHistogram instrument.

    // Create a ObservableGauge instrument and register a callback that reports the measurement.
    let gauge = meter
        .f64_observable_gauge("my_gauge")
        .with_description("A gauge set to 1.0")
        .with_unit(Unit::new("myunit"))
        .init();

    // Register a callback that reports the measurement.
    meter.register_callback(&[gauge.as_any()], move |observer| {
        observer.observe_f64(
            &gauge,
            1.0,
            [
                KeyValue::new("mykey1", "myvalue1"),
                KeyValue::new("mykey2", "myvalue2"),
            ]
            .as_ref(),
        )
    })?;

    // Note that Gauge only has a Observable version.
    sleep(Duration::from_secs(1)).await;
    Ok(())
}
