use crate::otel::common::{AttributeSet, Resource, Scope};
use chrono::{DateTime, Utc};
use opentelemetry::{global, metrics::MetricsError};
use opentelemetry_sdk::metrics::data;
use serde::Serialize;
use std::any::Any;
use uuid::Uuid;

/// Transformed metrics data that can be serialized
#[derive(Serialize, Debug, Clone)]
pub struct MetricsData {
    pub metrics: Vec<Metric>,
}

impl From<&mut data::ResourceMetrics> for MetricsData {
    fn from(value: &mut data::ResourceMetrics) -> Self {
        let resource = Resource::from(&value.resource);

        let metrics = value
            .scope_metrics
            .iter()
            .flat_map(|scope_metric| {
                let scope = Scope::from(scope_metric.scope.clone());
                scope_metric
                    .metrics
                    .iter()
                    .map(move |metric| Metric::builder(metric, scope.clone()))
            })
            .map(|mut m| m.set_resource(resource.clone()).clone().buid())
            .collect();

        MetricsData { metrics }
    }
}

#[derive(Serialize, Debug, Clone)]
struct Unit(String);

impl From<opentelemetry::metrics::Unit> for Unit {
    fn from(unit: opentelemetry::metrics::Unit) -> Self {
        Unit(unit.as_str().to_string().into())
    }
}

#[derive(Serialize, Debug, Clone)]
pub struct Metric {
    id: Uuid,
    name: String,
    description: String,
    unit: Unit,
    data: Option<MetricData>,

    resource: Resource,
    scope: Scope,
}

#[derive(Serialize, Debug, Clone)]
pub struct MetricBuilder {
    name: String,
    description: String,
    unit: Unit,
    data: Option<MetricData>,

    resource: Option<Resource>,
    scope: Scope,
}

impl Metric {
    pub fn builder(value: &data::Metric, scope: Scope) -> MetricBuilder {
        MetricBuilder {
            name: value.name.to_string(),
            description: value.description.to_string(),
            unit: value.unit.clone().into(),
            data: map_data(value.data.as_any()),
            scope,
            resource: None,
        }
    }
}

impl MetricBuilder {
    pub fn set_resource(&mut self, resource: Resource) -> &mut MetricBuilder {
        self.resource = Some(resource);
        self
    }

    pub fn buid(self) -> Metric {
        Metric {
            id: Uuid::new_v4(),
            name: self.name,
            description: self.description,
            unit: self.unit,
            data: self.data,
            resource: self.resource.unwrap_or_default(),
            scope: self.scope,
        }
    }
}

fn map_data(data: &dyn Any) -> Option<MetricData> {
    if let Some(hist) = data.downcast_ref::<data::Histogram<i64>>() {
        Some(MetricData::Histogram(hist.into()))
    } else if let Some(hist) = data.downcast_ref::<data::Histogram<u64>>() {
        Some(MetricData::Histogram(hist.into()))
    } else if let Some(hist) = data.downcast_ref::<data::Histogram<f64>>() {
        Some(MetricData::Histogram(hist.into()))
    } else if let Some(hist) = data.downcast_ref::<data::ExponentialHistogram<i64>>() {
        Some(MetricData::ExponentialHistogram(hist.into()))
    } else if let Some(hist) = data.downcast_ref::<data::ExponentialHistogram<u64>>() {
        Some(MetricData::ExponentialHistogram(hist.into()))
    } else if let Some(hist) = data.downcast_ref::<data::ExponentialHistogram<f64>>() {
        Some(MetricData::ExponentialHistogram(hist.into()))
    } else if let Some(sum) = data.downcast_ref::<data::Sum<u64>>() {
        Some(MetricData::Sum(sum.into()))
    } else if let Some(sum) = data.downcast_ref::<data::Sum<i64>>() {
        Some(MetricData::Sum(sum.into()))
    } else if let Some(sum) = data.downcast_ref::<data::Sum<f64>>() {
        Some(MetricData::Sum(sum.into()))
    } else if let Some(gauge) = data.downcast_ref::<data::Gauge<u64>>() {
        Some(MetricData::Gauge(gauge.into()))
    } else if let Some(gauge) = data.downcast_ref::<data::Gauge<i64>>() {
        Some(MetricData::Gauge(gauge.into()))
    } else if let Some(gauge) = data.downcast_ref::<data::Gauge<f64>>() {
        Some(MetricData::Gauge(gauge.into()))
    } else {
        global::handle_error(MetricsError::Other("unknown aggregator".into()));
        None
    }
}

