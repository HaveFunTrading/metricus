//! Regression test for the `#[counter]`/`#[span]` concurrent-first-touch race.
//!
//! Before this fix, `metricus_macros` backed each instrumented function
//! with `static mut COUNTER: core::cell::LazyCell<...>`. `LazyCell` is
//! not `Sync` — its init-state tracking is a plain, unsynchronized
//! `Cell`, so two threads calling an instrumented function for the
//! first time at the same instant could race on that state and panic
//! with "LazyCell instance has previously been poisoned". The fix
//! swaps `LazyCell` for `std::sync::LazyLock`, which synchronizes
//! first-touch via a `Once` internally.
//!
//! Each lazy static lives once per compiled function (monomorphized),
//! so reproducing the race needs many *distinct* instrumented functions
//! each hit by many threads released simultaneously — repeating calls
//! to the same function after its first touch is race-free by
//! construction and would never reproduce anything. This test defines
//! many small functions for exactly that reason, and launches every
//! thread across every function before joining any of them, to
//! maximize genuine cross-thread contention on first touch.
//!
//! This is inherently a probabilistic reproduction (real thread
//! scheduling, not a deterministic interleaving), so it errs toward
//! more functions x more threads rather than fewer, to make a false
//! negative (passing on genuinely racy code) very unlikely.

use metricus_macros::{counter, span};
use std::sync::{Arc, Barrier};
use std::thread;

const THREADS_PER_FN: usize = 16;
const FN_COUNT: usize = 16;

macro_rules! define_counter_fns {
    ($($name:ident),* $(,)?) => {
        $(
            #[counter(measurement = "concurrent_first_touch_counter", tags(test = "race"))]
            fn $name() {}
        )*
        const COUNTER_FNS: [fn(); FN_COUNT] = [$($name),*];
    };
}

macro_rules! define_span_fns {
    ($($name:ident),* $(,)?) => {
        $(
            #[span(measurement = "concurrent_first_touch_span", tags(test = "race"))]
            fn $name() {}
        )*
        const SPAN_FNS: [fn(); FN_COUNT] = [$($name),*];
    };
}

define_counter_fns!(
    counter_fn_00,
    counter_fn_01,
    counter_fn_02,
    counter_fn_03,
    counter_fn_04,
    counter_fn_05,
    counter_fn_06,
    counter_fn_07,
    counter_fn_08,
    counter_fn_09,
    counter_fn_10,
    counter_fn_11,
    counter_fn_12,
    counter_fn_13,
    counter_fn_14,
    counter_fn_15,
);

define_span_fns!(
    span_fn_00, span_fn_01, span_fn_02, span_fn_03, span_fn_04, span_fn_05, span_fn_06, span_fn_07, span_fn_08,
    span_fn_09, span_fn_10, span_fn_11, span_fn_12, span_fn_13, span_fn_14, span_fn_15,
);

/// Spawns THREADS_PER_FN threads per function across every function in
/// `fns`, all released via per-function barriers, and joins all of
/// them only after every thread has been spawned -- so contention
/// happens across functions too, not just within one function's group.
fn race_all(fns: &[fn(); FN_COUNT]) {
    let mut handles = Vec::with_capacity(FN_COUNT * THREADS_PER_FN);

    for f in fns {
        let f = *f;
        let barrier = Arc::new(Barrier::new(THREADS_PER_FN));
        for _ in 0..THREADS_PER_FN {
            let barrier = Arc::clone(&barrier);
            handles.push(thread::spawn(move || {
                barrier.wait();
                f();
            }));
        }
    }

    for handle in handles {
        handle.join().expect(
            "thread panicked -- likely the LazyCell concurrent first-touch race \
             (metricus_macros should back #[counter]/#[span] with std::sync::LazyLock, not core::cell::LazyCell)",
        );
    }
}

#[test]
fn concurrent_first_touch_counters_do_not_panic() {
    race_all(&COUNTER_FNS);
}

#[test]
fn concurrent_first_touch_spans_do_not_panic() {
    race_all(&SPAN_FNS);
}
