# metricus-allocator

Contains allocator.

## Usage notes

- Call `metricus::set_metrics` before enabling allocator instrumentation if you expect allocation counters to emit.
- Call `enable_allocator_instrumentation` for each thread that should report allocation metrics.

## OpenSSL allocations

Enable the `openssl` feature to include OpenSSL allocations in metricus. OpenSSL's memory callbacks
are process-global and must be installed before any OpenSSL object is created:

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

Call `enable_openssl_allocator_instrumentation` on every thread whose OpenSSL allocations should be
reported. It safely reuses the process-global installation after the first call. Applications that
must install the OpenSSL callbacks before initializing metricus can call
`OpenSslCountingAllocator::install()` earlier and enable instrumentation later. Allocation and
reallocation byte counters contain requested bytes; frees report only the number of calls because
OpenSSL's free callback does not provide the allocation size.
