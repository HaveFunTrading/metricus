use crate::access::get_metrics_mut;
use crate::{Id, Tags};
use quanta::Clock;
use std::cell::{LazyCell, UnsafeCell};

#[derive(Debug)]
pub struct Histogram {
    id: Id,
    clock: Clock,
}

impl Histogram {
    pub fn new(name: &str, tags: Tags) -> Self {
        Self::new_with_clock(name, tags, Clock::new())
    }

    pub fn new_with_clock(name: &str, tags: Tags, clock: Clock) -> Self {
        let histogram_id = get_metrics_mut().new_histogram(name, tags);
        Self {
            id: histogram_id,
            clock,
        }
    }
}

pub trait HistogramOps {
    fn record(&self, value: u64);

    fn span(&self) -> Span;

    fn with_span<F: FnOnce() -> R, R>(&self, f: F) -> R;
}

impl HistogramOps for Histogram {
    fn record(&self, value: u64) {
        get_metrics_mut().record(self.id, value);
    }

    fn span(&self) -> Span {
        Span {
            histogram: self,
            start_raw: self.clock.raw(),
        }
    }

    fn with_span<F: FnOnce() -> R, R>(&self, f: F) -> R {
        let _span = self.span();
        f()
    }
}

impl HistogramOps for LazyCell<UnsafeCell<Histogram>> {
    fn record(&self, value: u64) {
        unsafe { &mut *self.get() }.record(value)
    }

    fn span(&self) -> Span {
        unsafe { &mut *self.get() }.span()
    }

    fn with_span<F: FnOnce() -> R, R>(&self, f: F) -> R {
        unsafe { &mut *self.get() }.with_span(f)
    }
}

impl Drop for Histogram {
    fn drop(&mut self) {
        get_metrics_mut().delete_histogram(self.id);
    }
}

pub struct Span<'a> {
    histogram: &'a Histogram,
    start_raw: u64,
}

impl Drop for Span<'_> {
    fn drop(&mut self) {
        let end_raw = self.histogram.clock.raw();
        let elapsed = self.histogram.clock.delta_as_nanos(self.start_raw, end_raw);
        self.histogram.record(elapsed);
    }
}