#[derive(Serialize, Debug, Clone)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
enum MetricData {
    Gauge(Gauge),
    Sum(Sum),
    Histogram(Histogram),
    ExponentialHistogram(ExponentialHistogram),
}

#[derive(Serialize, Debug, Clone)]
#[serde(untagged)]
enum DataValue {
    F64(f64),
    I64(i64),
    U64(u64),
}

impl From<f64> for DataValue {
    fn from(value: f64) -> Self {
        DataValue::F64(value)
    }
}

impl From<i64> for DataValue {
    fn from(value: i64) -> Self {
        DataValue::I64(value)
    }
}

impl From<u64> for DataValue {
    fn from(value: u64) -> Self {
        DataValue::U64(value)
    }
}

#[derive(Serialize, Debug, Clone)]
struct Gauge {
    data_points: Vec<DataPoint>,
}

impl<T: Into<DataValue> + Copy> From<&data::Gauge<T>> for Gauge {
    fn from(value: &data::Gauge<T>) -> Self {
        Gauge {
            data_points: value.data_points.iter().map(Into::into).collect(),
        }
    }
}

#[derive(Serialize, Debug, Clone, Copy)]
enum Temporality {
    #[allow(dead_code)]
    Unspecified, // explicitly never used
    Delta,
    Cumulative,
}

impl From<data::Temporality> for Temporality {
    fn from(value: data::Temporality) -> Self {
        match value {
            data::Temporality::Cumulative => Temporality::Cumulative,
            data::Temporality::Delta => Temporality::Delta,
            _ => panic!("unexpected temporality"),
        }
    }
}

#[derive(Serialize, Debug, Clone)]
struct Sum {
    data_points: Vec<DataPoint>,
    aggregation_temporality: Temporality,
    is_monotonic: bool,
}

impl<T: Into<DataValue> + Copy> From<&data::Sum<T>> for Sum {
    fn from(value: &data::Sum<T>) -> Self {
        Sum {
            data_points: value.data_points.iter().map(Into::into).collect(),
            aggregation_temporality: value.temporality.into(),
            is_monotonic: value.is_monotonic,
        }
    }
}

#[derive(Serialize, Debug, Clone)]
struct DataPoint {
    attributes: AttributeSet,
    start_time: Option<DateTime<Utc>>,
    time: Option<DateTime<Utc>>,
    value: DataValue,
    exemplars: Vec<Exemplar>,
    flags: u8,
}

impl<T: Into<DataValue> + Copy> From<&data::DataPoint<T>> for DataPoint {
    fn from(value: &data::DataPoint<T>) -> Self {
        DataPoint {
            attributes: AttributeSet::from(&value.attributes),
            start_time: value.start_time.map(Into::into),
            time: value.time.map(Into::into),
            value: value.value.into(),
            exemplars: value.exemplars.iter().map(Into::into).collect(),
            flags: 0,
        }
    }
}

#[derive(Serialize, Debug, Clone)]
struct Histogram {
    data_points: Vec<HistogramDataPoint>,
    aggregation_temporality: Temporality,
}

impl<T: Into<DataValue> + Copy> From<&data::Histogram<T>> for Histogram {
    fn from(value: &data::Histogram<T>) -> Self {
        Histogram {
            data_points: value.data_points.iter().map(Into::into).collect(),
            aggregation_temporality: value.temporality.into(),
        }
    }
}

