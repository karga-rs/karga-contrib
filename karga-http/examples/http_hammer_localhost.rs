use core::f64;
use karga::{Executor, Scenario, Stage, StageExecutor};
use karga_http::{
    make_http_action, HttpActionConfig, HttpAggregate, HttpReport, HttpResponseMetric,
};
use reqwest::Method;
use std::time::Duration;
#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();
    let action_config = HttpActionConfig::builder()
        .url("http://localhost:3000")
        .method(Method::GET)
        .build();
    let results: HttpAggregate = StageExecutor::builder()
        .stages(vec![
            Stage::new(Duration::ZERO, f64::MAX),
            Stage::new(Duration::from_secs(1), f64::MAX),
        ])
        .workers(3000)
        .build()
        .exec(
            &Scenario::builder()
                .name("Http scenario")
                .action(make_http_action!(action_config))
                .build(),
        )
        .await
        .unwrap();

    let report = HttpReport::from(results);
    println!("{report:#?}");
}
