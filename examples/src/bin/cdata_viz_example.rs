//! Пример CDATA-визуализации: временные ряды 4 ключевых метрик.
//!
//! Запуск:
//! ```bash
//! cargo run --bin cdata_viz_example
//! ```
//! Результат: `viz_output/cdata_timeseries.png` — 4-панельный PNG-график.

use cell_dt_core::{SimulationManager, SimulationConfig};
use centriole_module::CentrioleModule;
use cell_cycle_module::CellCycleModule;
use human_development_module::{HumanDevelopmentModule, HumanTissueType};
use myeloid_shift_module::MyeloidShiftModule;
use cell_dt_viz::CdataTimeSeriesVisualizer;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("=== CDATA Visualization Example ===\n");
    std::fs::create_dir_all("viz_output")?;

    // Симуляция: 100 лет × 12 шагов/год = 1200 шагов; dt = 1/12 года
    let config = SimulationConfig {
        max_steps: 1200,
        dt: 1.0 / 12.0,
        checkpoint_interval: 100,
        num_threads: None,
        seed: Some(42),
        parallel_modules: false,
        cleanup_dead_interval: Some(200),
    };

    let mut sim = SimulationManager::new(config);

    sim.register_module(Box::new(CentrioleModule::new()))?;
    sim.register_module(Box::new(CellCycleModule::new()))?;
    sim.register_module(Box::new(HumanDevelopmentModule::new()))?;
    sim.register_module(Box::new(MyeloidShiftModule::new()))?;

    // 5 ниш разных тканей
    {
        let world = sim.world_mut();
        use cell_dt_core::components::CellCycleStateExtended;
        use human_development_module::HumanDevelopmentComponent;
        for tissue in &[
            HumanTissueType::Blood,
            HumanTissueType::Neural,
            HumanTissueType::Epithelial,
            HumanTissueType::Muscle,
            HumanTissueType::Skin,
        ] {
            world.spawn((
                CellCycleStateExtended::new(),
                HumanDevelopmentComponent::for_tissue(*tissue),
            ));
        }
    }

    // Визуализатор: собирать каждые 12 шагов (~1 год)
    let mut viz = CdataTimeSeriesVisualizer::new(12);

    println!("Running simulation (1200 steps ≈ 100 years)...");
    sim.initialize()?;

    for _ in 0..sim.config().max_steps {
        sim.step()?;
        viz.collect(sim.world(), sim.current_step());
    }

    println!("Collected {} snapshots", viz.snapshot_count());

    let out_path = "viz_output/cdata_timeseries.png";
    viz.plot(out_path)?;
    println!("✅ Plot saved: {}", out_path);

    Ok(())
}
