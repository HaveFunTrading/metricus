#![doc = include_str!("../README.md")]

mod aggregator;
pub mod config;
mod error;
mod exporter;

use crate::aggregator::{Counter, MetricsAggregator};
use metricus::{set_backend, Id, MetricsBackend, Tag, Tags};
use rtrb::Producer;
use std::collections::HashMap;
use std::io::Write;
use crate::config::MetricsConfig;
use crate::exporter::Exporter;

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
    tx: Producer<Event>,
    next_id: Id,
    metric_key_to_id: HashMap<MetricKey, Id>,
}

impl MetricsAgent {

    pub fn init() {
        Self::init_with_config(MetricsConfig::default());
    }

    pub fn init_with_config(config: MetricsConfig) {
        let (tx, rx) = rtrb::RingBuffer::new(config.event_channel_size);
        let exporter = config.exporter.try_into().unwrap();
        let _ = MetricsAggregator::start_on_thread(rx, exporter, config.flush_interval);
        set_backend(MetricsAgent::new(tx));
    }

    fn new(tx: Producer<Event>) -> Self {
        Self {
            tx,
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
    fn send_event(&mut self, event: Event) {
        let _ = self.tx.push(event);
    }
}

impl MetricsBackend for MetricsAgent {
    fn name(&self) -> &'static str {
        "metrics-agent"
    }

    fn new_counter(&mut self, name: &str, tags: Tags) -> Id {
        let mut tags = tags.to_owned_tags();
        tags.push(("type", "counter").to_owned_tag());
        tags.sort_unstable();
        tags.dedup();
        let id = self.assign_next_id(name, tags.clone());
        self.send_event(Event::CounterCreate(id, name.to_owned(), tags));
        id
    }

    fn delete_counter(&mut self, id: Id) {
        self.send_event(Event::CounterDelete(id));
    }

    #[inline]
    fn increment_counter_by(&mut self, id: Id, delta: usize) {
        self.send_event(Event::CounterIncrement(id, delta));
    }

    fn new_histogram(&mut self, name: &str, tags: Tags) -> Id {
        let mut tags = tags.to_owned_tags();
        tags.push(("type", "histogram").to_owned_tag());
        tags.sort_unstable();
        tags.dedup();
        let id = self.assign_next_id(name, tags.clone());
        self.send_event(Event::HistogramCreate(id, name.to_owned(), tags));
        id
    }

    fn delete_histogram(&mut self, id: Id) {
        self.send_event(Event::HistogramDelete(id));
    }

    #[inline]
    fn record(&mut self, id: Id, value: u64) {
        self.send_event(Event::HistogramRecord(id, value));
    }
}

#[derive(Debug)]
enum Event {
    CounterCreate(Id, String, OwnedTags),
    CounterIncrement(Id, usize),
    CounterDelete(Id),
    HistogramCreate(Id, String, OwnedTags),
    HistogramDelete(Id),
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


