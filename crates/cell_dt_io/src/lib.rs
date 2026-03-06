//! Модуль ввода/вывода данных для Cell DT платформы

mod csv_exporter;
mod config;
mod cdata_exporter;

pub use csv_exporter::*;
pub use config::*;
pub use cdata_exporter::{CdataRecord, CdataExporter, write_cdata_csv};

use cell_dt_core::{
    components::*,
    hecs::World,
};
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Ошибки ввода/вывода
#[derive(Error, Debug)]
pub enum IoError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("CSV error: {0}")]
    Csv(#[from] csv::Error),
    
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Результат операций ввода/вывода
pub type IoResult<T> = Result<T, IoError>;

/// Данные одной клетки для экспорта
#[derive(Debug, Clone)]
pub struct CellData {
    pub cell_id: u64,
    pub step: u64,
    pub time: f64,
    pub mother_maturity: f32,
    pub daughter_maturity: f32,
    pub mtoc_activity: f32,
    pub cilium_present: bool,
    pub phase: String,
    pub cycle_progress: f32,
    pub cycle_count: u32,
    pub growth_signal: f32,
    pub stress_level: f32,
}

impl CellData {
    pub fn from_components(
        cell_id: u64,
        step: u64,
        time: f64,
        centriole: &CentriolePair,
        cell_cycle: &CellCycleStateExtended,
    ) -> Self {
        Self {
            cell_id,
            step,
            time,
            mother_maturity: centriole.mother.maturity,
            daughter_maturity: centriole.daughter.maturity,
            mtoc_activity: centriole.mtoc_activity,
            cilium_present: centriole.cilium_present,
            phase: format!("{:?}", cell_cycle.phase),
            cycle_progress: cell_cycle.progress,
            cycle_count: cell_cycle.cycle_count,
            growth_signal: cell_cycle.growth_factors.growth_signal,
            stress_level: cell_cycle.growth_factors.stress_level,
        }
    }
    
    pub fn csv_headers() -> Vec<String> {
        vec![
            "cell_id".to_string(),
            "step".to_string(),
            "time".to_string(),
            "mother_maturity".to_string(),
            "daughter_maturity".to_string(),
            "mtoc_activity".to_string(),
            "cilium_present".to_string(),
            "phase".to_string(),
            "cycle_progress".to_string(),
            "cycle_count".to_string(),
            "growth_signal".to_string(),
            "stress_level".to_string(),
        ]
    }
    
    pub fn to_csv_record(&self) -> Vec<String> {
        vec![
            self.cell_id.to_string(),
            self.step.to_string(),
            format!("{:.6}", self.time),
            format!("{:.6}", self.mother_maturity),
            format!("{:.6}", self.daughter_maturity),
            format!("{:.6}", self.mtoc_activity),
            (self.cilium_present as u8).to_string(),
            self.phase.clone(),
            format!("{:.6}", self.cycle_progress),
            self.cycle_count.to_string(),
            format!("{:.6}", self.growth_signal),
            format!("{:.6}", self.stress_level),
        ]
    }
}

/// Менеджер экспорта данных
pub struct DataExporter {
    output_dir: PathBuf,
    prefix: String,
    buffer: Vec<CellData>,
}

impl DataExporter {
    pub fn new(output_dir: impl AsRef<Path>, prefix: &str) -> Self {
        let output_dir = output_dir.as_ref().to_path_buf();
        let _ = std::fs::create_dir_all(&output_dir);
        
        Self {
            output_dir,
            prefix: prefix.to_string(),
            buffer: Vec::new(),
        }
    }
    
    pub fn collect_data(&mut self, world: &World, step: u64, time: f64) -> IoResult<()> {
        let mut query = world.query::<(&CentriolePair, &CellCycleStateExtended)>();
        
        for (entity, (centriole, cell_cycle)) in query.iter() {
            let cell_id = entity.to_bits().get();
            
            let cell_data = CellData::from_components(
                cell_id,
                step,
                time,
                centriole,
                cell_cycle,
            );
            
            self.buffer.push(cell_data);
        }
        
        Ok(())
    }
    
    pub fn save_snapshot(&mut self, step: u64) -> IoResult<PathBuf> {
        if self.buffer.is_empty() {
            return Err(IoError::Io(std::io::Error::other("No data to save")));
        }
        
        let csv_path = self.output_dir.join(format!(
            "{}_step_{:06}.csv",
            self.prefix, step
        ));
        
        csv_exporter::write_csv(&csv_path, &self.buffer)?;
        self.buffer.clear();
        
        Ok(csv_path)
    }
    
    pub fn clear(&mut self) {
        self.buffer.clear();
    }

    /// Число записей в буфере (до сохранения)
    pub fn buffered(&self) -> usize {
        self.buffer.len()
    }
}
