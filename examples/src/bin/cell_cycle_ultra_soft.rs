use cell_dt_core::{
    SimulationManager, SimulationConfig,
    components::{CentriolePair, CellCycleStateExtended, Phase},
};
use centriole_module::CentrioleModule;
use cell_cycle_module::{CellCycleModule, CellCycleParams};
use cell_dt_viz::{
    VisualizationManager,
    ScatterPlotVisualizer,
    HeatmapVisualizer,
    TimeSeriesVisualizer,
};
use rand::Rng;
use std::io::Write;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Cell DT Platform - Ultra Soft Cell Cycle Module ===\n");
    
    std::fs::create_dir_all("cell_cycle_output")?;
    
    // Очень мягкие параметры
    let config = SimulationConfig {
        max_steps: 3000,
        dt: 0.02,  // Еще меньший шаг
        checkpoint_interval: 300,
        num_threads: Some(4),
        seed: Some(42),
        parallel_modules: false,
        cleanup_dead_interval: None,
    };
    
    let max_steps = config.max_steps;
    
    let mut sim = SimulationManager::new(config);
    
    // Регистрируем модуль центриоли
    let centriole_module = CentrioleModule::with_parallel(true);
    sim.register_module(Box::new(centriole_module))?;
    
    // Экстремально мягкие параметры клеточного цикла
    let cell_cycle_params = CellCycleParams {
        base_cycle_time: 10.0,           // Быстрый цикл
        growth_factor_sensitivity: 0.2,
        stress_sensitivity: 0.1,
        checkpoint_strictness: 0.05,      // Почти нет контроля
        enable_apoptosis: false,           // Отключаем апоптоз
        nutrient_availability: 1.0,        // Максимум питательных веществ
        growth_factor_level: 1.0,           // Максимум факторов роста
        random_variation: 0.5,              // Больше случайности
    };
    
    println!("📊 Cell Cycle Parameters (Ultra Soft):");
    println!("   Checkpoint strictness: {:.3}", cell_cycle_params.checkpoint_strictness);
    println!("   Growth factors: {:.2}", cell_cycle_params.growth_factor_level);
    println!("   Nutrients: {:.2}", cell_cycle_params.nutrient_availability);
    println!("   Random variation: {:.2}", cell_cycle_params.random_variation);
    
    let cell_cycle_module = CellCycleModule::with_params(cell_cycle_params);
    sim.register_module(Box::new(cell_cycle_module))?;
    
    // Инициализируем клетки
    initialize_cells(&mut sim, 30)?;
    
    // Настраиваем визуализацию
    let mut viz_manager = VisualizationManager::new(30);
    viz_manager.add_visualizer(Box::new(ScatterPlotVisualizer::new("cell_cycle_output/scatter")));
    viz_manager.add_visualizer(Box::new(HeatmapVisualizer::new("cell_cycle_output/heatmap")));
    
    let data_history = viz_manager.data_history.clone();
    viz_manager.add_visualizer(Box::new(TimeSeriesVisualizer::new("cell_cycle_output/timeseries", data_history)));
    
    println!("\n🚀 Starting simulation with ultra-soft checkpoints...");
    println!("   Expecting cells to divide within {} steps\n", max_steps);
    
    sim.initialize()?;
    
    for step in 0..max_steps {
        sim.step()?;
        
        if step % 100 == 0 {
            viz_manager.update(sim.world(), sim.current_step(), sim.current_time())?;
            print_progress(step, max_steps, &sim);
        }
    }
    
    println!("\n✅ Simulation completed!");
    print_final_stats(&sim);
    
    Ok(())
}

