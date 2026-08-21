#![cfg(feature = "openssl")]

use metricus::{Id, Metrics, Tag, Tags};
use metricus_allocator::{OpenSslCountingAllocator, enable_openssl_allocator_instrumentation};
use std::sync::{LazyLock, Mutex};

static EVENTS: LazyLock<Mutex<Vec<(Id, u64)>>> = LazyLock::new(|| Mutex::new(Vec::new()));

struct RecordingMetrics;

impl Metrics for RecordingMetrics {
    fn name(&self) -> &'static str {
        "recording"
    }

    fn new_counter(&mut self, _name: &str, _tags: &[Tag<'_>]) -> Id {
        unreachable!("OpenSSL allocator counters must be preallocated")
    }

    fn delete_counter(&mut self, _id: Id) {}

    fn increment_counter_by(&mut self, id: Id, delta: u64) {
        EVENTS.lock().unwrap().push((id, delta));
    }

    fn new_histogram(&mut self, _name: &str, _tags: Tags<'_>) -> Id {
        unreachable!()
    }

    fn delete_histogram(&mut self, _id: Id) {}

    fn record(&mut self, _id: Id, _value: u64) {}
}

#[test]
fn records_openssl_allocation_calls_and_requested_bytes() {
    metricus::set_metrics(RecordingMetrics);

    let metrics = OpenSslCountingAllocator::metrics();
    let id_for = |operation: &str| {
        metrics
            .iter()
            .find_map(|metric| match metric {
                metricus::PreAllocatedMetric::Counter { id, tags, .. }
                    if tags.iter().any(|(key, value)| key == "fn_name" && value == operation) =>
                {
                    Some(*id)
                }
                _ => None,
            })
            .unwrap()
    };

    // Explicit early installation followed by enablement exercises the idempotent second install
    // performed by enable_openssl_allocator_instrumentation.
    OpenSslCountingAllocator::install().unwrap();
    enable_openssl_allocator_instrumentation().unwrap();

    // SAFETY: CRYPTO_malloc and CRYPTO_free are paired and the allocation is checked before use.
    unsafe {
        let ptr = openssl_sys::CRYPTO_malloc(40, std::ptr::null(), 0);
        assert!(!ptr.is_null());
        openssl_sys::CRYPTO_free(ptr, std::ptr::null(), 0);
    }

    let events = EVENTS.lock().unwrap();
    assert!(events.contains(&(id_for("alloc"), 1)));
    assert!(events.contains(&(id_for("alloc_bytes"), 40)));
    assert!(events.contains(&(id_for("free"), 1)));
}
