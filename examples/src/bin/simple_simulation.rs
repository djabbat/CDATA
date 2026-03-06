use cell_dt_core::{
    SimulationManager, SimulationConfig,
    components::{CentriolePair, CellCycleState, Phase},
};
use centriole_module::CentrioleModule;
use rand::Rng;
use std::io::Write;  // Добавляем этот импорт для flush()

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    
    println!("=== Cell Differentiation Platform Simulation ===");
    println!("Example with parallel cell processing\n");
    
    let config = SimulationConfig {
        max_steps: 200,
        dt: 0.1,
        checkpoint_interval: 50,
        num_threads: Some(4),
        seed: Some(42),
        parallel_modules: false,
        cleanup_dead_interval: None,
    };
    
    let mut sim = SimulationManager::new(config);
    
    // Создаем модуль центриоли с параллельной обработкой клеток
    let centriole_module = CentrioleModule::with_parallel(true);
    sim.register_module(Box::new(centriole_module))?;
    
    // Создаем 1000 клеток
    initialize_cells(&mut sim, 1000)?;
    
    println!("\n📊 Initial state:");
    print_statistics(&sim);
    
    println!("\n⚡ Starting simulation with parallel cell processing...");
    sim.run()?;
    
    println!("\n✅ Simulation finished!");
    print_statistics(&sim);
    
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
        
        if i % 200 == 0 {
            print!(".");
            std::io::stdout().flush().unwrap();  // Теперь flush() работает
        }
    }
    println!(" done!");
    
    Ok(())
}

fn print_statistics(sim: &SimulationManager) {
    let world = sim.world();
    let mut query = world.query::<(&CentriolePair, &CellCycleState)>();
    
    let mut total_cells = 0;
    let mut total_mother_maturity = 0.0;
    let mut total_daughter_maturity = 0.0;
    let mut total_mtoc_activity = 0.0;
    let mut total_cafds = 0;
    let mut cells_with_cilia = 0;
    let mut phase_counts = [0; 4];
    
    for (_, (pair, cycle)) in query.iter() {
        total_cells += 1;
        total_mother_maturity += pair.mother.maturity;
        total_daughter_maturity += pair.daughter.maturity;
        total_mtoc_activity += pair.mtoc_activity;
        total_cafds += pair.mother.associated_cafds.len();
        
        if pair.cilium_present {
            cells_with_cilia += 1;
        }
        
        match cycle.phase {
            Phase::G1 => phase_counts[0] += 1,
            Phase::S => phase_counts[1] += 1,
            Phase::G2 => phase_counts[2] += 1,
            Phase::M => phase_counts[3] += 1,
        }
    }
    
    println!("\n📈 Statistics at step {}:", sim.current_step());
    println!("  Time: {:.2}", sim.current_time());
    println!("  Total cells: {}", total_cells);
    
    if total_cells > 0 {
        println!("\n  🔬 Centriole statistics:");
        println!("    Mother maturity: {:.3}", total_mother_maturity / total_cells as f32);
        println!("    Daughter maturity: {:.3}", total_daughter_maturity / total_cells as f32);
        println!("    MTOC activity: {:.3}", total_mtoc_activity / total_cells as f32);
        println!("    Avg CAFDs per cell: {:.2}", total_cafds as f32 / total_cells as f32);
        println!("    Cells with cilia: {} ({:.1}%)", 
                 cells_with_cilia, 
                 cells_with_cilia as f32 / total_cells as f32 * 100.0);
        
        println!("\n  🔄 Cell cycle phases:");
        println!("    G1: {} cells ({:.1}%)", phase_counts[0], 
                 phase_counts[0] as f32 / total_cells as f32 * 100.0);
        println!("    S:  {} cells ({:.1}%)", phase_counts[1],
                 phase_counts[1] as f32 / total_cells as f32 * 100.0);
        println!("    G2: {} cells ({:.1}%)", phase_counts[2],
                 phase_counts[2] as f32 / total_cells as f32 * 100.0);
        println!("    M:  {} cells ({:.1}%)", phase_counts[3],
                 phase_counts[3] as f32 / total_cells as f32 * 100.0);
    }
}
