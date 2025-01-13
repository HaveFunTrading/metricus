use metricus::{get_metrics2_mut, init_backend2, BackendHandle, BackendVTable, Id};

#[derive(Debug)]
struct CustomBackend {
    counter: usize,
}

impl From<BackendHandle> for CustomBackend {
    fn from(handle: BackendHandle) -> Self {
        let b = unsafe { Box::from_raw(handle.ptr as *mut CustomBackend) };
        *b
    }
}

impl CustomBackend {
    fn new() -> Self {
        Self { counter: 0 }
    }

    fn into_handle(self) -> BackendHandle {
        let ptr = Box::into_raw(Box::new(self)) as *mut _;
        let vtable = BackendVTable {
            new_counter: Self::new_counter_raw,
        };
        BackendHandle { ptr, vtable }
    }

    unsafe fn new_counter_raw(ptr: *mut u8) -> Id {
        let backend = &mut *(ptr as *mut Self);
        backend.new_counter()
    }

    fn new_counter(&mut self) -> Id {
        Id::default()
    }
}

fn main() {
    init_backend2(CustomBackend::new().into_handle());

    get_metrics2_mut().new_counter();
    get_metrics2_mut().new_counter();
}
