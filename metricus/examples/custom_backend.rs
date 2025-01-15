use metricus::{get_backend_name, set_backend, Id, MetricsBackend, Tags};
use metricus_macros::counter;

#[derive(Debug)]
struct CustomBackend {
    counter: Id,
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
        let id = self.counter;
        println!("[CustomBackend] new counter: {}", id);
        self.counter += 1;
        id
    }

    fn delete_counter(&mut self, _id: Id) {
        // no-op
    }

    fn increment_counter_by(&mut self, id: Id, _delta: usize) {
        println!("[CustomBackend] increment_counter_by: {}", id);
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

    foo();
    foo();
    foo();

    bar();
    bar();
    bar();
}
