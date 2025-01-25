use crate::config::MetricsConfig;
use crate::Error;
use core_affinity::CoreId;
use log::{info, warn};

pub enum Affinity {
    NoOp,
    CpuId(CoreId),
    CpuIndex(usize),
}

impl TryFrom<MetricsConfig> for Affinity {
    type Error = Error;

    fn try_from(config: MetricsConfig) -> Result<Self, Self::Error> {
        match (config.aggregator_affinity_cpu_id, config.aggregator_affinity_cpu_index) {
            (Some(_), Some(_)) => Err(Error::other("cannot specify both cpu and cpu_index")),
            (Some(cpu_id), None) => Ok(Affinity::CpuId(CoreId { id: cpu_id })),
            (None, Some(cpu_index)) => Ok(Affinity::CpuIndex(cpu_index)),
            (None, None) => Ok(Affinity::NoOp),
        }
    }
}

impl Affinity {
    pub fn pin_current_thread_to_core(&self) {
        if let Some(core_ids) = core_affinity::get_core_ids() {
            match self {
                Affinity::CpuId(core_id) => {
                    if core_ids.contains(core_id) {
                        core_affinity::set_for_current(*core_id);
                        info!("successfully pinned current thread to core {}", core_id.id);
                    } else {
                        warn!("core id {} is not present in the available cpu set", core_id.id)
                    }
                }
                Affinity::CpuIndex(cpu_index) => {
                    if let Some(core_id) = core_ids.get(*cpu_index) {
                        core_affinity::set_for_current(*core_id);
                        info!("successfully pinned current thread to core {}", core_id.id);
                    } else {
                        warn!("core index {cpu_index} is not present in the available cpu set")
                    }
                }
                Affinity::NoOp => {}
            }
        } else {
            warn!("unable to find any cores on which the current thread is allowed to run")
        }
    }
}
