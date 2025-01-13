use metricus::{empty_tags, get_metrics2_mut, init_backend2, BackendHandle, BackendVTable, Id, MetricsBackend, Tags};

#[derive(Debug)]
struct CustomBackend {
    counter: usize,
}

impl MetricsBackend for CustomBackend {
    type Config = ();

    fn new_with_config(config: Self::Config) -> Self {
        Self { counter: 0 }
    }

    fn name(&self) -> &'static str {
        "custom"
    }

    fn new_counter(&mut self, name: &str, tags: Tags) -> Id {
        Id::default()
    }

    fn delete_counter(&mut self, id: Id) {
        // no-op
    }

    fn increment_counter_by(&mut self, id: Id, delta: usize) {
        // no-op
    }
}

fn main() {
    init_backend2(CustomBackend::new().into_handle());

    get_metrics2_mut().new_counter("foo", empty_tags());
    get_metrics2_mut().new_counter("bar", empty_tags());
}
