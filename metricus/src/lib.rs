#![doc = include_str!("../README.md")]

mod counter;
mod histogram;

use std::arch::x86_64::_mm_mfence;
use crate::access::{get_metrics, get_metrics_mut};
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{fence, Ordering};
// re-exports
pub use counter::{Counter, CounterOps};
pub use histogram::{Histogram, HistogramOps};

/// Metric id.
pub type Id = u64;
/// Metric tag expresses as key-value pair.
pub type Tag<'a> = (&'a str, &'a str);
/// Metrics tags expresses as array of key-value pairs.
pub type Tags<'a> = &'a [Tag<'a>];
/// Pre-allocated metric consists of name, id and tags.
pub type PreAllocatedMetric = (String, Id, Vec<(String, String)>);

/// Returns empty tags.
pub const fn empty_tags() -> Tags<'static> {
    &[]
}

/// Common interface for metrics backend. Each new backend must implement this trait.
pub trait MetricsBackend: Sized {
    fn into_backend_handle(self) -> BackendHandle {
        let name = self.name();
        let ptr = Box::into_raw(Box::new(self)) as *mut _;
        let vtable = BackendVTable {
            new_counter: new_counter_raw::<Self>,
            delete_counter: delete_counter_raw::<Self>,
            increment_counter: increment_counter_raw::<Self>,
            increment_counter_by: increment_counter_by_raw::<Self>,
            new_histogram: new_histogram_raw::<Self>,
            delete_histogram: delete_histogram_raw::<Self>,
            record: record_raw::<Self>,
        };
        BackendHandle { ptr, vtable, name }
    }

    fn name(&self) -> &'static str;

    fn new_counter(&mut self, name: &str, tags: Tags) -> Id;

    fn delete_counter(&mut self, id: Id);

    fn increment_counter_by(&mut self, id: Id, delta: usize);

    fn increment_counter(&mut self, id: Id) {
        self.increment_counter_by(id, 1)
    }

    fn new_histogram(&mut self, name: &str, tags: Tags) -> Id;

    fn delete_histogram(&mut self, id: Id);

    fn record(&mut self, id: Id, value: u64);
}

fn new_counter_raw<T: MetricsBackend>(ptr: *mut u8, name: &str, tags: Tags) -> Id {
    let backend = unsafe { &mut *(ptr as *mut T) };
    backend.new_counter(name, tags)
}

fn delete_counter_raw<T: MetricsBackend>(ptr: *mut u8, id: Id) {
    let backend = unsafe { &mut *(ptr as *mut T) };
    backend.delete_counter(id)
}

fn increment_counter_by_raw<T: MetricsBackend>(ptr: *mut u8, id: Id, delta: usize) {
    let backend = unsafe { &mut *(ptr as *mut T) };
    backend.increment_counter_by(id, delta)
}

fn increment_counter_raw<T: MetricsBackend>(ptr: *mut u8, id: Id) {
    increment_counter_by_raw::<T>(ptr, id, 1)
}

fn new_histogram_raw<T: MetricsBackend>(ptr: *mut u8, name: &str, tags: Tags) -> Id {
    let backend = unsafe { &mut *(ptr as *mut T) };
    backend.new_histogram(name, tags)
}

fn delete_histogram_raw<T: MetricsBackend>(ptr: *mut u8, id: Id) {
    let backend = unsafe { &mut *(ptr as *mut T) };
    backend.delete_histogram(id)
}

fn record_raw<T: MetricsBackend>(ptr: *mut u8, id: Id, value: u64) {
    let backend = unsafe { &mut *(ptr as *mut T) };
    backend.record(id, value)
}

/// A trivial no-op backend for the "uninitialized" state.
struct NoOpBackend;

impl MetricsBackend for NoOpBackend {
    fn name(&self) -> &'static str {
        "no-op"
    }

    fn new_counter(&mut self, _name: &str, _tags: Tags) -> Id {
        Id::default()
    }

    fn delete_counter(&mut self, _id: Id) {
        // no-op
    }

    fn increment_counter_by(&mut self, _id: Id, _delta: usize) {
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

const NO_OP_BACKEND: NoOpBackend = NoOpBackend;

const NO_OP_BACKEND_VTABLE: BackendVTable = BackendVTable {
    new_counter: new_counter_raw::<NoOpBackend>,
    delete_counter: delete_counter_raw::<NoOpBackend>,
    increment_counter: increment_counter_raw::<NoOpBackend>,
    increment_counter_by: increment_counter_by_raw::<NoOpBackend>,
    new_histogram: new_histogram_raw::<NoOpBackend>,
    delete_histogram: delete_histogram_raw::<NoOpBackend>,
    record: record_raw::<NoOpBackend>,
};

const NO_OP_BACKEND_HANDLE: BackendHandle = BackendHandle {
    ptr: &NO_OP_BACKEND as *const NoOpBackend as *mut u8,
    vtable: NO_OP_BACKEND_VTABLE,
    name: "no-op",
};

struct Metrics {
    handle: BackendHandle,
}

impl Deref for Metrics {
    type Target = BackendHandle;

    fn deref(&self) -> &Self::Target {
        &self.handle
    }
}

impl DerefMut for Metrics {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.handle
    }
}

/// Initially set to no-op backend.
static mut METRICS: Metrics = Metrics {
    handle: NO_OP_BACKEND_HANDLE,
};

/// Set a new metrics backend. This should be called as early as possible. Otherwise,
/// all metrics calls will delegate to the `NoOpBackend`.
pub fn set_backend(backend: impl MetricsBackend) {
    get_metrics_mut().handle = backend.into_backend_handle();
}

/// Get name of the active metrics backend.
pub fn get_backend_name() -> &'static str {
    get_metrics().handle.name
}

struct BackendVTable {
    new_counter: fn(*mut u8, &str, Tags) -> Id,
    delete_counter: fn(*mut u8, Id),
    increment_counter: fn(*mut u8, Id),
    increment_counter_by: fn(*mut u8, Id, usize),
    new_histogram: fn(*mut u8, &str, Tags) -> Id,
    delete_histogram: fn(*mut u8, Id),
    record: fn(*mut u8, Id, u64),
}

/// Metrics backend handle.
pub struct BackendHandle {
    ptr: *mut u8,
    vtable: BackendVTable,
    name: &'static str,
}

impl BackendHandle {
    fn new_counter(&mut self, name: &str, tags: Tags) -> Id {
        (self.vtable.new_counter)(self.ptr, name, tags)
    }

    fn delete_counter(&mut self, id: Id) {
        (self.vtable.delete_counter)(self.ptr, id)
    }

    fn increment_counter_by(&mut self, id: Id, delta: usize) {
        (self.vtable.increment_counter_by)(self.ptr, id, delta)
    }

    fn increment_counter(&mut self, id: Id) {
        (self.vtable.increment_counter)(self.ptr, id)
    }

    fn new_histogram(&mut self, name: &str, tags: Tags) -> Id {
        (self.vtable.new_histogram)(self.ptr, name, tags)
    }

    fn delete_histogram(&mut self, id: Id) {
        (self.vtable.delete_histogram)(self.ptr, id)
    }

    fn record(&mut self, id: Id, value: u64) {
        (self.vtable.record)(self.ptr, id, value)
    }
}

mod access {
    use crate::{Metrics, METRICS};

    #[allow(static_mut_refs)]
    pub fn get_metrics_mut() -> &'static mut Metrics {
        unsafe { &mut METRICS }
    }

    #[allow(static_mut_refs)]
    pub fn get_metrics() -> &'static Metrics {
        unsafe { &METRICS }
    }
}
