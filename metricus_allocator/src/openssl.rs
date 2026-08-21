//! OpenSSL allocation instrumentation for metricus.
//!
//! OpenSSL does not allocate through Rust's [`GlobalAlloc`](std::alloc::GlobalAlloc). This module
//! installs process-global callbacks with `CRYPTO_set_mem_functions` so OpenSSL allocation activity
//! can be reported separately from [`CountingAllocator`](crate::CountingAllocator).
//!
//! # Lifecycle
//!
//! OpenSSL only accepts custom memory functions before its first allocation. There are two supported
//! initialization sequences:
//!
//! - Most applications initialize their metricus backend and then call
//!   [`enable_openssl_allocator_instrumentation`]. It installs the callbacks if necessary and enables
//!   reporting for the calling thread.
//! - Applications that might touch OpenSSL before metricus is ready call
//!   [`OpenSslCountingAllocator::install`] as early as possible. Allocations continue to be forwarded
//!   without reporting until instrumentation is enabled later.
//!
//! Installation is idempotent and process-global. Enablement is thread-local and must happen after
//! the metricus backend and the metrics returned by [`OpenSslCountingAllocator::metrics`] have been
//! initialized.
//!
//! # Metrics
//!
//! The `openssl_allocator` measurement reports `alloc`, `alloc_bytes`, `realloc`, `realloc_bytes`,
//! and `free`, distinguished by the `fn_name` tag. Byte counters contain the requested allocation
//! size. Only the number of frees is reported because OpenSSL's free callback does not receive a
//! size.
//!
//! # Allocation behavior
//!
//! The callbacks forward to the corresponding libc allocation functions. They instrument OpenSSL;
//! they do not route OpenSSL allocations through Rust's configured global allocator. Metric updates
//! are emitted synchronously on each enabled allocation operation, so instrumentation has a small
//! hot-path cost.

use metricus::{Counter, CounterOps, Id, PreAllocatedMetric};
use std::cell::Cell;
use std::ffi::{c_char, c_int, c_void};
use std::fmt::{Display, Formatter};
use std::sync::{LazyLock, OnceLock};

const ALLOC_COUNTER_ID: Id = Id::MAX - 1010;
const ALLOC_BYTES_COUNTER_ID: Id = Id::MAX - 1009;
const REALLOC_COUNTER_ID: Id = Id::MAX - 1008;
const REALLOC_BYTES_COUNTER_ID: Id = Id::MAX - 1007;
const FREE_COUNTER_ID: Id = Id::MAX - 1006;

type CryptoMallocFn = unsafe extern "C" fn(usize, *const c_char, c_int) -> *mut c_void;
type CryptoReallocFn = unsafe extern "C" fn(*mut c_void, usize, *const c_char, c_int) -> *mut c_void;
type CryptoFreeFn = unsafe extern "C" fn(*mut c_void, *const c_char, c_int);

unsafe extern "C" {
    fn CRYPTO_set_mem_functions(
        malloc_fn: Option<CryptoMallocFn>,
        realloc_fn: Option<CryptoReallocFn>,
        free_fn: Option<CryptoFreeFn>,
    ) -> c_int;
}

/// Installs metricus-aware allocation callbacks for OpenSSL.
///
/// OpenSSL memory callbacks are process-global and must be installed before OpenSSL performs its
/// first allocation. Instrumentation remains disabled until
/// [`enable_openssl_allocator_instrumentation`] is called on a thread.
pub struct OpenSslCountingAllocator;

impl OpenSslCountingAllocator {
    /// Metrics emitted by the OpenSSL allocation callbacks.
    ///
    /// Pass this function to `MetricsConfig::with_pre_allocated_metrics` before initializing the
    /// metrics backend.
    pub fn metrics() -> Vec<PreAllocatedMetric> {
        vec![
            PreAllocatedMetric::counter("openssl_allocator", ALLOC_COUNTER_ID, &[("fn_name", "alloc")]),
            PreAllocatedMetric::counter("openssl_allocator", ALLOC_BYTES_COUNTER_ID, &[("fn_name", "alloc_bytes")]),
            PreAllocatedMetric::counter("openssl_allocator", REALLOC_COUNTER_ID, &[("fn_name", "realloc")]),
            PreAllocatedMetric::counter("openssl_allocator", REALLOC_BYTES_COUNTER_ID, &[("fn_name", "realloc_bytes")]),
            PreAllocatedMetric::counter("openssl_allocator", FREE_COUNTER_ID, &[("fn_name", "free")]),
        ]
    }

