#![doc = include_str!("../README.md")]

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

    fn name(&self) -> &'static str;

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

    fn name(&self) -> &'static str {
        "no-op"
    }

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
        /// Holds either a no-op or the real backend.
        enum Metrics {
            /// Uninitialised metrics backend.
            Uninit($crate::NoOpBackend),
            /// Initialised metrics backend.
            Init($BackendType),
        }

        impl Metrics {
            /// Get active backend name.
            pub fn name(&self) -> &'static str {
                match self {
                    Self::Uninit(backend) => backend.name(),
                    Self::Init(backend) => backend.name(),
                }
            }

            /// Create a new counter using the global metrics backend.
            /// In uninitialized state, this is effectively no-op.
            fn new_counter(&mut self, name: &str, tags: Tags) -> Id {
                match self {
                    Metrics::Uninit(backend) => backend.new_counter(name, tags),
                    Metrics::Init(backend) => backend.new_counter(name, tags),
                }
            }

            /// Delete a counter using the global metrics backend.
            /// In uninitialized state, this is effectively no-op.
            fn delete_counter(&mut self, id: Id) {
                match self {
                    Metrics::Uninit(backend) => backend.delete_counter(id),
                    Metrics::Init(backend) => backend.delete_counter(id),
                }
            }

            /// Increment a counter by 1 using the global metrics backend.
            /// In uninitialized state, this is effectively no-op.
            fn increment_counter(&mut self, id: Id) {
                self.increment_counter_by(id, 1)
            }

            /// Increment a counter by specified delta using the global metrics backend.
            /// In uninitialized state, this is effectively no-op.
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

        /// Get active metrics backend name.
        pub fn get_metrics_backend_name() -> &'static str {
            get_metrics().name()
        }

        pub fn get_metrics() -> &'static Metrics {
            unsafe { &METRICS }
        }

        pub fn get_metrics_mut() -> &'static mut Metrics {
            unsafe { &mut METRICS }
        }

        #[derive(Debug)]
        pub struct Counter {
            id: Id,
        }

        impl Counter {
            /// Creates a new counter with the specified `name` and `tags`.
            ///
            /// # Examples
            ///
            /// ```no_run
            /// use metricus::Counter;
            ///
            /// let tags = [("service", "user"), ("status", "active")];
            /// let counter = Counter::new("user_count", &tags);
            /// ```
            pub fn new(name: &str, tags: Tags) -> Self {
                let counter_id = get_metrics_mut().new_counter(name, tags);
                Self { id: counter_id }
            }
        }
    };
}
