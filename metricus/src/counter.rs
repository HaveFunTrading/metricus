use crate::access::get_metrics_mut;
use crate::{Id, Tags};

pub struct Counter {
    id: Id,
}

impl Counter {
    pub fn new(name: &str, tags: Tags) -> Self {
        let counter_id = get_metrics_mut().new_counter(name, tags);
        Self { id: counter_id }
    }

    pub fn increment(&mut self) {
        get_metrics_mut().increment_counter(self.id);
    }

    pub fn increment_by(&mut self, delta: usize) {
        get_metrics_mut().increment_counter_by(self.id, delta);
    }
}

impl Drop for Counter {
    fn drop(&mut self) {
        get_metrics_mut().delete_counter(self.id);
    }
}
