use metricus_allocator::{enable_allocator_instrumentation, CountingAllocator};

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

fn foo() -> usize {
    Vec::<u8>::with_capacity(1024).capacity()
}

fn main() {
    let task = std::thread::spawn(|| {
        enable_allocator_instrumentation();
        assert_eq!(1024, foo());
    });
    task.join().unwrap();
}
