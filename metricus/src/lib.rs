#![doc = include_str!("../README.md")]

pub type Id = u64;
pub type Tag<'a> = (&'a str, &'a str);
pub type Tags<'a> = &'a [Tag<'a>];
pub type PreAllocatedMetric<'a> = (&'a str, Id, Tags<'a>);

pub const fn empty_tags() -> Tags<'static> {
    &[]
}

pub trait MetricsBackend: Sized {
    type Config: Default;

    fn new() -> Self {
        Self::new_with_config(Self::Config::default())
    }

    fn new_with_config(config: Self::Config) -> Self;

    fn new_counter(&mut self, name: &str, tags: Tags) -> Id;

    fn delete_counter(&mut self, id: Id);

    fn increment_counter_by(&mut self, id: Id, delta: usize);

    fn increment_counter(&mut self, id: Id) {
        self.increment_counter_by(id, 1)
    }
}

/// A trivial no-op backend for the "uninitialized" state.
pub struct NoOpBackend;

impl MetricsBackend for NoOpBackend {
    type Config = ();

    fn new_with_config(_config: Self::Config) -> Self {
        Self
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

#[macro_export]
macro_rules! register_backend {
    ($BackendType:ty) => {
        /// This enum holds either a no-op or the real backend.
        pub enum Metrics {
            Uninit($crate::NoOpBackend),
            Init($BackendType),
        }

        impl Metrics {
            fn new_counter(&mut self, name: &str, tags: Tags) -> Id {
                match self {
                    Metrics::Uninit(backend) => backend.new_counter(name, tags),
                    Metrics::Init(backend) => backend.new_counter(name, tags),
                }
            }

            fn delete_counter(&mut self, id: Id) {
                match self {
                    Metrics::Uninit(backend) => backend.delete_counter(id),
                    Metrics::Init(backend) => backend.delete_counter(id),
                }
            }

            fn increment_counter(&mut self, id: Id) {
                self.increment_counter_by(id, 1)
            }

            fn increment_counter_by(&mut self, id: Id, delta: usize) {
                match self {
                    Metrics::Uninit(backend) => backend.increment_counter_by(id, delta),
                    Metrics::Init(backend) => backend.increment_counter_by(id, delta),
                }
            }
        }

        /// The single global instance of metrics backend.
        static mut METRICS: Metrics = Metrics::Uninit($crate::NoOpBackend);

        /// Call this to initialize the global metrics backend with default config.
        /// Panics if it's already initialized.
        pub fn init_backend() {
            init_backend_with_config(<$BackendType as MetricsBackend>::Config::default())
        }

        /// Call this to initialize the global metrics backend with user supplied config.
        /// Panics if it's already initialized.
        pub fn init_backend_with_config(config: <$BackendType as MetricsBackend>::Config) {
            unsafe {
                match METRICS {
                    Metrics::Uninit(_) => {
                        METRICS = Metrics::Init(<$BackendType>::new_with_config(config));
                    }
                    Metrics::Init(_) => {
                        panic!("Backend is already initialized!");
                    }
                }
            }
        }

        /// Create a new counter using the global metrics backend.
        /// In uninitialized state, this is effectively no-op.
        pub fn new_counter(name: &str, tags: Tags) -> Id {
            unsafe { METRICS.new_counter(name, tags) }
        }

        /// Delete a counter using the global metrics backend.
        /// In uninitialized state, this is effectively no-op.
        pub fn delete_counter(id: Id) {
            unsafe { METRICS.delete_counter(id) }
        }

        /// Increment a counter by specified delta using the global metrics backend.
        /// In uninitialized state, this is effectively no-op.
        pub fn increment_counter_by(id: Id, delta: usize) {
            unsafe { METRICS.increment_counter_by(id, delta) }
        }

        /// Increment a counter by 1 using the global metrics backend.
        /// In uninitialized state, this is effectively no-op.
        pub fn increment_counter(id: Id) {
            increment_counter_by(id, 1)
        }
    };
}
