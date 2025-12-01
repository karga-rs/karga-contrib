use hdrhistogram::Histogram;
use karga::{Aggregate, Metric, Report};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, time::Duration};
use typed_builder::TypedBuilder;

#[derive(Clone, PartialEq, PartialOrd)]
pub struct HttpResponseMetric {
    pub latency: Duration,
    pub status_code: u16,
    pub bytes_sent: u64,
    pub bytes_received: u64,
}

// Sometime a request can fail so the metrics shall be ignored
#[derive(Clone, PartialEq, PartialOrd)]
pub enum HttpMetric {
    Success(HttpResponseMetric),
    Failure,
}

impl Metric for HttpMetric {}
pub struct HttpFailedRequestMetric {}
#[derive(Clone)]
pub struct HttpAggregate {
    pub latency_hist: Histogram<u64>,
    pub status_count: HashMap<u16, u64>,
    pub total_bytes_sent: u64,
    pub total_bytes_received: u64,
    pub count: u64,
    pub failure_count: u64,
}

impl Aggregate for HttpAggregate {
    type Metric = HttpMetric;

    fn new() -> Self {
        Self {
            latency_hist: Histogram::new(3).expect("Create histogram"),
            status_count: HashMap::new(),
            total_bytes_sent: 0,
            total_bytes_received: 0,
            count: 0,
            failure_count: 0,
        }
    }

    fn consume(&mut self, metric: &Self::Metric) {
        match metric {
            HttpMetric::Success(metric) => {
                let res = self.latency_hist.record(metric.latency.as_nanos() as u64);
                if let Err(res) = res {
                    tracing::warn!("Ignoring metric reading due to error: {res}");
                    self.failure_count += 1;
                    return;
                }
                *self.status_count.entry(metric.status_code).or_default() += 1;
                self.total_bytes_sent += metric.bytes_sent;
                self.total_bytes_received += metric.bytes_received;
            }
            HttpMetric::Failure => self.failure_count += 1,
        };
        self.count += 1;
    }

    fn merge(&mut self, other: Self) {
        self.latency_hist += other.latency_hist;

        for (status_code, other_count) in other.status_count {
            *self.status_count.entry(status_code).or_default() += other_count;
        }
        self.total_bytes_sent += other.total_bytes_sent;
        self.total_bytes_received += other.total_bytes_received;
        self.failure_count += other.failure_count;
        self.count += other.count;
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct HttpLatencyStats {
    pub avg: Duration,
    pub min: Duration,
    pub med: Duration,
    pub max: Duration,
    pub p90: Duration,
    pub p95: Duration,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct HttpReport {
    pub req_duration: HttpLatencyStats,
    pub reqs_total: u64,
    pub req_failure_ratio: f64,
    pub status_codes: HashMap<u16, u64>,
    pub data_sent: u64,
    pub data_received: u64,
}

impl From<HttpAggregate> for HttpReport {
    fn from(value: HttpAggregate) -> Self {
        let req_duration = HttpLatencyStats {
            avg: Duration::from_nanos(value.latency_hist.mean() as u64),
            min: Duration::from_nanos(value.latency_hist.min()),
            med: Duration::from_nanos(value.latency_hist.value_at_quantile(0.5)),
            max: Duration::from_nanos(value.latency_hist.max()),
            p90: Duration::from_nanos(value.latency_hist.value_at_quantile(0.90)),
            p95: Duration::from_nanos(value.latency_hist.value_at_quantile(0.95)),
        };

        Self {
            req_duration,
            reqs_total: value.count,
            req_failure_ratio: (value.failure_count as f64 / value.count as f64) * 100.0,
            status_codes: value.status_count,
            data_sent: value.total_bytes_sent,
            data_received: value.total_bytes_received,
        }
    }
}

impl Report<HttpAggregate> for HttpReport {}

pub use reqwest::header::HeaderMap as Headers;
pub use reqwest::Body;
pub use reqwest::Method;
pub use reqwest::Url;

#[derive(TypedBuilder)]
pub struct HttpActionConfig {
    #[builder(default = Client::new())]
    pub client: Client,

    pub method: Method,

    #[builder(setter(transform = |s: &str| Url::parse(s).expect("Invalid URL passed to HttpActionConfig")))]
    pub url: Url,

    #[builder(default = None)]
    pub headers: Option<Headers>,

    #[builder(default = None)]
    pub body: Option<Body>,
}

#[macro_export]
macro_rules! make_http_action {
    ($config:expr) => {{
        let config = $config;

        let mut req_builder = config
            .client
            .request(config.method.clone(), config.url.clone());
        if let Some(h) = config.headers {
            req_builder = req_builder.headers(h)
        }

        if let Some(b) = config.body {
            req_builder = req_builder.body(b)
        }
        let req = req_builder.build().expect("Unable to build request");
        req.try_clone().expect("request must be Clone");
        let req = std::sync::Arc::new(req);
        move || {
            let client = config.client.clone();
            let req = req.clone();
            async move {
                let req = req.try_clone().unwrap();
                let start = std::time::Instant::now();
                let client = client.clone();
                let res = client.execute(req).await;
                let elapsed = start.elapsed();
                match res {
                    Ok(res) => HttpMetric::Success(HttpResponseMetric {
                        latency: elapsed,
                        status_code: res.status().into(),
                        bytes_received: res.content_length().unwrap_or(0),
                        bytes_sent: 0,
                    }),
                    Err(_) => HttpMetric::Failure,
                }
            }
        }
    }};
}

#[cfg(test)]
mod tests {
    use karga::Scenario;

    use super::*;

    #[test]
    fn action_compatibility() {
        let config = HttpActionConfig::builder()
            .method(Method::GET)
            .url("http://localhost:3000")
            .build();

        let _: Scenario<HttpAggregate, _, _> = Scenario::builder()
            .name("random")
            .action(make_http_action!(config))
            .build();
    }
}
