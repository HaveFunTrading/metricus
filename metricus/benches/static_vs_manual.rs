use criterion::{black_box, criterion_group, criterion_main, Criterion};
use metricus::{set_metrics, Counter, HistogramOps, Id, Metrics, Tags};
use metricus::{CounterOps, Histogram};
use metricus_macros::{counter, span};

#[derive(Debug)]
struct CustomBackend {
    next_id: Id,
}

impl CustomBackend {
    fn new() -> Self {
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

    fn increment_counter_by(&mut self, _id: Id, _delta: u64) {
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

fn benchmark_static_counter(c: &mut Criterion) {
    #[counter(measurement = "counters", tags(key1 = "value1", key2 = "value2"))]
    fn foo(arg: usize) {
        black_box(arg);
    }

    set_metrics(CustomBackend::new());

    c.benchmark_group("metrics").bench_function("static_counter", |b| {
        b.iter(|| {
            foo(1);
        });
    });
}

fn benchmark_static_histogram(c: &mut Criterion) {
    #[span(measurement = "latencies", tags(key1 = "value1", key2 = "value2"))]
    fn foo(arg: usize) {
        black_box(arg);
    }

    set_metrics(CustomBackend::new());

    c.benchmark_group("metrics").bench_function("static_histogram", |b| {
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

    set_metrics(CustomBackend::new());

    let counter_holder = CounterHolder {
        counter: Counter::new("counters", &[("fn_name", "foo"), ("key1", "value1"), ("key2", "value2")]),
    };

    c.benchmark_group("metrics").bench_function("manual_counter", |b| {
        b.iter(|| {
            counter_holder.foo(1);
        });
    });
}

fn benchmark_manual_histogram(c: &mut Criterion) {
    struct HistogramHolder {
        histogram: Histogram,
    }

    impl HistogramHolder {
        fn foo(&self, arg: usize) {
            let _span = self.histogram.span();
            black_box(arg);
        }
    }

    set_metrics(CustomBackend::new());

    let histogram_holder = HistogramHolder {
        histogram: Histogram::new("latencies", &[("fn_name", "foo"), ("key1", "value1"), ("key2", "value2")]),
    };

    c.benchmark_group("metrics").bench_function("manual_histogram", |b| {
        b.iter(|| {
            histogram_holder.foo(1);
        });
    });
}

criterion_group!(benches_counter, benchmark_static_counter, benchmark_manual_counter);
criterion_group!(benches_histogram, benchmark_static_histogram, benchmark_manual_histogram);
criterion_main!(benches_counter, benches_histogram);
