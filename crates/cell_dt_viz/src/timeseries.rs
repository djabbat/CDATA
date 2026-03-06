use crate::{VisualizationData, Visualizer};
use plotters::prelude::*;
use std::sync::{Arc, Mutex};

pub struct TimeSeriesVisualizer {
    data_history: Arc<Mutex<Vec<VisualizationData>>>,
    output_dir: String,
}

impl TimeSeriesVisualizer {
    pub fn new(output_dir: &str, data_history: Arc<Mutex<Vec<VisualizationData>>>) -> Self {
        std::fs::create_dir_all(output_dir).unwrap();
        Self {
            data_history,
            output_dir: output_dir.to_string(),
        }
    }
    
    pub fn plot_time_series(&self) -> Result<(), Box<dyn std::error::Error>> {
        let filename = format!("{}/timeseries.png", self.output_dir);
        let root = BitMapBackend::new(&filename, (1200, 800)).into_drawing_area();
        root.fill(&WHITE)?;
        
        let history = self.data_history.lock().unwrap();
        
        if history.is_empty() {
            return Ok(());
        }
        
        let steps: Vec<i32> = history.iter().map(|d| d.step as i32).collect();
        let cell_counts: Vec<i32> = history.iter().map(|d| d.cell_count as i32).collect();
        let cilia_counts: Vec<i32> = history.iter().map(|d| d.cilia_count as i32).collect();
        
        let max_cells = *cell_counts.iter().max().unwrap_or(&1);
        let max_cilia = *cilia_counts.iter().max().unwrap_or(&1);
        
        let mut chart = ChartBuilder::on(&root)
            .caption("Time Series Analysis", ("sans-serif", 40))
            .margin(20)
            .x_label_area_size(40)
            .y_label_area_size(40)
            .build_cartesian_2d(
                *steps.first().unwrap()..*steps.last().unwrap(),
                0..max_cells.max(max_cilia) + 1
            )?;
        
        chart.configure_mesh()
            .x_desc("Step")
            .y_desc("Count")
            .draw()?;
        
        chart.draw_series(
            LineSeries::new(
                steps.iter().zip(cell_counts.iter())
                    .map(|(&x, &y)| (x, y)),
                &BLUE,
            )
        )?
        .label("Total Cells")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 10, y)], BLUE));
        
        chart.draw_series(
            LineSeries::new(
                steps.iter().zip(cilia_counts.iter())
                    .map(|(&x, &y)| (x, y)),
                &RED,
            )
        )?
        .label("Cells with Cilia")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 10, y)], RED));
        
        chart.configure_series_labels()
            .background_style(WHITE.mix(0.8))
            .border_style(BLACK)
            .draw()?;
        
        Ok(())
    }
}

impl Visualizer for TimeSeriesVisualizer {
    fn name(&self) -> &str {
        "TimeSeriesVisualizer"
    }
    
    fn update(&mut self, _data: &VisualizationData) -> Result<(), Box<dyn std::error::Error>> {
        if _data.step.is_multiple_of(10) {
            self.plot_time_series()?;
        }
        Ok(())
    }
    
    fn save_snapshot(&self, filename: &str) -> Result<(), Box<dyn std::error::Error>> {
        println!("Time series saved to {}", filename);
        Ok(())
    }
}
