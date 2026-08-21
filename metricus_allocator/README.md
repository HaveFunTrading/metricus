# metricus-allocator

Allocation instrumentation for the metricus framework.

The crate provides two independent integrations:

- `CountingAllocator` instruments allocations made through Rust's global allocator.
- `OpenSslCountingAllocator`, enabled with the `openssl` feature, instruments allocations made by
  OpenSSL. OpenSSL does not use Rust's global allocator.

Instrumentation is disabled by default and enabled per thread. Register the corresponding
preallocated metrics before enabling it so the cached counters bind to the intended metricus
backend.

## Rust global allocator

Register `CountingAllocator` as the process's Rust global allocator:

```rust
use metricus_allocator::CountingAllocator;

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;
```

Include its metrics while initializing the agent, then enable instrumentation on each thread whose
allocations should be reported:

```ignore
use metricus_agent::{MetricsAgent, config::MetricsConfig};
use metricus_allocator::{CountingAllocator, enable_allocator_instrumentation};

MetricsAgent::init_with_config(
    MetricsConfig::default()
        .with_pre_allocated_metrics(CountingAllocator::metrics),
)?;
enable_allocator_instrumentation();
```

The `global_allocator` measurement provides `alloc`, `alloc_bytes`, `dealloc`, and `dealloc_bytes`
values through the `fn_name` tag. Byte counters use the allocation layout rounded to its alignment.

## OpenSSL allocator

Enable the integration in `Cargo.toml`:

```toml
metricus_allocator = { version = "0.0.18", features = ["openssl"] }
```

OpenSSL memory callbacks are process-global and must be installed before OpenSSL performs its first
allocation. In the usual initialization sequence, initialize metricus first and enable OpenSSL
instrumentation before constructing any OpenSSL object:

```ignore
use metricus_agent::{MetricsAgent, config::MetricsConfig};
use metricus_allocator::{
    OpenSslCountingAllocator,
    enable_openssl_allocator_instrumentation,
};

MetricsAgent::init_with_config(
    MetricsConfig::default()
        .with_pre_allocated_metrics(OpenSslCountingAllocator::metrics),
)?;
enable_openssl_allocator_instrumentation()?;
```

`enable_openssl_allocator_instrumentation()` installs the callbacks if necessary and enables
reporting for the calling thread. Repeated installation is safe: the first result is cached and
subsequent calls do not invoke OpenSSL again.

### Early installation

If something may initialize OpenSSL before the metricus backend is ready, split installation from
enablement:

```ignore
OpenSslCountingAllocator::install()?;

// Initialize metricus and preallocate OpenSslCountingAllocator::metrics() here.

enable_openssl_allocator_instrumentation()?;
```

The installed callbacks continue forwarding allocations while instrumentation is disabled. A failed
installation means OpenSSL was already used and its allocator can no longer be replaced; that error
is retained by later calls.

The `openssl_allocator` measurement provides:

| `fn_name` | Meaning |
| --- | --- |
| `alloc` | Allocation calls |
| `alloc_bytes` | Requested allocation bytes |
| `realloc` | Reallocation calls |
| `realloc_bytes` | Requested reallocation bytes |
| `free` | Free calls |

Free bytes are intentionally not reported because OpenSSL's free callback does not receive the
allocation size.

The callbacks use libc `malloc`, `realloc`, and `free`; they do not route OpenSSL through Rust's
global allocator. Each enabled operation emits metricus updates synchronously, so measure the
instrumentation overhead before leaving it enabled on a latency-critical thread.

## Threading

Both integrations use thread-local enablement. Call the relevant enable function from every thread
that should report allocations. Ensure the selected metricus backend supports the set of producing
threads used by the application.
