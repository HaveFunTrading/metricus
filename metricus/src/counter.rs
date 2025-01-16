//! A `Counter` proxy struct for managing a metrics counter.

use crate::access::get_metrics_mut;
use crate::{Id, Tags};
use std::cell::{LazyCell, UnsafeCell};

/// Provides methods to create a new counter, increment it, and
/// increment it by a specified amount. It automatically deletes the counter
/// when it is dropped.
///
/// ## Examples
///
/// You can create a counter, increment it, and increment it by a specific value.
///
/// ```no_run
/// use metricus::{Counter, CounterOps};
///
/// let tags = [("service", "payment"), ("currency", "USD")];
/// let counter = Counter::new("transaction_count", &tags);
///
/// counter.increment();
/// counter.increment_by(5);
/// ```
///
/// Another option is to use `#[counter]` macro to instrument your code to automatically create
/// a static `Counter` for you.
///
/// ```no_run
/// use metricus_macros::counter;
///
/// #[counter(measurement = "counters", tags(key1 = "value1", key2 = "value2"))]
/// fn my_function_with_tags() {
///     // function body
/// }
///
/// my_function_with_tags();
/// ````
#[derive(Debug)]
pub struct Counter {
    id: Id,
}

impl Counter {
    /// Creates a new counter with the specified `name` and `tags`.
    ///
    /// ## Examples
    ///
    /// Create a counter with tags.
    /// ```no_run
    /// use metricus::Counter;
    ///
    /// let tags = [("service", "user"), ("status", "active")];
    /// let counter = Counter::new("user_count", &tags);
    /// ```
    ///
    /// Create a counter without tags.
    /// ```no_run
    /// use metricus::{empty_tags, Counter};
    ///
    /// let counter = Counter::new("user_count", empty_tags());
    /// ```
    pub fn new(name: &str, tags: Tags) -> Self {
        let counter_id = get_metrics_mut().new_counter(name, tags);
        Self { id: counter_id }
    }

    /// Create a counter object without registering it.
    /// This creates a new counter proxy that assumes the metrics backend has already created the counter.
    ///
    /// ## Examples
    ///
    /// Create a counter with specific id.
    ///
    /// ```no_run
    /// use metricus::Counter;
    ///
    /// let counter = Counter::new_with_id(1);
    /// ```
    pub fn new_with_id(id: Id) -> Self {
        Self { id }
    }
}

impl Drop for Counter {
    fn drop(&mut self) {
        get_metrics_mut().delete_counter(self.id);
    }
}

/// Defines a series of operations that can be performed on a `Counter`.
pub trait CounterOps {
    /// Increments the counter by 1.
    ///
    /// ## Examples
    ///
    /// ```no_run
    /// use metricus::{Counter, CounterOps};
    ///
    /// let counter = Counter::new("example_counter", &[]);
    /// counter.increment();
    /// ```
    fn increment(&self);

    /// Increments the counter by a specified amount.
    ///
    /// ## Examples
    ///
    /// ```
    /// use metricus::{Counter, CounterOps};
    ///
    /// let counter = Counter::new("example_counter", &[]);
    /// counter.increment_by(5);
    /// ```
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
