use core::cell::LazyCell;
use std::sync::LazyLock;

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use metricus::{Counter, CounterOps, Histogram, HistogramOps, Id, Metrics, Tags, set_metrics};

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

// mirrors the shape the `#[counter]`/`#[span]` macros expand to today (`std::sync::LazyLock`).
fn benchmark_lazylock_counter(c: &mut Criterion) {
    fn foo(arg: usize) {
        static COUNTER: LazyLock<Counter> =
            LazyLock::new(|| Counter::new("counters", &[("fn_name", "foo"), ("key1", "value1"), ("key2", "value2")]));
        CounterOps::increment(&COUNTER);
        black_box(arg);
    }

    set_metrics(CustomBackend::new());

    c.benchmark_group("lazy_init").bench_function("lazylock_counter", |b| {
        b.iter(|| {
            foo(1);
        });
    });
}

// mirrors the shape the macros expanded to before the thread-safety change (`core::cell::LazyCell`).
fn benchmark_lazycell_counter(c: &mut Criterion) {
    fn foo(arg: usize) {
        static mut COUNTER: LazyCell<Counter> =
            LazyCell::new(|| Counter::new("counters", &[("fn_name", "foo"), ("key1", "value1"), ("key2", "value2")]));
        #[allow(static_mut_refs)]
        unsafe {
            CounterOps::increment(&COUNTER);
        }
        black_box(arg);
    }

    set_metrics(CustomBackend::new());

    c.benchmark_group("lazy_init").bench_function("lazycell_counter", |b| {
        b.iter(|| {
            foo(1);
        });
    });
}

fn benchmark_lazylock_histogram(c: &mut Criterion) {
    fn foo(arg: usize) {
        static HISTOGRAM: LazyLock<Histogram> = LazyLock::new(|| {
            Histogram::new("latencies", &[("fn_name", "foo"), ("key1", "value1"), ("key2", "value2")])
        });
        let _span = HistogramOps::span(&HISTOGRAM);
        black_box(arg);
    }

    set_metrics(CustomBackend::new());

    c.benchmark_group("lazy_init")
        .bench_function("lazylock_histogram", |b| {
            b.iter(|| {
                foo(1);
            });
        });
}

fn benchmark_lazycell_histogram(c: &mut Criterion) {
    fn foo(arg: usize) {
        static mut HISTOGRAM: LazyCell<Histogram> = LazyCell::new(|| {
            Histogram::new("latencies", &[("fn_name", "foo"), ("key1", "value1"), ("key2", "value2")])
        });
        #[allow(static_mut_refs)]
        let _span = unsafe { HistogramOps::span(&HISTOGRAM) };
        black_box(arg);
    }

    set_metrics(CustomBackend::new());

    c.benchmark_group("lazy_init")
        .bench_function("lazycell_histogram", |b| {
            b.iter(|| {
                foo(1);
            });
        });
}

criterion_group!(benches_counter, benchmark_lazylock_counter, benchmark_lazycell_counter);
criterion_group!(benches_histogram, benchmark_lazylock_histogram, benchmark_lazycell_histogram);
criterion_main!(benches_counter, benches_histogram);
