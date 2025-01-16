use metricus::{empty_tags, get_backend_name, set_backend, Counter, Id, MetricsBackend, Tags};
use metricus_macros::counter;

#[derive(Debug)]
struct CustomBackend {
    next_id: Id,
}

impl MetricsBackend for CustomBackend {
    type Config = ();

    fn new_with_config(_config: Self::Config) -> Self {
        Self { next_id: 0 }
    }

    fn name(&self) -> &'static str {
        "custom"
    }

    fn new_counter(&mut self, _name: &str, _tags: Tags) -> Id {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn delete_counter(&mut self, _id: Id) {
        // no-op
    }

    fn increment_counter_by(&mut self, _id: Id, _delta: usize) {
        // no-op
    }

    fn new_histogram(&mut self, _name: &str, _tags: Tags) -> Id {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn delete_histogram(&mut self, _id: Id) {
        // no-op
    }

    fn record(&mut self, _id: Id, _value: u64) {
        // no-op
    }
}

#[counter(measurement = "counters", tags(key1 = "value1", key2 = "value2"))]
fn foo() {}

#[counter(measurement = "counters", tags(key1 = "value1", key2 = "value2"))]
fn bar() {}

fn main() {
    set_backend(CustomBackend::new());
    assert_eq!("custom", get_backend_name());

    Counter::new("", empty_tags());

    foo();
    foo();
    foo();

    bar();
    bar();
    bar();
}
