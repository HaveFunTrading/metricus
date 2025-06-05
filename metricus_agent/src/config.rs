use crate::OwnedTags;
use crate::aggregator::Encoder;
use duration_str::deserialize_duration;
use metricus::PreAllocatedMetric;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use std::collections::HashMap;
use std::net::{SocketAddr, ToSocketAddrs};
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;
use std::vec;

/// Metrics config to be passed to MetricsAgent during initialisation.
#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct MetricsConfig {
    /// Interval at which metrics are written to the targets. This defaults to 10 seconds.
    #[serde(deserialize_with = "deserialize_duration")]
    #[serde(default = "get_default_flush_interval")]
    pub flush_interval: Duration,
    /// Default tags that will be added to all metrics.
    #[serde_as(as = "HashMap<_, _>")]
    #[serde(default)]
    pub default_tags: OwnedTags,
    /// Event channel size between the metrics agent and aggregator. This defaults to 1 million.
    #[serde(default = "get_default_event_channel_size")]
    pub event_channel_size: usize,
    /// Metrics exporter type.
    #[serde(default)]
    pub exporter: ExporterSource,
    #[serde(default)]
    pub pre_allocated_metrics: Vec<PreAllocatedMetric>,
    /// CPU id for the metrics aggregator thread. Cannot be used with [MetricsConfig:aggregator_affinity_cpu_index] `aggregator_affinity_cpu_index`.
    #[serde(default)]
    pub aggregator_affinity_cpu_id: Option<usize>,
    /// CPU index of the available cpu set for the metrics aggregator thread. Cannot be used with `aggregator_affinity_cpu_id`.
    #[serde(default)]
    pub aggregator_affinity_cpu_index: Option<usize>,
}

impl MetricsConfig {
    pub fn from_file(path: impl AsRef<Path>) -> std::io::Result<MetricsConfig> {
        serde_yaml::from_reader(std::fs::File::open(path)?).map_err(std::io::Error::other)
    }

    pub fn with_default_tags(self, default_tags: OwnedTags) -> MetricsConfig {
        MetricsConfig {
            default_tags: [self.default_tags, default_tags].concat(),
            ..self
        }
    }

    pub fn with_pre_allocated_metrics<F>(self, pre_allocated_metrics: F) -> MetricsConfig
    where
        F: FnOnce() -> Vec<PreAllocatedMetric>,
    {
        MetricsConfig {
            pre_allocated_metrics: [self.pre_allocated_metrics, pre_allocated_metrics()].concat(),
            ..self
        }
    }
}

impl FromStr for MetricsConfig {
    type Err = std::io::Error;

    fn from_str(config: &str) -> Result<Self, Self::Err> {
        serde_yaml::from_str(config).map_err(std::io::Error::other)
    }
}

const fn get_default_event_channel_size() -> usize {
    1024 * 1024
}

const fn get_default_flush_interval() -> Duration {
    Duration::from_secs(10)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum Format {
    LineProtocol,
    Json,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type", content = "config")]
pub enum ExporterSource {
    #[default]
    NoOp,
    Udp(UdpConfig),
    File(FileConfig),
    UnixStream(UnixSocketConfig),
    UnixDatagram(UnixSocketConfig),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UdpConfig {
    pub host: String,
    pub port: u16,
    pub encoder: Encoder,
}

impl ToSocketAddrs for UdpConfig {
    type Iter = vec::IntoIter<SocketAddr>;

    fn to_socket_addrs(&self) -> std::io::Result<Self::Iter> {
        format!("{}:{}", self.host, self.port).to_socket_addrs()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileConfig {
    pub path: String,
    pub encoder: Encoder,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UnixSocketConfig {
    pub path: String,
    pub encoder: Encoder,
}
