use criterion::{black_box, criterion_group, criterion_main, Criterion};
use metricus::{Counter, CounterOps};
use metricus::{set_backend, Id, MetricsBackend, Tags};
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
}

fn benchmark_static_counter(c: &mut Criterion) {
    #[counter(measurement = "counters", tags(key1 = "value1", key2 = "value2"))]
    fn foo(arg: usize) {
        black_box(arg);
    }

    set_backend(CustomBackend::new());

    c.benchmark_group("metrics").bench_function("static_counter", |b| {
        b.iter(|| {
            foo(1);
        });
    });
}

fn benchmark_manual_counter(c: &mut Criterion) {
    struct CounterHolder {
        counter: Counter,
    }

    impl CounterHolder {
        fn foo(&self, arg: usize) {
            self.counter.increment();
            black_box(arg);
        }
    }

    set_backend(CustomBackend::new());

    let counter_holder = CounterHolder {
        counter: Counter::new("counters", &[("fn_name", "foo"), ("key1", "value1"), ("key2", "value2")]),
    };

    c.benchmark_group("metrics").bench_function("manual_counter", |b| {
        b.iter(|| {
            counter_holder.foo(1);
        });
    });
}

criterion_group!(benches, benchmark_static_counter, benchmark_manual_counter);
criterion_main!(benches);
