use crate::aggregator::{Counter, Encoder};
use crate::config::{ExporterSource, FileConfig, UdpConfig};
use metricus::Id;
use std::collections::HashMap;
use std::fs::{create_dir_all, File};
use std::io::{BufWriter, Write};
use std::net::UdpSocket;
use std::path::Path;

pub enum Exporter {
    NoOp,
    Udp(UdpExporter),
    File(FileExporter),
}

impl Exporter {
    pub fn publish_counters(&mut self, counters: &HashMap<Id, Counter>, timestamp: u64) -> std::io::Result<()> {
        match self {
            Exporter::NoOp => Ok(()),
            Exporter::Udp(exporter) => exporter.publish_counters(counters),
            Exporter::File(exporter) => exporter.publish_counters(counters, timestamp),
        }
    }
}

struct UdpExporter {
    socket: UdpSocket,
    buffer: Vec<u8>,
    encoder: Encoder,
}

impl TryFrom<UdpConfig> for UdpExporter {
    type Error = std::io::Error;

    fn try_from(config: UdpConfig) -> Result<Self, Self::Error> {
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        socket.connect(&config)?;
        Ok(Self {
            socket,
            buffer: Vec::with_capacity(1024),
            encoder: config.encoder,
        })
    }
}

impl UdpExporter {
    fn publish_counters(&mut self, counters: &HashMap<Id, Counter>) -> std::io::Result<()> {
        todo!()
        // FIXME this should take a batch of counters
        // self.encoder.encode_counter(counter, &mut self.buffer)?;
        // self.socket.send(&self.buffer)?;
        // self.buffer.clear();
        // Ok(())
    }
}

struct FileExporter {
    writer: BufWriter<File>,
    encoder: Encoder,
}

impl TryFrom<FileConfig> for FileExporter {
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

impl FileExporter {
    fn publish_counters(&mut self, counters: &HashMap<Id, Counter>, timestamp: u64) -> std::io::Result<()> {
        for counter in counters.values() {
            self.encoder.encode_counter(counter, timestamp, &mut self.writer)?;
        }
        self.writer.flush()?;
        Ok(())
    }
}

impl TryFrom<ExporterSource> for Exporter {
    type Error = std::io::Error;

    fn try_from(source: ExporterSource) -> Result<Self, Self::Error> {
        match source {
            ExporterSource::NoOp => Ok(Exporter::NoOp),
            ExporterSource::Udp(config) => Ok(Exporter::Udp(UdpExporter::try_from(config)?)),
            ExporterSource::File(config) => Ok(Exporter::File(FileExporter::try_from(config)?)),
        }
    }
}
