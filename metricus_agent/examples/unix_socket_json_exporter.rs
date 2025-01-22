use metricus_agent::config::MetricsConfig;
use metricus_agent::MetricsAgent;
use metricus_macros::{counter, span};
use std::str::FromStr;

#[counter(measurement = "counters", tags(key1 = "value1", key2 = "value2"))]
fn foo() {}

#[counter(measurement = "counters", tags(key1 = "value1", key2 = "value2"))]
fn bar() {}

#[span(measurement = "latencies", tags(key1 = "value1", key2 = "value2"))]
fn baz() {}

fn main() -> anyhow::Result<()> {
    const CONFIG: &str = r#"
    exporter:
        type: unixsocket
        config:
            path: /tmp/metrics-agent.sock
            encoder: lineprotocol
    "#;

    env_logger::init();

    MetricsAgent::init_with_config(MetricsConfig::from_str(CONFIG)?);

    loop {
        foo();
        bar();
        baz();
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
