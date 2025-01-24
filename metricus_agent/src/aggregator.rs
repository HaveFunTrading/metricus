use crate::config::MetricsConfig;
use crate::exporter::Exporter;
use crate::{error, ControlEvent, Error, OwnedTags, UpdateEvent};
use metricus::Id;
#[cfg(feature = "rtrb")]
use rtrb::Consumer;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
#[cfg(not(feature = "rtrb"))]
use std::sync::mpsc::Receiver;
use std::thread::JoinHandle;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

type Counters = HashMap<Id, Counter>;
type Histograms = HashMap<Id, Histogram>;

pub struct MetricsAggregator {
    #[cfg(feature = "rtrb")]
    rx_upd: Consumer<UpdateEvent>,
    #[cfg(feature = "rtrb")]
    rx_cnc: Consumer<ControlEvent>,
    #[cfg(not(feature = "rtrb"))]
    rx_upd: Receiver<UpdateEvent>,
    #[cfg(not(feature = "rtrb"))]
    rx_cnc: Receiver<ControlEvent>,
    exporter: Exporter,
    counters: Counters,
    histograms: Histograms,
    next_flush_time_ns: u64,
    flush_interval_ns: u64,
}

impl MetricsAggregator {
    pub fn new(
        #[cfg(feature = "rtrb")] rx_upd: Consumer<UpdateEvent>,
        #[cfg(feature = "rtrb")] rx_cnc: Consumer<ControlEvent>,
        #[cfg(not(feature = "rtrb"))] rx_upd: Receiver<UpdateEvent>,
        #[cfg(not(feature = "rtrb"))] rx_cnc: Receiver<ControlEvent>,
        exporter: Exporter,
        flush_interval: Duration,
    ) -> Self {
        Self {
            rx_upd,
            rx_cnc,
            exporter,
            counters: Default::default(),
            histograms: Default::default(),
            flush_interval_ns: flush_interval.as_nanos() as u64,
            next_flush_time_ns: current_time_ns() + flush_interval.as_nanos() as u64,
        }
    }

    pub fn start_on_thread(
        #[cfg(feature = "rtrb")] rx_upd: Consumer<UpdateEvent>,
        #[cfg(feature = "rtrb")] rx_cnc: Consumer<ControlEvent>,
        #[cfg(not(feature = "rtrb"))] rx_upd: Receiver<UpdateEvent>,
        #[cfg(not(feature = "rtrb"))] rx_cnc: Receiver<ControlEvent>,
        config: MetricsConfig,
    ) -> JoinHandle<error::Result<()>> {
        std::thread::spawn(move || {
            let exporter = config.exporter.try_into()?;
            let mut aggregator = MetricsAggregator::new(rx_upd, rx_cnc, exporter, config.flush_interval);
            loop {
                aggregator.poll()?;
                std::thread::sleep(Duration::from_millis(1));
            }
        })
    }

    #[inline]
    fn poll(&mut self) -> crate::Result<()> {
        self.process_events()?;
        let now = current_time_ns();
        if now > self.next_flush_time_ns {
            self.flush_metrics(now)?;
            self.next_flush_time_ns = now + self.flush_interval_ns;
        }
        Ok(())
    }

    #[cfg(feature = "rtrb")]
    #[inline]
    fn process_events(&mut self) -> crate::Result<()> {
        if let Ok(chunk) = self.rx_cnc.read_chunk(self.rx_cnc.slots()) {
            for event in chunk {
                Self::handle_control_event(&mut self.counters, &mut self.histograms, event)?;
            }
        }
        if let Ok(chunk) = self.rx_upd.read_chunk(self.rx_upd.slots()) {
            for event in chunk {
                Self::handle_update_event(&mut self.counters, &mut self.histograms, event)?;
            }
        }
        Ok(())
    }

    #[cfg(not(feature = "rtrb"))]
    #[inline]
    fn process_events(&mut self) -> crate::Result<()> {
        for event in self.rx_cnc.try_iter() {
            Self::handle_control_event(&mut self.counters, &mut self.histograms, event)?;
        }
        for event in self.rx_upd.try_iter() {
            Self::handle_update_event(&mut self.counters, &mut self.histograms, event)?;
        }
        Ok(())
    }

    #[inline]
    fn handle_control_event(
        counters: &mut Counters,
        histograms: &mut Histograms,
        event: ControlEvent,
    ) -> crate::Result<()> {
        match event {
            ControlEvent::CounterCreate(id, name, tags) => {
                counters.entry(id).or_insert_with(|| Counter::new(name, tags));
            }
            ControlEvent::CounterDelete(id) => {
                counters.remove(&id);
            }
            ControlEvent::HistogramCreate(id, name, tags) => {
                histograms.entry(id).or_insert_with(|| Histogram::new(name, tags));
            }
            ControlEvent::HistogramDelete(id) => {
                histograms.remove(&id);
            }
        }
        Ok(())
    }

    #[inline]
    fn handle_update_event(
        counters: &mut Counters,
        histograms: &mut Histograms,
        event: UpdateEvent,
    ) -> crate::Result<()> {
        match event {
            UpdateEvent::CounterIncrement(id, delta) => {
                if let Some(counter) = counters.get_mut(&id) {
                    counter.increment(delta);
                }
            }
            UpdateEvent::HistogramRecord(id, value) => {
                if let Some(histogram) = histograms.get_mut(&id) {
                    histogram.inner.record(value).map_err(Error::other)?;
                }
            }
        }
        Ok(())
    }

