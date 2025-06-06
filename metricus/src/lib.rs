#![doc = include_str!("../README.md")]

mod counter;
mod histogram;

use crate::access::get_metrics;
// re-exports
pub use counter::{Counter, CounterOps};
pub use histogram::{Histogram, HistogramOps, Span};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use std::collections::HashMap;
use std::sync::atomic::{AtomicPtr, Ordering};

/// Metric id.
pub type Id = u64;
/// Metric tag expresses as key-value pair.
pub type Tag<'a> = (&'a str, &'a str);
/// Metrics tags expresses as array of key-value pairs.
pub type Tags<'a> = &'a [Tag<'a>];

/// Returns empty tags.
pub const fn empty_tags() -> Tags<'static> {
    &[]
}

/// Common interface for metrics backend. Each new backend must implement this trait.
pub trait Metrics {
    fn name(&self) -> &'static str;

    fn new_counter(&mut self, name: &str, tags: Tags) -> Id;

    fn delete_counter(&mut self, id: Id);

    fn increment_counter_by(&mut self, id: Id, delta: u64);

    fn increment_counter(&mut self, id: Id) {
        self.increment_counter_by(id, 1)
    }

    fn new_histogram(&mut self, name: &str, tags: Tags) -> Id;

    fn delete_histogram(&mut self, id: Id);

    fn record(&mut self, id: Id, value: u64);
}

trait IntoHandle {
    fn into_handle(self) -> MetricsHandle;
}

impl<T: Metrics + Sized> IntoHandle for T {
    fn into_handle(self) -> MetricsHandle {
        let name = self.name();
        let ptr = Box::into_raw(Box::new(self)) as *mut _;

        let vtable = MetricsVTable {
            new_counter: new_counter_raw::<Self>,
            delete_counter: delete_counter_raw::<Self>,
            increment_counter: increment_counter_raw::<Self>,
            increment_counter_by: increment_counter_by_raw::<Self>,
            new_histogram: new_histogram_raw::<Self>,
            delete_histogram: delete_histogram_raw::<Self>,
            record: record_raw::<Self>,
        };
        MetricsHandle { ptr, vtable, name }
    }
}

#[inline]
fn new_counter_raw<T: Metrics>(ptr: *mut u8, name: &str, tags: Tags) -> Id {
    let metrics = unsafe { &mut *(ptr as *mut T) };
    metrics.new_counter(name, tags)
}

#[inline]
fn delete_counter_raw<T: Metrics>(ptr: *mut u8, id: Id) {
    let metrics = unsafe { &mut *(ptr as *mut T) };
    metrics.delete_counter(id)
}

#[inline]
fn increment_counter_by_raw<T: Metrics>(ptr: *mut u8, id: Id, delta: u64) {
    let metrics = unsafe { &mut *(ptr as *mut T) };
    metrics.increment_counter_by(id, delta)
}

#[inline]
fn increment_counter_raw<T: Metrics>(ptr: *mut u8, id: Id) {
    increment_counter_by_raw::<T>(ptr, id, 1)
}

#[inline]
fn new_histogram_raw<T: Metrics>(ptr: *mut u8, name: &str, tags: Tags) -> Id {
    let metrics = unsafe { &mut *(ptr as *mut T) };
    metrics.new_histogram(name, tags)
}

#[inline]
fn delete_histogram_raw<T: Metrics>(ptr: *mut u8, id: Id) {
    let metrics = unsafe { &mut *(ptr as *mut T) };
    metrics.delete_histogram(id)
}

#[inline]
fn record_raw<T: Metrics>(ptr: *mut u8, id: Id, value: u64) {
    let metrics = unsafe { &mut *(ptr as *mut T) };
    metrics.record(id, value)
}

/// Pre-allocated metric consists of name, id and tags.
#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
pub enum PreAllocatedMetric {
    Counter {
        name: String,
        id: Id,
        #[serde_as(as = "HashMap<_, _>")]
        #[serde(default)]
        tags: Vec<(String, String)>,
    },
    Histogram {
        name: String,
        id: Id,
        #[serde_as(as = "HashMap<_, _>")]
        #[serde(default)]
        tags: Vec<(String, String)>,
    },
}

impl PreAllocatedMetric {
    pub fn counter(name: &str, id: Id, tags: &[Tag]) -> Self {
        PreAllocatedMetric::Counter {
            name: name.to_owned(),
            id,
            tags: tags.iter().map(|tag| (tag.0.to_owned(), tag.1.to_owned())).collect(),
        }
    }

    pub fn histogram(name: &str, id: Id, tags: &[Tag]) -> Self {
        PreAllocatedMetric::Histogram {
            name: name.to_owned(),
            id,
            tags: tags.iter().map(|tag| (tag.0.to_owned(), tag.1.to_owned())).collect(),
        }
    }
}

/// A trivial no-op backend for the "uninitialized" state.
struct NoOpMetrics;

