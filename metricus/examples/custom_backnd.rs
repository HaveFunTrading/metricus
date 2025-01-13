use metricus::{register_backend, MetricsBackend};

register_backend!(CustomBackend);

struct CustomBackend;

#[derive(Default)]
struct CustomBackendConfig;

impl MetricsBackend for CustomBackend {
    type Config = CustomBackendConfig;

    fn new_with_config(_config: Self::Config) -> Self {
        Self
    }

    fn create_counter(&mut self) -> u64 {
        println!("[CustomBackend] Create counter");
        1
    }
}

fn main() {
    init_backend_with_config(CustomBackendConfig::default());

    create_counter();
    create_counter();
    create_counter();
}