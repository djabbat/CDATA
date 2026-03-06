//! Модуль визуализации для Cell DT платформы
//! Поддерживает 2D графики через plotters и 3D визуализацию через kiss3d

mod plot2d;
mod plot3d;
mod heatmap;
mod timeseries;
mod cdata_timeseries;

pub use plot2d::*;
pub use plot3d::*;
pub use heatmap::*;
pub use timeseries::*;
pub use cdata_timeseries::{CdataSnapshot, CdataTimeSeriesVisualizer};

use cell_dt_core::{
    components::{CentriolePair, CellCycleState, Phase},
    hecs::World,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

/// Типы визуализации
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisualizationType {
    Heatmap,
    ScatterPlot,
    TimeSeries,
    Histogram,
    PieChart,
    BarChart,
    Network,
    Dendrogram,
}

/// Данные для визуализации
#[derive(Debug, Clone)]
pub struct VisualizationData {
    pub step: u64,
    pub time: f64,
    pub cell_count: usize,
    pub phase_distribution: HashMap<Phase, usize>,
    pub centriole_maturity: Vec<f32>,
    pub mtoc_activity: Vec<f32>,
    pub cafd_counts: Vec<usize>,
    pub cilia_count: usize,
}

impl VisualizationData {
    pub fn from_world(world: &World) -> Self {
        let mut query = world.query::<(&CentriolePair, &CellCycleState)>();
        
        let mut phase_distribution = HashMap::new();
        let mut centriole_maturity = Vec::new();
        let mut mtoc_activity = Vec::new();
        let mut cafd_counts = Vec::new();
        let mut cilia_count = 0;
        let mut cell_count = 0;
        
        for (_, (pair, cycle)) in query.iter() {
            cell_count += 1;
            
            *phase_distribution.entry(cycle.phase).or_insert(0) += 1;
            
            centriole_maturity.push(pair.mother.maturity);
            centriole_maturity.push(pair.daughter.maturity);
            
            mtoc_activity.push(pair.mtoc_activity);
            
            cafd_counts.push(pair.mother.associated_cafds.len());
            
            if pair.cilium_present {
                cilia_count += 1;
            }
        }
        
        VisualizationData {
            step: 0,
            time: 0.0,
            cell_count,
            phase_distribution,
            centriole_maturity,
            mtoc_activity,
            cafd_counts,
            cilia_count,
        }
    }
}

/// Менеджер визуализации
pub struct VisualizationManager {
    pub data_history: Arc<Mutex<Vec<VisualizationData>>>,
    active_viz: Vec<Box<dyn Visualizer + Send>>,
    update_interval: u64,
    last_update: u64,
}

impl VisualizationManager {
    pub fn new(update_interval: u64) -> Self {
        Self {
            data_history: Arc::new(Mutex::new(Vec::new())),
            active_viz: Vec::new(),
            update_interval,
            last_update: 0,
        }
    }
    
    pub fn add_visualizer(&mut self, visualizer: Box<dyn Visualizer + Send>) {
        self.active_viz.push(visualizer);
    }
    
    pub fn update(&mut self, world: &World, step: u64, time: f64) -> Result<(), Box<dyn std::error::Error>> {
        if step - self.last_update < self.update_interval {
            return Ok(());
        }
        
        let mut data = VisualizationData::from_world(world);
        data.step = step;
        data.time = time;
        
        if let Ok(mut history) = self.data_history.lock() {
            history.push(data.clone());
            
            if history.len() > 1000 {
                history.remove(0);
            }
        }
        
        for viz in self.active_viz.iter_mut() {
            viz.update(&data)?;
        }
        
        self.last_update = step;
        Ok(())
    }
}

/// Трейт для визуализаторов
pub trait Visualizer {
    fn name(&self) -> &str;
    fn update(&mut self, data: &VisualizationData) -> Result<(), Box<dyn std::error::Error>>;
    fn save_snapshot(&self, filename: &str) -> Result<(), Box<dyn std::error::Error>>;
}
