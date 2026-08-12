#![doc = include_str!("../README.md")]

mod affinity;
mod aggregator;
pub mod config;
mod error;
mod exporter;

use crate::aggregator::MetricsAggregator;
use crate::config::MetricsConfig;
use metricus::{Id, Metrics, PreAllocatedMetric, Tag, Tags, set_metrics};
#[cfg(feature = "rtrb")]
use rtrb::Producer;
#[cfg(not(feature = "rtrb"))]
use std::sync::mpsc::SyncSender;

// re-exports
pub use error::{Error, Result};
use std::collections::HashMap;

type OwnedTag = (String, String);
type OwnedTags = Vec<OwnedTag>;

trait ToOwnedTags {
    fn to_owned_tags(self) -> OwnedTags;
}

impl ToOwnedTags for Tags<'_> {
    fn to_owned_tags(self) -> OwnedTags {
        self.iter().map(|tag| tag.to_owned_tag()).collect()
    }
}

trait ToOwnedTag {
    fn to_owned_tag(self) -> OwnedTag;
}

impl ToOwnedTag for Tag<'_> {
    fn to_owned_tag(self) -> OwnedTag {
        (self.0.to_owned(), self.1.to_owned())
    }
}

pub struct MetricsAgent {
    #[cfg(feature = "rtrb")]
    tx_upd: Producer<UpdateEvent>,
    #[cfg(feature = "rtrb")]
    tx_cnc: Producer<ControlEvent>,
    #[cfg(not(feature = "rtrb"))]
    tx_upd: SyncSender<UpdateEvent>,
    #[cfg(not(feature = "rtrb"))]
    tx_cnc: SyncSender<ControlEvent>,
    default_tags: OwnedTags,
    next_id: Id,
    metric_key_to_id: HashMap<MetricKey, Id>,
}

impl MetricsAgent {
    /// Init agent with default config.
    pub fn init() -> Result<()> {
        Self::init_with_config(MetricsConfig::default())
    }

    /// Init agent with user supplied config.
    pub fn init_with_config(config: MetricsConfig) -> Result<()> {
        #[cfg(feature = "rtrb")]
        let (mut tx_upd, rx_upd) = rtrb::RingBuffer::new(config.event_channel_size);
        #[cfg(feature = "rtrb")]
        let (mut tx_cnc, rx_cnc) = rtrb::RingBuffer::new(1024);
        #[cfg(not(feature = "rtrb"))]
        let (tx_upd, rx_upd) = std::sync::mpsc::sync_channel(config.event_channel_size);
        #[cfg(not(feature = "rtrb"))]
        let (tx_cnc, rx_cnc) = std::sync::mpsc::sync_channel(1024);

        #[cfg(feature = "rtrb")]
        if config.prefault_ring {
            prefault_ring(&mut tx_upd);
            prefault_ring(&mut tx_cnc);
        }

        // launch aggregator on background thread
        let _ = MetricsAggregator::start_on_thread(rx_upd, rx_cnc, config.clone());

        let mut agent = MetricsAgent::new(tx_upd, tx_cnc, config.default_tags);
        for metric in config.pre_allocated_metrics {
            agent.register_metric_with_id(metric);
        }

        set_metrics(agent);
        Ok(())
    }

    #[cfg(feature = "rtrb")]
    fn new(tx_upd: Producer<UpdateEvent>, tx_cnc: Producer<ControlEvent>, default_tags: OwnedTags) -> Self {
        Self {
            tx_upd,
            tx_cnc,
            default_tags,
            next_id: 0,
            metric_key_to_id: Default::default(),
        }
    }

    #[cfg(not(feature = "rtrb"))]
    fn new(tx_upd: SyncSender<UpdateEvent>, tx_cnc: SyncSender<ControlEvent>, default_tags: OwnedTags) -> Self {
        Self {
            tx_upd,
            tx_cnc,
            default_tags,
            next_id: 0,
            metric_key_to_id: Default::default(),
        }
    }

    #[inline]
    fn assign_next_id(&mut self, name: &str, tags: OwnedTags) -> Id {
        *self
            .metric_key_to_id
            .entry(MetricKey::new(name, tags))
            .or_insert_with(|| {
                let id = self.next_id;
                self.next_id += 1;
                id
            })
    }

    #[inline]
    fn send_control_event(&mut self, event: ControlEvent) {
        #[cfg(feature = "rtrb")]
        let _ = self.tx_cnc.push(event);
        #[cfg(not(feature = "rtrb"))]
        let _ = self.tx_cnc.try_send(event);
    }