    /// Install the allocation callbacks once for this process.
    ///
    /// Repeated successful calls are harmless. Installation fails if OpenSSL has already performed
    /// an allocation, because OpenSSL does not permit replacing its allocator after first use. The
    /// first result, including failure, is cached for all subsequent calls.
    ///
    /// This method does not enable reporting and does not require the metricus backend to be ready.
    /// Use it for early installation when OpenSSL might otherwise be initialized before metricus.
    pub fn install() -> Result<(), OpenSslAllocatorInstallError> {
        *INSTALL_RESULT.get_or_init(|| {
            // Referencing an openssl-sys symbol ensures its native link metadata is retained while
            // the callback-setting function, which openssl-sys does not expose, is declared above.
            let _ = openssl_sys::CRYPTO_malloc;

            // SAFETY: all callbacks have OpenSSL's required C ABI and remain valid for the lifetime
            // of the process. Each callback returns memory obtained from, and releases it through,
            // the same libc allocator family.
            let installed =
                unsafe { CRYPTO_set_mem_functions(Some(openssl_malloc), Some(openssl_realloc), Some(openssl_free)) };

            if installed == 1 {
                Ok(())
            } else {
                Err(OpenSslAllocatorInstallError)
            }
        })
    }
}

/// OpenSSL rejected custom allocation callbacks, normally because it has already allocated memory.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OpenSslAllocatorInstallError;

impl Display for OpenSslAllocatorInstallError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter
            .write_str("OpenSSL rejected custom memory functions; install them before the first OpenSSL allocation")
    }
}

impl std::error::Error for OpenSslAllocatorInstallError {}

thread_local! {
    static INSTRUMENTATION_ENABLED: Cell<bool> = const { Cell::new(false) };
}

/// Enable OpenSSL allocation instrumentation for the calling thread.
///
/// This installs the process-global OpenSSL allocation callbacks if necessary, then binds the
/// counters to the active metricus backend. Initialize the metrics backend before calling this
/// function and include [`OpenSslCountingAllocator::metrics`] in its preallocated metrics.
///
/// OpenSSL allocations on other threads are not reported until this function is called from those
/// threads as well. The custom allocator callbacks remain installed process-wide regardless.
pub fn enable_openssl_allocator_instrumentation() -> Result<(), OpenSslAllocatorInstallError> {
    OpenSslCountingAllocator::install()?;
    LazyLock::force(&COUNTERS);
    INSTRUMENTATION_ENABLED.set(true);
    Ok(())
}

static INSTALL_RESULT: OnceLock<Result<(), OpenSslAllocatorInstallError>> = OnceLock::new();

static COUNTERS: LazyLock<Counters> = LazyLock::new(|| Counters {
    alloc_count: Counter::new_with_id(ALLOC_COUNTER_ID),
    alloc_bytes: Counter::new_with_id(ALLOC_BYTES_COUNTER_ID),
    realloc_count: Counter::new_with_id(REALLOC_COUNTER_ID),
    realloc_bytes: Counter::new_with_id(REALLOC_BYTES_COUNTER_ID),
    free_count: Counter::new_with_id(FREE_COUNTER_ID),
});

struct Counters {
    alloc_count: Counter,
    alloc_bytes: Counter,
    realloc_count: Counter,
    realloc_bytes: Counter,
    free_count: Counter,
}

unsafe extern "C" fn openssl_malloc(size: usize, _file: *const c_char, _line: c_int) -> *mut c_void {
    if INSTRUMENTATION_ENABLED.get() {
        COUNTERS.alloc_count.increment();
        COUNTERS.alloc_bytes.increment_by(size as u64);
    }

    // SAFETY: forwarding OpenSSL's allocation request to libc. OpenSSL receives the returned pointer
    // unchanged and will return it to openssl_realloc or openssl_free.
    unsafe { libc::malloc(size) }
}

unsafe extern "C" fn openssl_realloc(ptr: *mut c_void, size: usize, _file: *const c_char, _line: c_int) -> *mut c_void {
    if INSTRUMENTATION_ENABLED.get() {
        COUNTERS.realloc_count.increment();
        COUNTERS.realloc_bytes.increment_by(size as u64);
    }

    // SAFETY: ptr is null or was allocated by this callback family, and is passed back to libc's
    // corresponding realloc implementation.
    unsafe { libc::realloc(ptr, size) }
}

unsafe extern "C" fn openssl_free(ptr: *mut c_void, _file: *const c_char, _line: c_int) {
    if INSTRUMENTATION_ENABLED.get() {
        COUNTERS.free_count.increment();
    }

    // SAFETY: ptr is null or was allocated by this callback family.
    unsafe { libc::free(ptr) }
}
