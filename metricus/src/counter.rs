use crate::access::get_metrics_mut;
use crate::{Id, Tags};
use std::cell::{LazyCell, UnsafeCell};

pub struct Counter {
    id: Id,
}

impl Counter {
    pub fn new(name: &str, tags: Tags) -> Self {
        let counter_id = get_metrics_mut().new_counter(name, tags);
        Self { id: counter_id }
    }

    pub fn new_with_id(id: Id) -> Self {
        Self { id }
    }
}

impl Drop for Counter {
    fn drop(&mut self) {
        get_metrics_mut().delete_counter(self.id);
    }
}

pub trait CounterOps {
    fn increment(&self);

    fn increment_by(&self, delta: usize);
}

impl CounterOps for Counter {
    fn increment(&self) {
        get_metrics_mut().increment_counter(self.id);
    }

    fn increment_by(&self, delta: usize) {
        get_metrics_mut().increment_counter_by(self.id, delta);
    }
}

impl CounterOps for LazyCell<UnsafeCell<Counter>> {
    fn increment(&self) {
        unsafe { &mut *self.get() }.increment()
    }

    fn increment_by(&self, delta: usize) {
        unsafe { &mut *self.get() }.increment_by(delta)
    }
}