    #[inline]
    fn flush_metrics(&mut self, timestamp: u64) -> crate::Result<()> {
        self.exporter.publish_counters(&self.counters, timestamp)?;
        self.exporter.publish_histograms(&self.histograms, timestamp)?;
        // clear histograms
        self.histograms
            .iter_mut()
            .for_each(|(_, histogram)| histogram.inner.clear());
        Ok(())
    }
}

#[derive(Serialize)]
pub struct Counter {
    value: u64,
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

    fn increment(&mut self, delta: u64) {
        self.value += delta;
    }
}

pub struct Histogram {
    inner: hdrhistogram::Histogram<u64>,
    meta_data: MetaData,
}

impl Histogram {
    fn new(name: String, tags: OwnedTags) -> Self {
        Self {
            inner: hdrhistogram::Histogram::<u64>::new(3).unwrap(), // will never fail
            meta_data: MetaData::new(name, tags),
        }
    }
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
#[serde(rename_all = "snake_case")]
pub enum Encoder {
    LineProtocol,
    Json,
}

impl Encoder {
    pub fn encode_counter(&self, counter: &Counter, timestamp: u64, dst: &mut impl Write) -> std::io::Result<()> {
        match self {
            Encoder::LineProtocol => LineProtocol::encode_counter(counter, timestamp, dst),
            Encoder::Json => Json::encode_counter(counter, timestamp, dst),
        }
    }

    pub fn encode_histogram(&self, histogram: &Histogram, timestamp: u64, dst: &mut impl Write) -> std::io::Result<()> {
        match self {
            Encoder::LineProtocol => LineProtocol::encode_histogram(histogram, timestamp, dst),
            Encoder::Json => Ok(()),
        }
    }
}

struct LineProtocol;

impl LineProtocol {
    fn encode_counter(counter: &Counter, timestamp: u64, dst: &mut impl Write) -> std::io::Result<()> {
        // measurement
        dst.write_all(counter.meta_data.name.as_bytes())?;
        // tags
        for tag in counter.meta_data.tags.iter() {
            dst.write_all(b",")?;
            dst.write_all(tag.0.as_bytes())?;
            dst.write_all(b"=")?;
            dst.write_all(tag.1.as_bytes())?;
        }
        // field
        dst.write_all(b" value=")?;
        dst.write_all(itoa::Buffer::new().format(counter.value).as_bytes())?;
        dst.write_all(b"u ")?;
        // timestamp
        dst.write_all(itoa::Buffer::new().format(timestamp).as_bytes())?;
        // new line
        dst.write_all(b"\n")?;
        Ok(())
    }

    fn encode_histogram(histogram: &Histogram, timestamp: u64, dst: &mut impl Write) -> std::io::Result<()> {
        // measurement
        dst.write_all(histogram.meta_data.name.as_bytes())?;
        // tags
        for tag in histogram.meta_data.tags.iter() {
            dst.write_all(b",")?;
            dst.write_all(tag.0.as_bytes())?;
            dst.write_all(b"=")?;
            dst.write_all(tag.1.as_bytes())?;
        }
        // fields
        dst.write_all(b" count=")?;
        dst.write_all(itoa::Buffer::new().format(histogram.inner.len()).as_bytes())?;
        dst.write_all(b"u,min=")?;
        dst.write_all(itoa::Buffer::new().format(histogram.inner.min()).as_bytes())?;
        dst.write_all(b"u,max=")?;
        dst.write_all(itoa::Buffer::new().format(histogram.inner.max()).as_bytes())?;
        dst.write_all(b"u,mean=")?;
        dst.write_all(dtoa::Buffer::new().format(histogram.inner.mean()).as_bytes())?;
        dst.write_all(b"f,p50=")?;
        dst.write_all(
            itoa::Buffer::new()
                .format(histogram.inner.value_at_quantile(0.50))
                .as_bytes(),
        )?;
        dst.write_all(b"u,p75=")?;
        dst.write_all(
            itoa::Buffer::new()
                .format(histogram.inner.value_at_quantile(0.75))
                .as_bytes(),
        )?;
        dst.write_all(b"u,p90=")?;
        dst.write_all(
            itoa::Buffer::new()
                .format(histogram.inner.value_at_quantile(0.90))
                .as_bytes(),
        )?;
        dst.write_all(b"u,p95=")?;
        dst.write_all(
            itoa::Buffer::new()
                .format(histogram.inner.value_at_quantile(0.95))
                .as_bytes(),
        )?;
        dst.write_all(b"u,p99=")?;
        dst.write_all(
            itoa::Buffer::new()
                .format(histogram.inner.value_at_quantile(0.99))
                .as_bytes(),
        )?;
        dst.write_all(b"u,p999=")?;
        dst.write_all(
            itoa::Buffer::new()
                .format(histogram.inner.value_at_quantile(0.999))
                .as_bytes(),
        )?;
        dst.write_all(b"u,p9999=")?;
        dst.write_all(
            itoa::Buffer::new()
                .format(histogram.inner.value_at_quantile(0.9999))
                .as_bytes(),
        )?;
        dst.write_all(b"u ")?;
        // timestamp
        dst.write_all(itoa::Buffer::new().format(timestamp).as_bytes())?;
        // new line
        dst.write_all(b"\n")?;
        Ok(())
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
