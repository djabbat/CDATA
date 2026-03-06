use crate::IoResult;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationConfig {
    pub max_steps: u64,
    pub dt: f64,
    pub num_threads: Option<usize>,
    pub seed: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleConfigs {
    pub centriole: Option<serde_json::Value>,
    pub cell_cycle: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationConfigFull {
    pub simulation: SimulationConfig,
    pub modules: ModuleConfigs,
}

pub fn load_json_config(path: impl AsRef<Path>) -> IoResult<SimulationConfigFull> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let config: SimulationConfigFull = serde_json::from_reader(reader)?;
    Ok(config)
}

pub fn save_json_config(path: impl AsRef<Path>, config: &SimulationConfigFull) -> IoResult<()> {
    let file = File::create(path)?;
    serde_json::to_writer_pretty(file, config)?;
    Ok(())
}
