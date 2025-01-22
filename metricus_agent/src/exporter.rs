use crate::aggregator::{Counter, Encoder, Histogram};
use crate::config::{ExporterSource, FileConfig, UdpConfig, UnixSocketConfig};
use log::warn;
use metricus::Id;
use std::collections::HashMap;
use std::fs::{create_dir_all, File};
use std::io::{BufWriter, ErrorKind, Write};
use std::net::UdpSocket;
use std::os::unix::net::UnixStream;
use std::path::Path;

type FileExporter = StreamExporter<File>;
type UnixSocketExporter = StreamExporter<UnixStream>;

pub enum Exporter {
    NoOp,
    Udp(UdpExporter),
    File(FileExporter),
    UnixSocket(UnixSocketExporter),
}

impl TryFrom<ExporterSource> for Exporter {
    type Error = std::io::Error;

    fn try_from(source: ExporterSource) -> Result<Self, Self::Error> {
        match source {
            ExporterSource::NoOp => Ok(Exporter::NoOp),
            ExporterSource::Udp(config) => Ok(Exporter::Udp(UdpExporter::try_from(config)?)),
            ExporterSource::File(config) => Ok(Exporter::File(FileExporter::try_from(config)?)),
            ExporterSource::UnixSocket(config) => Ok(Exporter::UnixSocket(UnixSocketExporter::try_from(config)?)),
        }
    }
}

impl Exporter {
    pub fn publish_counters(&mut self, counters: &HashMap<Id, Counter>, timestamp: u64) -> std::io::Result<()> {
        match self {
            Exporter::NoOp => Ok(()),
            Exporter::Udp(exporter) => exporter.publish_counters(counters, timestamp),
            Exporter::File(exporter) => exporter.publish_counters(counters, timestamp),
            Exporter::UnixSocket(exporter) => exporter.publish_counters(counters, timestamp),
        }
    }

    pub fn publish_histograms(&mut self, histograms: &HashMap<Id, Histogram>, timestamp: u64) -> std::io::Result<()> {
        match self {
            Exporter::NoOp => Ok(()),
            Exporter::Udp(exporter) => exporter.publish_histograms(histograms, timestamp),
            Exporter::File(exporter) => exporter.publish_histograms(histograms, timestamp),
            Exporter::UnixSocket(exporter) => exporter.publish_histograms(histograms, timestamp),
        }
    }
}

pub struct UdpExporter {
    socket: UdpSocket,
    buffer: Vec<u8>,
    encoder: Encoder,
}

impl TryFrom<UdpConfig> for UdpExporter {
    type Error = std::io::Error;

    fn try_from(config: UdpConfig) -> Result<Self, Self::Error> {
        let socket = UdpSocket::bind("127.0.0.1:0")?;
        socket.connect(&config)?;
        Ok(Self {
            socket,
            buffer: Vec::with_capacity(1024),
            encoder: config.encoder,
        })
    }
}

impl UdpExporter {
    fn publish_counters(&mut self, counters: &HashMap<Id, Counter>, timestamp: u64) -> std::io::Result<()> {
        if counters.is_empty() {
            return Ok(());
        }

        for counter in counters.values() {
            self.encoder.encode_counter(counter, timestamp, &mut self.buffer)?;
        }

        // we can ignore connection refused in case the udp listener is temporarily unavailable
        if let Err(err) = self.socket.send(&self.buffer) {
            match err.kind() {
                ErrorKind::ConnectionRefused => warn!("Failed to send metrics via udp: [{}]", err),
                _ => Err(err)?,
            }
        }

        self.buffer.clear();
        Ok(())
    }

    fn publish_histograms(&mut self, histograms: &HashMap<Id, Histogram>, timestamp: u64) -> std::io::Result<()> {
        if histograms.is_empty() {
            return Ok(());
        }

        for histogram in histograms.values() {
            self.encoder.encode_histogram(histogram, timestamp, &mut self.buffer)?
        }

        // we can ignore connection refused in case the udp listener is temporarily unavailable
        if let Err(err) = self.socket.send(&self.buffer) {
            match err.kind() {
                ErrorKind::ConnectionRefused => warn!("Failed to send metrics via udp: [{}]", err),
                _ => Err(err)?,
            }
        }

        self.buffer.clear();
        Ok(())
    }
}

pub struct StreamExporter<S: Write> {
    writer: BufWriter<S>,
    encoder: Encoder,
}

impl TryFrom<FileConfig> for StreamExporter<File> {
    type Error = std::io::Error;

    fn try_from(config: FileConfig) -> Result<Self, Self::Error> {
        let path = Path::new(&config.path);
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                create_dir_all(parent)?;
            }
        }
        let file = File::create(path)?;
        Ok(Self {
            writer: BufWriter::new(file),
            encoder: config.encoder,
        })
    }
}

impl TryFrom<UnixSocketConfig> for StreamExporter<UnixStream> {
    type Error = std::io::Error;

    fn try_from(config: UnixSocketConfig) -> Result<Self, Self::Error> {
        let stream = UnixStream::connect(config.path)?;
        Ok(Self {
            writer: BufWriter::new(stream),
            encoder: config.encoder,
        })
    }
}

impl<S: Write> StreamExporter<S> {
    fn publish_counters(&mut self, counters: &HashMap<Id, Counter>, timestamp: u64) -> std::io::Result<()> {
        for counter in counters.values() {
            self.encoder.encode_counter(counter, timestamp, &mut self.writer)?;
        }
        self.writer.flush()?;
        Ok(())
    }

    fn publish_histograms(&mut self, histograms: &HashMap<Id, Histogram>, timestamp: u64) -> std::io::Result<()> {
        for histogram in histograms.values() {
            self.encoder.encode_histogram(histogram, timestamp, &mut self.writer)?;
        }
        self.writer.flush()?;
        Ok(())
    }
}
