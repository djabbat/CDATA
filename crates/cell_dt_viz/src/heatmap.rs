use crate::{VisualizationData, Visualizer};
use plotters::prelude::*;

pub struct HeatmapVisualizer {
    output_dir: String,
    current_step: u64,
}

impl HeatmapVisualizer {
    pub fn new(output_dir: &str) -> Self {
        std::fs::create_dir_all(output_dir).unwrap();
        Self {
            output_dir: output_dir.to_string(),
            current_step: 0,
        }
    }
    
    pub fn plot_activity_heatmap(&self, data: &VisualizationData) -> Result<(), Box<dyn std::error::Error>> {
        let filename = format!("{}/activity_heatmap_{:06}.png", self.output_dir, self.current_step);
        let root = BitMapBackend::new(&filename, (800, 600)).into_drawing_area();
        root.fill(&WHITE)?;
        
        let size = 20;
        let mut matrix = vec![vec![0.0; size]; size];
        
        for i in 0..data.cell_count.min(400) {
            let x = i % size;
            let y = i / size;
            if y < size && i < data.mtoc_activity.len() {
                matrix[x][y] = data.mtoc_activity[i];
            }
        }
        
        let mut chart = ChartBuilder::on(&root)
            .caption(format!("Cellular Activity Heatmap (Step {})", self.current_step), ("sans-serif", 30))
            .margin(10)
            .x_label_area_size(40)
            .y_label_area_size(40)
            .build_cartesian_2d(0..size as i32, 0..size as i32)?;
        
        chart.configure_mesh()
            .x_desc("Cell X Position")
            .y_desc("Cell Y Position")
            .draw()?;
        
        for (i, row) in matrix.iter().enumerate() {
            for (j, &value) in row.iter().enumerate() {
                let color = if value > 0.8 {
                    RED
                } else if value > 0.6 {
                    YELLOW
                } else if value > 0.4 {
                    GREEN
                } else if value > 0.2 {
                    CYAN
                } else {
                    BLUE
                };

                let rect = Rectangle::new(
                    [(i as i32, j as i32), (i as i32 + 1, j as i32 + 1)],
                    color.filled(),
                );
                chart.draw_series(std::iter::once(rect))?;
            }
        }
        
        Ok(())
    }
}

impl Visualizer for HeatmapVisualizer {
    fn name(&self) -> &str {
        "HeatmapVisualizer"
    }
    
    fn update(&mut self, data: &VisualizationData) -> Result<(), Box<dyn std::error::Error>> {
        self.current_step = data.step;
        self.plot_activity_heatmap(data)?;
        Ok(())
    }
    
    fn save_snapshot(&self, filename: &str) -> Result<(), Box<dyn std::error::Error>> {
        println!("Heatmap snapshot saved to {}", filename);
        Ok(())
    }
}
