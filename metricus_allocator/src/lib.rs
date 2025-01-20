#![doc = include_str!("../README.md")]

use metricus::{Counter, CounterOps, Id, PreAllocatedMetric};
use metricus_macros::counter_with_id;
use std::alloc::{GlobalAlloc, Layout};
use std::cell::{LazyCell, UnsafeCell};

const ALLOCATOR_MEASUREMENT_NAME: &str = "global_allocator";
const ALLOC_COUNTER_ID: Id = Id::MAX - 1004;
const ALLOC_TAGS: [(&str, &str); 1] = [("fn_name", "alloc")];
const ALLOC_BYTES_COUNTER_ID: Id = Id::MAX - 1003;
const ALLOC_BYTES_TAGS: [(&str, &str); 1] = [("fn_name", "alloc_bytes")];
const DEALLOC_COUNTER_ID: Id = Id::MAX - 1002;
const DEALLOC_TAGS: [(&str, &str); 1] = [("fn_name", "dealloc")];
const DEALLOC_BYTES_COUNTER_ID: Id = Id::MAX - 1001;
const DEALLOC_BYTES_TAGS: [(&str, &str); 1] = [("fn_name", "dealloc_bytes")];

/// Default counters to be used with the `CountingAllocator`.
pub const DEFAULT_ALLOCATOR_COUNTERS: [PreAllocatedMetric; 4] = [
    (ALLOCATOR_MEASUREMENT_NAME, ALLOC_COUNTER_ID, &ALLOC_TAGS),
    (ALLOCATOR_MEASUREMENT_NAME, ALLOC_BYTES_COUNTER_ID, &ALLOC_BYTES_TAGS),
    (ALLOCATOR_MEASUREMENT_NAME, DEALLOC_COUNTER_ID, &DEALLOC_TAGS),
    (ALLOCATOR_MEASUREMENT_NAME, DEALLOC_BYTES_COUNTER_ID, &DEALLOC_BYTES_TAGS),
];

const fn get_alloc_id() -> Id {
    ALLOC_COUNTER_ID
}

const fn get_dealloc_id() -> Id {
    DEALLOC_COUNTER_ID
}

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

    // FIXME has to be thread safe
    // TODO can we attach thread id and name - so maybe thread_local counter

    #[counter_with_id(id = "get_alloc_id")]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // in addition to the number of allocations count the number of bytes allocated
        static mut BYTE_COUNTER: LazyCell<UnsafeCell<Counter>> =
            LazyCell::new(|| UnsafeCell::new(Counter::new_with_id(ALLOC_BYTES_COUNTER_ID)));
        #[allow(static_mut_refs)]
        unsafe {
            BYTE_COUNTER.increment_by(get_aligned_size(layout));
        }

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

    #[counter_with_id(id = "get_dealloc_id")]
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // in addition to the number of de-allocations count the number of bytes released
        static mut BYTE_COUNTER: LazyCell<UnsafeCell<Counter>> =
            LazyCell::new(|| UnsafeCell::new(Counter::new_with_id(DEALLOC_BYTES_COUNTER_ID)));
        #[allow(static_mut_refs)]
        unsafe {
            BYTE_COUNTER.increment_by(get_aligned_size(layout));
        }

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
