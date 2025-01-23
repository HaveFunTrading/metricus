use criterion::{black_box, criterion_group, criterion_main, Criterion};

trait Backend {
    fn foo(&mut self, value: usize);

    fn into_backend_handle(self) -> Handle
    where
        Self: Sized,
    {
        let ptr = Box::into_raw(Box::new(self)) as *mut _;
        let vtable = VTable { foo: foo_raw::<Self> };
        Handle { ptr, vtable }
    }
}

struct VTable {
    foo: fn(*mut u8, usize),
}

struct Handle {
    ptr: *mut u8,
    vtable: VTable,
}

impl Handle {
    #[inline]
    fn foo(&mut self, value: usize) {
        (self.vtable.foo)(self.ptr, value);
    }
}

#[inline]
fn foo_raw<T: Backend>(ptr: *mut u8, value: usize) {
    let backend = unsafe { &mut *(ptr as *mut T) };
    backend.foo(value)
}

struct CustomBackend;

impl Backend for CustomBackend {
    #[inline]
    fn foo(&mut self, value: usize) {
        black_box(value);
    }
}

fn benchmark_vtable_dispatch(c: &mut Criterion) {
    let mut handle = CustomBackend.into_backend_handle();

    c.benchmark_group("metrics").bench_function("vtable_dispatch", |b| {
        b.iter(|| {
            handle.foo(1);
        });
    });
}

fn benchmark_dynamic_dispatch(c: &mut Criterion) {
    fn create_backend() -> &'static mut dyn Backend {
        Box::leak(Box::new(CustomBackend))
    }

    let backend = create_backend();

    c.benchmark_group("metrics").bench_function("dynamic_dispatch", |b| {
        b.iter(|| {
            backend.foo(1);
        });
    });
}

fn benchmark_static_dispatch(c: &mut Criterion) {
    struct CustomBackend;

    impl CustomBackend {
        #[inline]
        fn foo(&mut self, value: usize) {
            black_box(value);
        }
    }

    let mut backend = CustomBackend;

    c.benchmark_group("metrics").bench_function("static_dispatch", |b| {
        b.iter(|| {
            backend.foo(1);
        });
    });
}

fn benchmark_enum_dispatch(c: &mut Criterion) {
    enum Backends {
        #[allow(dead_code)]
        Unit,
        Init(CustomBackend),
    }

    impl Backends {
        #[inline]
        fn foo(&mut self, value: usize) {
            match self {
                Backends::Unit => {}
                Backends::Init(backend) => backend.foo(value),
            }
        }
    }

    let mut backend = Backends::Init(CustomBackend);

    c.benchmark_group("metrics").bench_function("enum_dispatch", |b| {
        b.iter(|| {
            backend.foo(1);
        });
    });
}

criterion_group!(
    benches,
    benchmark_vtable_dispatch,
    benchmark_dynamic_dispatch,
    benchmark_static_dispatch,
    benchmark_enum_dispatch
);
criterion_main!(benches);
