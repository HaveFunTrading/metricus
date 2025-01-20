use crate::exporter::Exporter;
use crate::{Event, OwnedTags};
use metricus::Id;
use rtrb::Consumer;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::thread::JoinHandle;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub struct MetricsAggregator {
    rx: Consumer<Event>,
    exporter: Exporter,
    counters: HashMap<Id, Counter>,
    histograms: HashMap<Id, Histogram>,
    next_flush_time_ns: u64,
    flush_interval_ns: u64,
}

impl MetricsAggregator {
    pub fn new(rx: Consumer<Event>, exporter: Exporter, flush_interval: Duration) -> Self {
        Self {
            rx,
            exporter,
            counters: Default::default(),
            histograms: Default::default(),
            flush_interval_ns: flush_interval.as_nanos() as u64,
            next_flush_time_ns: current_time_ns() + flush_interval.as_nanos() as u64,
        }
    }

    pub fn start_on_thread(
        rx: Consumer<Event>,
        exporter: Exporter,
        flush_interval: Duration,
    ) -> JoinHandle<Result<(), &'static str>> {
        std::thread::spawn(move || {
            let mut aggregator = MetricsAggregator::new(rx, exporter, flush_interval);
            loop {
                aggregator.poll();
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
        })
    }

    fn poll(&mut self) {
        self.process_events();
        let now = current_time_ns();
        if now > self.next_flush_time_ns {
            self.flush_counters(now).unwrap(); // TODO error handling
            self.next_flush_time_ns = now + self.flush_interval_ns;
        }
    }

    fn process_events(&mut self) {
        let available = self.rx.slots();
        if let Ok(chunk) = self.rx.read_chunk(available) {
            for event in chunk {
                match event {
                    Event::CounterCreate(id, name, tags) => {
                        self.counters.entry(id).or_insert_with(|| Counter::new(name, tags));
                    }
                    Event::CounterIncrement(id, delta) => {
                        if let Some(counter) = self.counters.get_mut(&id) {
                            counter.increment(delta);
                        }
                    }
                    Event::CounterDelete(id) => {
                        self.counters.remove(&id);
                    }
                    Event::HistogramCreate(_, _, _) => {}
                    Event::HistogramDelete(_) => {}
                    Event::HistogramRecord(_, _) => {}
                }
            }
        }
    }

    fn flush_counters(&mut self, timestamp: u64) -> std::io::Result<()> {
        self.exporter.publish_counters(&self.counters, timestamp)
    }
}

#[derive(Serialize)]
pub struct Counter {
    value: usize,
    #[serde(flatten)]
    meta_data: MetaData,
}

impl Counter {
    fn new(name: String, tags: OwnedTags) -> Self {
        Self {
            value: 0,
            meta_data: MetaData::new(name, tags),
        }
    }

    fn increment(&mut self, delta: usize) {
        self.value += delta;
    }
}

struct Histogram {
    inner: hdrhistogram::Histogram<u64>,
    meta_data: MetaData,
}

#[derive(Serialize)]
struct MetaData {
    name: String,
    tags: OwnedTags,
}

impl MetaData {
    fn new(name: String, tags: OwnedTags) -> Self {
        Self { name, tags }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum Encoder {
    LineProtocol,
    Json,
}

impl Encoder {
    pub fn encode_counter(&self, counter: &Counter, timestamp: u64, dst: &mut impl Write) -> std::io::Result<()> {
        match self {
            Encoder::LineProtocol => LineProtocol::encode_counter(counter, dst),
            Encoder::Json => Json::encode_counter(counter, timestamp, dst),
        }
    }
}

struct LineProtocol;

impl LineProtocol {
    fn encode_counter(_counter: &Counter, _dst: &mut impl Write) -> std::io::Result<()> {
        todo!()
    }
}

struct Json;

impl Json {
    fn encode_counter(counter: &Counter, timestamp: u64, dst: &mut impl Write) -> std::io::Result<()> {
        serde_json::to_writer(&mut *dst, &CounterWithTimestamp::new(counter, timestamp))
            .map_err(std::io::Error::other)
            .and_then(|_| dst.write_all(b"\n"))
    }
}

#[derive(Serialize)]
struct CounterWithTimestamp<'a> {
    timestamp: u64,
    #[serde(flatten)]
    counter: &'a Counter,
}

impl<'a> CounterWithTimestamp<'a> {
    fn new(counter: &'a Counter, timestamp: u64) -> Self {
        Self { timestamp, counter }
    }
}

fn current_time_ns() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64
}
