#![doc = include_str!("../README.md")]

use metricus::{Counter, CounterOps, Id, PreAllocatedMetric};
use std::alloc::{GlobalAlloc, Layout};
use std::sync::{LazyLock, Mutex};

const ALLOC_COUNTER_ID: Id = Id::MAX - 1004;
const ALLOC_BYTES_COUNTER_ID: Id = Id::MAX - 1003;
const DEALLOC_COUNTER_ID: Id = Id::MAX - 1002;
const DEALLOC_BYTES_COUNTER_ID: Id = Id::MAX - 1001;

const fn get_aligned_size(layout: Layout) -> usize {
    let alignment_mask: usize = layout.align() - 1;
    (layout.size() + alignment_mask) & !alignment_mask
}

/// This allocator will use instrumentation to count the number of allocations and de-allocations
/// occurring in the program. All calls to allocate (and free) memory are delegated to the concrete
/// allocator (`std::alloc::System` by default).
///
/// ```no_run
/// use metricus_allocator::CountingAllocator;
///
/// #[global_allocator]
/// static GLOBAL: CountingAllocator = CountingAllocator;
/// ```
pub struct CountingAllocator;

#[allow(static_mut_refs)]
unsafe impl GlobalAlloc for CountingAllocator {
    // `counter_with_id` creates a counter object without registering it.
    // This is used for allocation and de-allocation counters, which are special cases that are initialised before the metrics backend is created.
    // In that case, the `Counter` is created with the `NoOpBackend`, so we defer the registration of the counters until the actual backend is ready.
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        COUNTERS.increment_alloc_count();
        COUNTERS.increment_alloc_bytes(get_aligned_size(layout));

        // delegate to the appropriate allocator
        #[cfg(all(feature = "jemalloc", not(feature = "mimalloc")))]
        {
            return jemallocator::Jemalloc.alloc(layout);
        }
        #[cfg(all(feature = "mimalloc", not(feature = "jemalloc")))]
        {
            return mimalloc::MiMalloc.alloc(layout);
        }
        std::alloc::System.alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        COUNTERS.increment_dealloc_count();
        COUNTERS.increment_dealloc_bytes(get_aligned_size(layout));

        // delegate to the appropriate allocator
        #[cfg(all(feature = "jemalloc", not(feature = "mimalloc")))]
        {
            jemallocator::Jemalloc.dealloc(ptr, layout);
            return;
        }
        #[cfg(all(feature = "mimalloc", not(feature = "jemalloc")))]
        {
            jemallocator::Jemalloc.dealloc(ptr, layout);
            return;
        }
        std::alloc::System.dealloc(ptr, layout)
    }
}

impl CountingAllocator {
    /// Default counters to be used with the `CountingAllocator`.
    pub fn counters() -> Vec<PreAllocatedMetric> {
        vec![
            ("global_allocator".into(), ALLOC_COUNTER_ID, vec![("fn_name".into(), "alloc".into())]),
            ("global_allocator".into(), ALLOC_BYTES_COUNTER_ID, vec![("fn_name".into(), "alloc_bytes".into())]),
            ("global_allocator".into(), DEALLOC_COUNTER_ID, vec![("fn_name".into(), "dealloc".into())]),
            ("global_allocator".into(), DEALLOC_BYTES_COUNTER_ID, vec![("fn_name".into(), "dealloc_bytes".into())]),
        ]
    }
}

static COUNTERS: LazyLock<Counters> = LazyLock::new(|| Counters {
    alloc_count: Counter::new_with_id(ALLOC_COUNTER_ID),
    alloc_bytes: Counter::new_with_id(ALLOC_BYTES_COUNTER_ID),
    dealloc_count: Counter::new_with_id(DEALLOC_COUNTER_ID),
    dealloc_bytes: Counter::new_with_id(DEALLOC_COUNTER_ID),
    inner: Mutex::new(()),
});

struct Counters {
    alloc_count: Counter,
    alloc_bytes: Counter,
    dealloc_count: Counter,
    dealloc_bytes: Counter,
    inner: Mutex<()>,
}

impl Counters {
    #[inline]
    fn increment_alloc_count(&self) {
        let _guard = self.inner.lock().unwrap();
        self.alloc_count.increment();
    }

    #[inline]
    fn increment_alloc_bytes(&self, count: usize) {
        let _guard = self.inner.lock().unwrap();
        self.alloc_bytes.increment_by(count);
    }

    #[inline]
    fn increment_dealloc_count(&self) {
        let _guard = self.inner.lock().unwrap();
        self.dealloc_count.increment();
    }

    #[inline]
    fn increment_dealloc_bytes(&self, count: usize) {
        let _guard = self.inner.lock().unwrap();
        self.dealloc_bytes.increment_by(count);
    }
}