#[derive(Serialize, Debug, Clone)]
struct HistogramDataPoint {
    attributes: AttributeSet,
    start_time: DateTime<Utc>,
    time: DateTime<Utc>,
    count: u64,
    explicit_bounds: Vec<f64>,
    bucket_counts: Vec<u64>,
    min: Option<DataValue>,
    max: Option<DataValue>,
    sum: DataValue,
    exemplars: Vec<Exemplar>,
    flags: u8,
}

impl<T: Into<DataValue> + Copy> From<&data::HistogramDataPoint<T>> for HistogramDataPoint {
    fn from(value: &data::HistogramDataPoint<T>) -> Self {
        HistogramDataPoint {
            attributes: AttributeSet::from(&value.attributes),
            start_time: value.start_time.into(),
            time: value.time.into(),
            count: value.count,
            explicit_bounds: value.bounds.clone(),
            bucket_counts: value.bucket_counts.clone(),
            min: value.min.map(Into::into),
            max: value.max.map(Into::into),
            sum: value.sum.into(),
            exemplars: value.exemplars.iter().map(Into::into).collect(),
            flags: 0,
        }
    }
}
#[derive(Serialize, Debug, Clone)]
struct ExponentialHistogram {
    data_points: Vec<ExponentialHistogramDataPoint>,
    aggregation_temporality: Temporality,
}

impl<T: Into<DataValue> + Copy> From<&data::ExponentialHistogram<T>> for ExponentialHistogram {
    fn from(value: &data::ExponentialHistogram<T>) -> Self {
        ExponentialHistogram {
            data_points: value.data_points.iter().map(Into::into).collect(),
            aggregation_temporality: value.temporality.into(),
        }
    }
}

#[derive(Serialize, Debug, Clone)]
struct ExponentialHistogramDataPoint {
    attributes: AttributeSet,
    start_time: DateTime<Utc>,
    time: DateTime<Utc>,
    count: usize,
    min: Option<DataValue>,
    max: Option<DataValue>,
    sum: DataValue,
    scale: i8,
    zero_count: u64,
    positive: ExponentialBucket,
    negative: ExponentialBucket,
    zero_threshold: f64,
    exemplars: Vec<Exemplar>,
    flags: u8,
}

impl<T: Into<DataValue> + Copy> From<&data::ExponentialHistogramDataPoint<T>>
    for ExponentialHistogramDataPoint
{
    fn from(value: &data::ExponentialHistogramDataPoint<T>) -> Self {
        ExponentialHistogramDataPoint {
            attributes: AttributeSet::from(&value.attributes),
            start_time: value.start_time.into(),
            time: value.time.into(),
            count: value.count,
            min: value.min.map(Into::into),
            max: value.max.map(Into::into),
            sum: value.sum.into(),
            scale: value.scale,
            zero_count: value.zero_count,
            positive: (&value.positive_bucket).into(),
            negative: (&value.negative_bucket).into(),
            zero_threshold: value.zero_threshold,
            exemplars: value.exemplars.iter().map(Into::into).collect(),
            flags: 0,
        }
    }
}

impl From<&data::ExponentialBucket> for ExponentialBucket {
    fn from(b: &data::ExponentialBucket) -> Self {
        ExponentialBucket {
            offset: b.offset,
            bucket_counts: b.counts.clone(),
        }
    }
}

#[derive(Serialize, Debug, Clone)]
struct ExponentialBucket {
    offset: i32,
    bucket_counts: Vec<u64>,
}

#[derive(Serialize, Debug, Clone)]
struct Exemplar {
    filtered_attributes: AttributeSet,
    time: DateTime<Utc>,
    value: DataValue,
    span_id: String,
    trace_id: String,
}

impl<T: Into<DataValue> + Copy> From<&data::Exemplar<T>> for Exemplar {
    fn from(value: &data::Exemplar<T>) -> Self {
        Exemplar {
            filtered_attributes: AttributeSet::from(&value.filtered_attributes),
            time: value.time.into(),
            value: value.value.into(),
            span_id: format!("{:016x}", u64::from_be_bytes(value.span_id)),
            trace_id: format!("{:032x}", u128::from_be_bytes(value.trace_id)),
        }
    }
}
