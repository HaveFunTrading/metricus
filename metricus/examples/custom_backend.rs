use metricus::{get_metrics_backend_name, set_metrics, Id, Metrics, Tags};
use metricus_macros::{counter, span};

#[derive(Debug)]
struct CustomBackend {
    next_id: Id,
}

impl CustomBackend {
    pub fn new() -> Self {
        Self { next_id: 0 }
    }
}

impl Metrics for CustomBackend {
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

#[span(measurement = "latencies", tags(key1 = "value1", key2 = "value2"))]
fn baz() {}

fn main() {
    set_metrics(CustomBackend::new());
    assert_eq!("custom", get_metrics_backend_name());

    foo();
    foo();
    foo();

    bar();
    bar();
    bar();

    baz();
    baz();
    baz();
}
