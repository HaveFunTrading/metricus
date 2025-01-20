use metricus_agent::config::MetricsConfig;
use metricus_agent::MetricsAgent;
use metricus_macros::counter;

#[counter(measurement = "counters", tags(key1 = "value1", key2 = "value2"))]
fn foo() {}

#[counter(measurement = "counters", tags(key1 = "value1", key2 = "value2"))]
fn bar() {}

fn main() -> anyhow::Result<()> {
    const CONFIG: &'static str = r#"
    exporter:
        type: file
        config:
            path: metrics.jsonl
            encoder: json
    "#;

    MetricsAgent::init_with_config(MetricsConfig::from_str(CONFIG)?);

    loop {
        foo();
        bar();
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