impl Metrics for NoOpMetrics {
    fn name(&self) -> &'static str {
        "no-op"
    }

    fn new_counter(&mut self, _name: &str, _tags: Tags) -> Id {
        Id::default()
    }

    fn delete_counter(&mut self, _id: Id) {
        // no-op
    }

    fn increment_counter_by(&mut self, _id: Id, _delta: u64) {
        // no-op
    }

    fn new_histogram(&mut self, _name: &str, _tags: Tags) -> Id {
        Id::default()
    }

    fn delete_histogram(&mut self, _id: Id) {
        // no-op
    }

    fn record(&mut self, _id: Id, _value: u64) {
        // no-op
    }
}

const NO_OP_METRICS: NoOpMetrics = NoOpMetrics;

const NO_OP_METRICS_VTABLE: MetricsVTable = MetricsVTable {
    new_counter: new_counter_raw::<NoOpMetrics>,
    delete_counter: delete_counter_raw::<NoOpMetrics>,
    increment_counter: increment_counter_raw::<NoOpMetrics>,
    increment_counter_by: increment_counter_by_raw::<NoOpMetrics>,
    new_histogram: new_histogram_raw::<NoOpMetrics>,
    delete_histogram: delete_histogram_raw::<NoOpMetrics>,
    record: record_raw::<NoOpMetrics>,
};

const NO_OP_METRICS_HANDLE: MetricsHandle = MetricsHandle {
    ptr: &NO_OP_METRICS as *const NoOpMetrics as *mut u8,
    vtable: NO_OP_METRICS_VTABLE,
    name: "no-op",
};

struct MetricsHolder {
    handle: AtomicRef<MetricsHandle>,
}

/// Initially set to no-op backend.
static mut METRICS: MetricsHolder = MetricsHolder {
    handle: AtomicRef::new(&NO_OP_METRICS_HANDLE),
};

/// Set a new metrics backend. This should be called as early as possible. Otherwise,
/// all metrics calls will delegate to the `NoOpMetrics`.
pub fn set_metrics(metrics: impl Metrics) {
    #[allow(static_mut_refs)]
    unsafe { &mut METRICS }
        .handle
        .set(Box::leak(Box::new(metrics.into_handle())), Ordering::SeqCst);
}

/// Get name of the active metrics backend.
pub fn get_metrics_backend_name() -> &'static str {
    get_metrics().name
}

struct MetricsVTable {
    new_counter: fn(*mut u8, &str, Tags) -> Id,
    delete_counter: fn(*mut u8, Id),
    increment_counter: fn(*mut u8, Id),
    increment_counter_by: fn(*mut u8, Id, u64),
    new_histogram: fn(*mut u8, &str, Tags) -> Id,
    delete_histogram: fn(*mut u8, Id),
    record: fn(*mut u8, Id, u64),
}

/// Metrics backend handle.
pub struct MetricsHandle {
    ptr: *mut u8,
    vtable: MetricsVTable,
    name: &'static str,
}

impl MetricsHandle {
    #[inline]
    fn new_counter(&mut self, name: &str, tags: Tags) -> Id {
        (self.vtable.new_counter)(self.ptr, name, tags)
    }

    #[inline]
    fn delete_counter(&mut self, id: Id) {
        (self.vtable.delete_counter)(self.ptr, id)
    }

    #[inline]
    fn increment_counter_by(&mut self, id: Id, delta: u64) {
        (self.vtable.increment_counter_by)(self.ptr, id, delta)
    }

    #[inline]
    fn increment_counter(&mut self, id: Id) {
        (self.vtable.increment_counter)(self.ptr, id)
    }

    #[inline]
    fn new_histogram(&mut self, name: &str, tags: Tags) -> Id {
        (self.vtable.new_histogram)(self.ptr, name, tags)
    }

    #[inline]
    fn delete_histogram(&mut self, id: Id) {
        (self.vtable.delete_histogram)(self.ptr, id)
    }

    #[inline]
    fn record(&mut self, id: Id, value: u64) {
        (self.vtable.record)(self.ptr, id, value)
    }
}

struct AtomicRef<T> {
    ptr: AtomicPtr<T>,
}

impl<T> AtomicRef<T> {
    pub const fn new(data: &T) -> Self {
        Self {
            ptr: AtomicPtr::new(data as *const T as *mut T),
        }
    }

    #[inline]
    pub fn get(&self, order: Ordering) -> &T {
        unsafe { &*self.ptr.load(order) }
    }

    #[inline]
    pub fn get_mut(&mut self, order: Ordering) -> &mut T {
        unsafe { &mut *self.ptr.load(order) }
    }

    #[inline]
    pub fn set(&self, new_ref: &T, order: Ordering) {
        self.ptr.store(new_ref as *const T as *mut T, order);
    }
}

mod access {
    use crate::{METRICS, MetricsHandle};
    use std::sync::atomic::Ordering;

    #[allow(static_mut_refs)]
    pub fn get_metrics_mut() -> &'static mut MetricsHandle {
        unsafe { &mut METRICS }.handle.get_mut(Ordering::Acquire)
    }

    #[allow(static_mut_refs)]
    pub fn get_metrics() -> &'static MetricsHandle {
        unsafe { &METRICS }.handle.get(Ordering::Acquire)
    }
}