fn initialize_cells(sim: &mut SimulationManager, count: usize) -> Result<(), cell_dt_core::SimulationError> {
    println!("Initializing {} cells with cell cycle states...", count);
    
    let world = sim.world_mut();
    let mut rng = rand::thread_rng();
    
    for i in 0..count {
        let mut cell_cycle = CellCycleStateExtended::new();
        
        // Все клетки начинают в G1 для чистоты эксперимента
        cell_cycle.phase = Phase::G1;
        cell_cycle.progress = rng.gen::<f32>() * 0.3;  // Начинаем с начала фазы
        
        // Даем каждой клетке уникальные характеристики
        cell_cycle.growth_factors.growth_signal = 0.9 + rng.gen::<f32>() * 0.1;
        cell_cycle.growth_factors.nutrient_level = 0.95 + rng.gen::<f32>() * 0.05;
        cell_cycle.growth_factors.stress_level = rng.gen::<f32>() * 0.05;
        
        let _entity = world.spawn((
            CentriolePair::default(),
            cell_cycle,
        ));
        
        if i % 10 == 0 {
            print!(".");
            std::io::stdout().flush()?;
        }
    }
    println!(" done!");
    
    Ok(())
}

fn print_progress(step: u64, max_steps: u64, sim: &SimulationManager) {
    let world = sim.world();
    let mut query = world.query::<&CellCycleStateExtended>();
    
    let mut phase_counts = [0; 4];
    let mut arrested = 0;
    let mut total_cycles = 0;
    let mut total_cells = 0;
    let mut max_cell_cycles = 0;
    
    for (_, cycle) in query.iter() {
        total_cells += 1;
        match cycle.phase {
            Phase::G1 => phase_counts[0] += 1,
            Phase::S => phase_counts[1] += 1,
            Phase::G2 => phase_counts[2] += 1,
            Phase::M => phase_counts[3] += 1,
        }
        
        if cycle.current_checkpoint.is_some() {
            arrested += 1;
        }
        
        total_cycles += cycle.cycle_count;
        max_cell_cycles = max_cell_cycles.max(cycle.cycle_count);
    }
    
    let progress = step as f32 / max_steps as f32 * 100.0;
    println!("\n📊 Step {}/{} ({:.1}%)", step, max_steps, progress);
    println!("   Phases: G1:{:3} S:{:3} G2:{:3} M:{:3}", 
             phase_counts[0], phase_counts[1], phase_counts[2], phase_counts[3]);
    println!("   Arrested: {:3}, Max cycles: {}", arrested, max_cell_cycles);
    if total_cells > 0 {
        println!("   Avg cycles: {:.2}", total_cycles as f32 / total_cells as f32);
    }
}

fn print_final_stats(sim: &SimulationManager) {
    let world = sim.world();
    let mut query = world.query::<&CellCycleStateExtended>();
    
    let mut total_cells = 0;
    let mut total_cycles = 0;
    let mut max_cycles = 0;
    let mut phase_counts = [0; 4];
    let mut arrested = 0;
    let mut cycle_distribution = std::collections::HashMap::new();
    let mut cells_with_cycles = 0;
    
    for (_, cycle) in query.iter() {
        total_cells += 1;
        total_cycles += cycle.cycle_count;
        max_cycles = max_cycles.max(cycle.cycle_count);
        
        if cycle.cycle_count > 0 {
            cells_with_cycles += 1;
        }
        
        *cycle_distribution.entry(cycle.cycle_count).or_insert(0) += 1;
        
        match cycle.phase {
            Phase::G1 => phase_counts[0] += 1,
            Phase::S => phase_counts[1] += 1,
            Phase::G2 => phase_counts[2] += 1,
            Phase::M => phase_counts[3] += 1,
        }
        
        if cycle.current_checkpoint.is_some() {
            arrested += 1;
        }
    }
    
    println!("\n=== Final Statistics ===");
    println!("Total cells: {}", total_cells);
    println!("Cells that completed at least one cycle: {}", cells_with_cycles);
    println!("Phase distribution: G1={}, S={}, G2={}, M={}", 
             phase_counts[0], phase_counts[1], phase_counts[2], phase_counts[3]);
    println!("Cells arrested: {}", arrested);
    if total_cells > 0 {
        println!("Average cycles completed: {:.2}", total_cycles as f32 / total_cells as f32);
    }
    println!("Maximum cycles: {}", max_cycles);
    
    println!("\n=== Cycle Distribution ===");
    let mut cycles: Vec<_> = cycle_distribution.into_iter().collect();
    cycles.sort();
    for (cycles, count) in cycles {
        println!("  {} cycles: {} cells", cycles, count);
    }
}
