use criterion::{Criterion, black_box, criterion_group, criterion_main};
use metricus::{Counter, HistogramOps};
use metricus::{CounterOps, Histogram};
use metricus_agent::MetricsAgent;
use metricus_macros::{counter, span};

fn benchmark_static_counter(c: &mut Criterion) {
    #[counter(measurement = "counters", tags(key1 = "value1", key2 = "value2"))]
    fn foo(arg: usize) {
        black_box(arg);
    }

    MetricsAgent::init().unwrap();

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

    MetricsAgent::init().unwrap();

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

    MetricsAgent::init().unwrap();

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

    MetricsAgent::init().unwrap();

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
