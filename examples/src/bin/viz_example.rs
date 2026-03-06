use cell_dt_core::{
    SimulationManager, SimulationConfig,
    components::{CentriolePair, CellCycleState, Phase},
};
use centriole_module::CentrioleModule;
use cell_dt_viz::{
    VisualizationManager,
    ScatterPlotVisualizer,
    HeatmapVisualizer,
    TimeSeriesVisualizer,
    ThreeDVisualizer,
};
use rand::Rng;
use std::io::Write;
use std::time::Duration;
use std::thread;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Cell DT Platform with Real-time Visualization ===\n");
    
    std::fs::create_dir_all("viz_output")?;
    
    let config = SimulationConfig {
        max_steps: 500,
        dt: 0.1,
        checkpoint_interval: 100,
        num_threads: Some(4),
        seed: Some(42),
        parallel_modules: false,
        cleanup_dead_interval: None,
    };
    
    let max_steps = config.max_steps; // Сохраняем значение до перемещения
    
    let mut sim = SimulationManager::new(config);
    
    let centriole_module = CentrioleModule::with_parallel(true);
    sim.register_module(Box::new(centriole_module))?;
    
    initialize_cells(&mut sim, 500)?;
    
    let mut viz_manager = VisualizationManager::new(5);
    
    viz_manager.add_visualizer(Box::new(ScatterPlotVisualizer::new("viz_output/scatter")));
    viz_manager.add_visualizer(Box::new(HeatmapVisualizer::new("viz_output/heatmap")));
    
    let data_history = viz_manager.data_history.clone();
    viz_manager.add_visualizer(Box::new(TimeSeriesVisualizer::new("viz_output/timeseries", data_history)));
    
    let mut viz3d = ThreeDVisualizer::new();
    viz3d.start();
    viz_manager.add_visualizer(Box::new(viz3d));
    
    println!("\n📊 Starting simulation with real-time visualization...");
    println!("   Output will be saved to ./viz_output/");
    println!("   Press Ctrl+C to stop\n");
    
    sim.initialize()?;
    
    for step in 0..max_steps {
        sim.step()?;
        
        viz_manager.update(sim.world(), sim.current_step(), sim.current_time())?;
        
        if step % 50 == 0 {
            println!("   Step {}/{}", step, max_steps);
        }
        
        thread::sleep(Duration::from_millis(10));
    }
    
    println!("\n✅ Simulation completed!");
    println!("   Check viz_output/ directory for generated visualizations");
    
    Ok(())
}

fn initialize_cells(sim: &mut SimulationManager, count: usize) -> Result<(), cell_dt_core::SimulationError> {
    println!("Initializing {} cells...", count);
    
    let world = sim.world_mut();
    let mut rng = rand::thread_rng();
    
    for i in 0..count {
        let _entity = world.spawn((
            CentriolePair::default(),
            CellCycleState {
                phase: match rng.gen_range(0..4) {
                    0 => Phase::G1,
                    1 => Phase::S,
                    2 => Phase::G2,
                    _ => Phase::M,
                },
                progress: rng.gen::<f32>(),
            },
        ));
        
        if i % 100 == 0 {
            print!(".");
            std::io::stdout().flush()?;
        }
    }
    println!(" done!");
    
    Ok(())
}
