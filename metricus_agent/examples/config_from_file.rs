use metricus_agent::MetricsAgent;
use metricus_agent::config::MetricsConfig;
use metricus_allocator::{CountingAllocator, enable_allocator_instrumentation};
use metricus_macros::{counter, span};

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

#[counter(measurement = "counters", tags(key1 = "value1", key2 = "value2"))]
fn foo() {}

#[counter(measurement = "counters", tags(key1 = "value1", key2 = "value2"))]
fn bar() {}

#[span(measurement = "latencies", tags(key1 = "value1", key2 = "value2"))]
fn baz() {}

fn main() -> anyhow::Result<()> {
    enable_allocator_instrumentation();

    env_logger::init();

    MetricsAgent::init_with_config(
        MetricsConfig::from_file("metricus_agent/examples/config.yml")?
            .with_pre_allocated_metrics(CountingAllocator::metrics),
    )?;

    loop {
        foo();
        bar();
        baz();
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
