use metricus::{empty_tags, register_backend, Id, MetricsBackend, Tags};

register_backend!(CustomBackend);

struct CustomBackend;

#[derive(Default)]
struct CustomBackendConfig;

impl MetricsBackend for CustomBackend {
    type Config = CustomBackendConfig;

    fn new_with_config(_config: Self::Config) -> Self {
        Self
    }

    fn name(&self) -> &'static str {
        "custom"
    }

    fn new_counter(&mut self, _name: &str, _tags: Tags) -> Id {
        println!("[CustomBackend] New counter");
        Id::default()
    }

    fn delete_counter(&mut self, _id: Id) {
        println!("[CustomBackend] Delete counter");
    }

    fn increment_counter_by(&mut self, _id: Id, _delta: usize) {
        println!("[CustomBackend] Increment counter by");
    }
}

fn main() {
    init_backend_with_config(CustomBackendConfig);

    assert_eq!("custom", get_metrics_backend_name());

    get_metrics_mut().new_counter("foo", empty_tags());
    get_metrics_mut().new_counter("bar", empty_tags());
    get_metrics_mut().new_counter("baz", empty_tags());
}
