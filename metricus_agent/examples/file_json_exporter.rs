use metricus_agent::MetricsAgent;
use metricus_agent::config::MetricsConfig;
use metricus_macros::counter;
use std::str::FromStr;

#[counter(measurement = "counters", tags(key1 = "value1", key2 = "value2"))]
fn foo() {}

#[counter(measurement = "counters", tags(key1 = "value1", key2 = "value2"))]
fn bar() {}

fn main() -> anyhow::Result<()> {
    const CONFIG: &str = r#"
    exporter:
        type: file
        config:
            path: metrics.jsonl
            encoder: json
    "#;

    env_logger::init();

    MetricsAgent::init_with_config(MetricsConfig::from_str(CONFIG)?)?;

    loop {
        foo();
        bar();
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
