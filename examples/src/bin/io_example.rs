use cell_dt_core::{
    SimulationManager, SimulationConfig,
    components::{CentriolePair, CellCycleStateExtended},
};
use centriole_module::CentrioleModule;
use cell_cycle_module::{CellCycleModule, CellCycleParams};
use human_development_module::{HumanDevelopmentModule, HumanTissueType};
use myeloid_shift_module::MyeloidShiftModule;
use cell_dt_io::{
    DataExporter, CdataExporter,
    load_json_config, save_json_config,
    SimulationConfigFull, ModuleConfigs,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Cell DT Platform - Data Export Example ===\n");

    // Директории вывода
    std::fs::create_dir_all("io_output/data")?;
    std::fs::create_dir_all("io_output/cdata")?;
    std::fs::create_dir_all("io_output/configs")?;

    // --- Сохранить/загрузить конфиг ---
    let config_full = SimulationConfigFull {
        simulation: cell_dt_io::SimulationConfig {
            max_steps: 200,
            dt: 0.05,
            num_threads: Some(4),
            seed: Some(42),
        },
        modules: ModuleConfigs {
            centriole: Some(serde_json::json!({
                "acetylation_rate": 0.02,
                "parallel_cells": true,
            })),
            cell_cycle: Some(serde_json::json!({
                "base_cycle_time": 15.0,
                "checkpoint_strictness": 0.1,
            })),
        },
    };
    save_json_config("io_output/configs/config.json", &config_full)?;
    println!("✅ Saved configuration to io_output/configs/config.json");

    let loaded_config = load_json_config("io_output/configs/config.json")?;
    println!("📋 Loaded configuration: simulation steps = {}", loaded_config.simulation.max_steps);

    // --- Инициализация SimulationManager ---
    let sim_config = SimulationConfig {
        max_steps: loaded_config.simulation.max_steps,
        dt: loaded_config.simulation.dt,
        checkpoint_interval: 100,
        num_threads: loaded_config.simulation.num_threads,
        seed: loaded_config.simulation.seed,
        parallel_modules: false,
        cleanup_dead_interval: None,
    };

    let mut sim = SimulationManager::new(sim_config);

    // Регистрируем модули (CDATA-стек)
    sim.register_module(Box::new(CentrioleModule::with_parallel(true)))?;

    let cell_cycle_params = CellCycleParams {
        base_cycle_time: 15.0,
        growth_factor_sensitivity: 0.3,
        stress_sensitivity: 0.2,
        checkpoint_strictness: 0.1,
        enable_apoptosis: true,
        nutrient_availability: 0.9,
        growth_factor_level: 0.8,
        random_variation: 0.2,
    };
    sim.register_module(Box::new(CellCycleModule::with_params(cell_cycle_params)))?;
    sim.register_module(Box::new(HumanDevelopmentModule::new()))?;
    sim.register_module(Box::new(MyeloidShiftModule::new()))?;

    // Спавним клетки (CentriolePair + CellCycleStateExtended —
    // HumanDevelopmentModule добавит HumanDevelopmentComponent в initialize())
    {
        let world = sim.world_mut();
        for _ in 0..5 {
            world.spawn((CentriolePair::default(), CellCycleStateExtended::new()));
        }
    }

    // Явно спавним ещё клетки через HumanDevelopmentModule для демонстрации CDATA-экспорта
    sim.world_mut().spawn((
        CellCycleStateExtended::new(),
        human_development_module::HumanDevelopmentComponent::for_tissue(HumanTissueType::Blood),
    ));
    sim.world_mut().spawn((
        CellCycleStateExtended::new(),
        human_development_module::HumanDevelopmentComponent::for_tissue(HumanTissueType::Neural),
    ));

    // --- Экспортёры ---
    let mut basic_exporter = DataExporter::new("io_output/data", "simulation");
    let mut cdata_exporter = CdataExporter::new("io_output/cdata", "cdata");

    println!("\n🚀 Starting simulation with data export...");
    println!("   Basic cell data  → io_output/data/");
    println!("   CDATA metrics    → io_output/cdata/\n");

    sim.initialize()?;

    for step in 0..sim.config().max_steps {
        sim.step()?;

        // Базовый экспорт: каждые 10 шагов
        if step % 10 == 0 {
            basic_exporter.collect_data(sim.world(), sim.current_step(), sim.current_time())?;
        }

        // CDATA-экспорт: каждые 10 шагов
        if step % 10 == 0 {
            cdata_exporter.collect(sim.world(), sim.current_step());
        }

        // Сохранение снимков каждые 50 шагов
        if step % 50 == 0 && step > 0 {
            if basic_exporter.buffered() > 0 {
                let path = basic_exporter.save_snapshot(step)?;
                println!("   💾 Basic snapshot: {}", path.display());
            }
            let path = cdata_exporter.save_snapshot(step)?;
            println!("   📊 CDATA snapshot: {} ({} records)",
                path.display(), cdata_exporter.buffered_records());
        }

        if step % 50 == 0 {
            println!("   Step {}/{}", step, sim.config().max_steps);
        }
    }

    // Финальный экспорт
    println!("\n📊 Final export...");
    let step = sim.current_step();
    if basic_exporter.buffered() > 0 {
        let path = basic_exporter.save_snapshot(step)?;
        println!("   ✅ Basic data:  {}", path.display());
    }
    let path = cdata_exporter.save_snapshot(step)?;
    println!("   ✅ CDATA data:  {}", path.display());

    println!("\n=== Summary ===");
    println!("Total steps:    {}", sim.current_step());
    println!("Total entities: {}", sim.world().len());

    Ok(())
}
