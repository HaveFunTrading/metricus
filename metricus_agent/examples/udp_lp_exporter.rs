use metricus_agent::MetricsAgent;
use metricus_agent::config::MetricsConfig;
use metricus_allocator::{CountingAllocator, enable_allocator_instrumentation};
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
        type: unix_datagram
        config:
            path: /var/run/shared-socket/telegraf.sock
            encoder: line_protocol
    "#;

    enable_allocator_instrumentation();

    env_logger::init();

    MetricsAgent::init_with_config(
        MetricsConfig::from_str(CONFIG)?
            .with_pre_allocated_metrics(CountingAllocator::metrics)
            .with_default_tags(vec![("example_name".to_owned(), "udp_lp_exporter".to_owned())]),
    )?;

    loop {
        foo();
        bar();
        baz();
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
