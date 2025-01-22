use metricus_agent::config::MetricsConfig;
use metricus_agent::MetricsAgent;
use metricus_allocator::{enable_allocator_instrumentation, CountingAllocator};
use metricus_macros::{counter, span};
use std::str::FromStr;

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

#[counter(measurement = "counters", tags(key1 = "value1", key2 = "value2"))]
fn foo() {}

#[counter(measurement = "counters", tags(key1 = "value1", key2 = "value2"))]
fn bar() {}

#[span(measurement = "latencies", tags(key1 = "value1", key2 = "value2"))]
fn baz() {}

fn main() -> anyhow::Result<()> {
    const CONFIG: &str = r#"
    exporter:
        type: udp
        config:
            host: 127.0.0.1
            port: 8777
            encoder: lineprotocol
    "#;

    enable_allocator_instrumentation();

    env_logger::init();

    MetricsAgent::init_with_config(
        MetricsConfig::from_str(CONFIG)?.with_pre_allocated_metrics(CountingAllocator::metrics),
    );

    loop {
        foo();
        bar();
        baz();
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
