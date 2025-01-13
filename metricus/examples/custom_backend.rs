use metricus::counter::Counter;
use metricus::{empty_tags, set_backend, Id, MetricsBackend, Tags};

#[derive(Debug)]
struct CustomBackend {
    counter: usize,
}

impl MetricsBackend for CustomBackend {
    type Config = ();

    fn new_with_config(_config: Self::Config) -> Self {
        Self { counter: 0 }
    }

    fn name(&self) -> &'static str {
        "custom"
    }

    fn new_counter(&mut self, _name: &str, _tags: Tags) -> Id {
        self.counter += 1;
        println!("[CustomBackend] new counter: {}", self.counter);
        Id::default()
    }

    fn delete_counter(&mut self, _id: Id) {
        // no-op
    }

    fn increment_counter_by(&mut self, _id: Id, _delta: usize) {
        // no-op
    }
}

fn main() {
    set_backend(CustomBackend::new());

    Counter::new("foo", empty_tags());
    Counter::new("bar", empty_tags());
}
