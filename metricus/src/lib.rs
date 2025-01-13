#![doc = include_str!("../README.md")]

use std::ops::{Deref, DerefMut};

pub type Id = u64;
pub type Tag<'a> = (&'a str, &'a str);
pub type Tags<'a> = &'a [Tag<'a>];
pub type PreAllocatedMetric<'a> = (&'a str, Id, Tags<'a>);

/// Returns empty tags.
pub const fn empty_tags() -> Tags<'static> {
    &[]
}

pub trait MetricsBackend: Sized {
    type Config: Default;

    fn new() -> Self {
        Self::new_with_config(Self::Config::default())
    }

    fn new_with_config(config: Self::Config) -> Self;

    fn into_handle(self) -> BackendHandle {
        let ptr = Box::into_raw(Box::new(self)) as *mut _;
        let vtable = BackendVTable {
            new_counter: new_counter_raw::<Self>,
            delete_counter: delete_counter_raw::<Self>,
            increment_counter: increment_counter_raw::<Self>,
            increment_counter_by: increment_counter_by_raw::<Self>,
        };
        BackendHandle { ptr, vtable }
    }

    fn name(&self) -> &'static str;

    fn new_counter(&mut self, name: &str, tags: Tags) -> Id;

    fn delete_counter(&mut self, id: Id);

    fn increment_counter_by(&mut self, id: Id, delta: usize);

    fn increment_counter(&mut self, id: Id) {
        self.increment_counter_by(id, 1)
    }
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

/// A trivial no-op backend for the "uninitialized" state.
struct NoOpBackend;

impl MetricsBackend for NoOpBackend {
    type Config = ();

    fn new_with_config(_config: Self::Config) -> Self {
        Self
    }

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
}

const NO_OP_BACKEND: NoOpBackend = NoOpBackend;

const NO_OP_BACKEND_VTABLE: BackendVTable = BackendVTable {
    new_counter: new_counter_raw::<NoOpBackend>,
    delete_counter: delete_counter_raw::<NoOpBackend>,
    increment_counter: increment_counter_raw::<NoOpBackend>,
    increment_counter_by: increment_counter_by_raw::<NoOpBackend>,
};

const NO_OP_BACKEND_HANDLE: BackendHandle = BackendHandle {
    ptr: &NO_OP_BACKEND as *const NoOpBackend as *mut u8,
    vtable: NO_OP_BACKEND_VTABLE,
};

struct BackendVTable {
    pub new_counter: fn(*mut u8, &str, Tags) -> Id,
    pub delete_counter: fn(*mut u8, Id),
    pub increment_counter: fn(*mut u8, Id),
    pub increment_counter_by: fn(*mut u8, Id, usize),
}

pub struct BackendHandle {
    ptr: *mut u8,
    vtable: BackendVTable,
}

impl BackendHandle {
    pub fn new_counter(&mut self, name: &str, tags: Tags) -> Id {
        (self.vtable.new_counter)(self.ptr, name, tags)
    }

    pub fn delete_counter(&mut self, id: Id) {
        (self.vtable.delete_counter)(self.ptr, id)
    }

    pub fn increment_counter_by(&mut self, id: Id, delta: usize) {
        (self.vtable.increment_counter_by)(self.ptr, id, delta)
    }

    pub fn increment_counter(&mut self, id: Id) {
        (self.vtable.increment_counter)(self.ptr, id)
    }
}

pub struct Metrics {
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

static mut METRICS: Metrics = Metrics {
    handle: NO_OP_BACKEND_HANDLE,
};

pub fn set_backend(handle: BackendHandle) {
    unsafe { METRICS.handle = handle };
}

#[allow(static_mut_refs)]
pub fn get_metrics_mut() -> &'static mut Metrics {
    unsafe { &mut METRICS }
}

#[allow(static_mut_refs)]
pub fn get_metrics() -> &'static Metrics {
    unsafe { &METRICS }
}
