use crate::{VisualizationData, Visualizer};
use plotters::prelude::*;

pub struct ScatterPlotVisualizer {
    output_dir: String,
    current_step: u64,
}

impl ScatterPlotVisualizer {
    pub fn new(output_dir: &str) -> Self {
        std::fs::create_dir_all(output_dir).unwrap();
        Self {
            output_dir: output_dir.to_string(),
            current_step: 0,
        }
    }
    
    fn plot_phase_distribution(&self, data: &VisualizationData) -> Result<(), Box<dyn std::error::Error>> {
        let filename = format!("{}/phase_distribution_{:06}.png", self.output_dir, self.current_step);
        let root = BitMapBackend::new(&filename, (800, 600)).into_drawing_area();
        root.fill(&WHITE)?;
        
        let phases = ["G1", "S", "G2", "M"];
        let phase_enum = [
            cell_dt_core::components::Phase::G1,
            cell_dt_core::components::Phase::S,
            cell_dt_core::components::Phase::G2,
            cell_dt_core::components::Phase::M,
        ];
        
        let counts: Vec<i32> = phase_enum.iter().map(|&p| {
            *data.phase_distribution.get(&p).unwrap_or(&0) as i32
        }).collect();
        
        let max_count = *counts.iter().max().unwrap_or(&1);
        
        let mut chart = ChartBuilder::on(&root)
            .caption(format!("Cell Cycle Phase Distribution (Step {})", self.current_step), ("sans-serif", 30))
            .margin(10)
            .x_label_area_size(40)
            .y_label_area_size(40)
            .build_cartesian_2d(0..4, 0..max_count + 1)?;
        
        chart.configure_mesh()
            .x_labels(4)
            .x_label_formatter(&|x| {
                let idx = *x as usize;
                if idx < phases.len() {
                    phases[idx].to_string()
                } else {
                    format!("Phase {}", idx)
                }
            })
            .draw()?;
        
        chart.draw_series(
            Histogram::vertical(&chart)
                .style(BLUE.filled())
                .data(counts.iter().enumerate().map(|(i, &c)| (i as i32, c)))
        )?;
        
        Ok(())
    }
    
    fn plot_maturity_distribution(&self, data: &VisualizationData) -> Result<(), Box<dyn std::error::Error>> {
        let filename = format!("{}/maturity_distribution_{:06}.png", self.output_dir, self.current_step);
        let root = BitMapBackend::new(&filename, (800, 600)).into_drawing_area();
        root.fill(&WHITE)?;
        
        let bins = 20;
        let mut histogram = vec![0; bins];
        
        for &maturity in &data.centriole_maturity {
            let bin = ((maturity * bins as f32) as usize).min(bins - 1);
            histogram[bin] += 1;
        }
        
        let histogram_i32 = histogram.to_vec();
        let max_count = *histogram_i32.iter().max().unwrap_or(&1);
        
        let mut chart = ChartBuilder::on(&root)
            .caption(format!("Centriole Maturity Distribution (Step {})", self.current_step), ("sans-serif", 30))
            .margin(10)
            .x_label_area_size(40)
            .y_label_area_size(40)
            .build_cartesian_2d(0..bins as i32, 0..max_count + 1)?;
        
        chart.configure_mesh()
            .x_desc("Maturity")
            .y_desc("Count")
            .x_labels(10)
            .x_label_formatter(&|x| format!("{:.1}", *x as f32 / bins as f32))
            .draw()?;
        
        chart.draw_series(
            Histogram::vertical(&chart)
                .style(RED.filled())
                .data(histogram_i32.iter().enumerate().map(|(i, &c)| (i as i32, c)))
        )?;
        
        Ok(())
    }
}

impl Visualizer for ScatterPlotVisualizer {
    fn name(&self) -> &str {
        "ScatterPlotVisualizer"
    }
    
    fn update(&mut self, data: &VisualizationData) -> Result<(), Box<dyn std::error::Error>> {
        self.current_step = data.step;
        
        // Добавляем проверку на наличие данных
        if data.cell_count > 0 {
            self.plot_phase_distribution(data)?;
            self.plot_maturity_distribution(data)?;
        }
        
        Ok(())
    }
    
    fn save_snapshot(&self, filename: &str) -> Result<(), Box<dyn std::error::Error>> {
        println!("Snapshot saved to {}", filename);
        Ok(())
    }
}
