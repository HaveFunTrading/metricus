#![doc = include_str!("../README.md")]

pub trait MetricsBackend : Sized {

    type Config: Default;

    fn new() -> Self {
        Self::new_with_config(Self::Config::default())
    }

    fn new_with_config(config: Self::Config) -> Self;


    fn create_counter(&mut self) -> u64;
}

/// A trivial no-op backend for the "uninitialized" state.
pub struct NoOpBackend;

impl MetricsBackend for NoOpBackend {
    type Config = ();

    fn new_with_config(_config: Self::Config) -> Self {
        Self
    }

    fn create_counter(&mut self) -> u64 {
        println!("[NoOpBackend] Create counter");
        0
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

        impl Default for Metrics {
            fn default() -> Self {
                Metrics::Uninit($crate::NoOpBackend)
            }
        }

        impl Metrics {
            fn create_counter(&mut self) -> u64 {
                match self {
                    Metrics::Uninit(backend) => backend.create_counter(),
                    Metrics::Init(backend)   => backend.create_counter(),
                }
            }
        }

        /// The single global instance of metrics backend.
        static mut METRICS: Metrics = Metrics::Uninit($crate::NoOpBackend);

        /// Call this to initialize the global metrics backend. with default config.
        /// Panics if it's already initialized.
        pub fn init_backend() {
            init_backend_with_config(<$BackendType as MetricsBackend>::Config::default())
        }

        /// Call this to initialize the global metrics backend. with user supplied config.
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

        /// Create a counter using the global metrics backend.
        /// In uninitialized state, this is effectively no-op.
        pub fn create_counter() -> u64 {
            unsafe {
                METRICS.create_counter()
            }
        }
    }
}