    #[inline]
    fn send_update_event(&mut self, event: UpdateEvent) {
        #[cfg(feature = "rtrb")]
        let _ = self.tx_upd.push(event);
        #[cfg(not(feature = "rtrb"))]
        let _ = self.tx_upd.try_send(event);
    }

    fn enrich_with_counter_tags(&self, tags: &mut OwnedTags) {
        tags.push(("type", "counter").to_owned_tag());
        tags.extend(self.default_tags.clone());
        tags.sort();
        tags.dedup();
    }

    fn enrich_with_histogram_tags(&self, tags: &mut OwnedTags) {
        tags.push(("type", "histogram").to_owned_tag());
        tags.extend(self.default_tags.clone());
        tags.sort();
        tags.dedup();
    }

    fn register_metric_with_id(&mut self, metric: PreAllocatedMetric) {
        match metric {
            PreAllocatedMetric::Counter { name, id, mut tags } => {
                self.enrich_with_counter_tags(&mut tags);
                self.send_control_event(ControlEvent::CounterCreate(id, name, tags))
            }
            PreAllocatedMetric::Histogram { name, id, mut tags } => {
                self.enrich_with_histogram_tags(&mut tags);
                self.send_control_event(ControlEvent::HistogramCreate(id, name, tags))
            }
        }
    }
}

impl Metrics for MetricsAgent {
    fn name(&self) -> &'static str {
        "metrics-agent"
    }

    fn new_counter(&mut self, name: &str, tags: Tags) -> Id {
        let mut tags = tags.to_owned_tags();
        self.enrich_with_counter_tags(&mut tags);
        let id = self.assign_next_id(name, tags.clone());
        self.send_control_event(ControlEvent::CounterCreate(id, name.to_owned(), tags));
        id
    }

    fn delete_counter(&mut self, id: Id) {
        self.send_control_event(ControlEvent::CounterDelete(id));
    }

    #[inline]
    fn increment_counter_by(&mut self, id: Id, delta: u64) {
        self.send_update_event(UpdateEvent::CounterIncrement(id, delta));
    }

    fn new_histogram(&mut self, name: &str, tags: Tags) -> Id {
        let mut tags = tags.to_owned_tags();
        self.enrich_with_histogram_tags(&mut tags);
        let id = self.assign_next_id(name, tags.clone());
        self.send_control_event(ControlEvent::HistogramCreate(id, name.to_owned(), tags));
        id
    }

    fn delete_histogram(&mut self, id: Id) {
        self.send_control_event(ControlEvent::HistogramDelete(id));
    }

    #[inline]
    fn record(&mut self, id: Id, value: u64) {
        self.send_update_event(UpdateEvent::HistogramRecord(id, value));
    }
}

#[derive(Debug)]
enum ControlEvent {
    CounterCreate(Id, String, OwnedTags),
    CounterDelete(Id),
    HistogramCreate(Id, String, OwnedTags),
    HistogramDelete(Id),
}

#[derive(Debug)]
enum UpdateEvent {
    CounterIncrement(Id, u64),
    HistogramRecord(Id, u64),
}

#[derive(Eq, PartialEq, Hash, Clone)]
struct MetricKey {
    name: String,
    tags: OwnedTags,
}

impl MetricKey {
    fn new(name: &str, tags: OwnedTags) -> Self {
        Self {
            name: name.to_owned(),
            tags,
        }
    }
}

#[cfg(feature = "rtrb")]
fn prefault_ring<T>(producer: &mut Producer<T>) {
    let capacity = producer.buffer().capacity();
    let mut chunk = producer
        .write_chunk_uninit(capacity)
        .expect("a newly created rtrb must have all slots available");
    let (first, second) = chunk.as_mut_slices();

    for slot in first.iter_mut().chain(second.iter_mut()) {
        // SAFETY: `slot` points to writable `MaybeUninit<T>` storage. The
        // chunk is deliberately not committed, so the zeroed bytes are never
        // observed as a `T` and the ring remains empty.
        unsafe {
            std::ptr::write_volatile(slot, std::mem::MaybeUninit::zeroed());
        }
    }
}

#[cfg(all(test, feature = "rtrb"))]
mod tests {
    use super::*;

    #[test]
    fn prefault_ring_leaves_the_ring_empty() {
        let (mut producer, consumer) = rtrb::RingBuffer::<u64>::new(1024);

        prefault_ring(&mut producer);

        assert_eq!(producer.slots(), producer.buffer().capacity());
        assert!(consumer.is_empty());
    }
}
