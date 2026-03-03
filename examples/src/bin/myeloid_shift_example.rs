//! Пример CDATA-симуляции с миелоидным сдвигом.
//!
//! Демонстрирует как накопление повреждений центриоли (CDATA) постепенно
//! сдвигает стволовые клетки от лимфоидного пути к миелоидному.
//!
//! ## Порядок модулей
//! 1. `CentrioleModule`       — (заглушка) базовый учёт центриоли
//! 2. `CellCycleModule`       — фазы клеточного цикла
//! 3. `HumanDevelopmentModule` — CDATA: накопление повреждений + O₂-индукторы
//! 4. `MyeloidShiftModule`    — читает CentriolarDamageState, пишет myeloid_bias
//!                             и InflammagingState (обратная связь на шаг N+1)
//!
//! ## Вывод каждые 10 лет
//! ```
//! Year  Stage               Damage   Spindle   Cilia   mBias  Phenotype   ROS_boost
//! ```

use cell_dt_core::{SimulationManager, SimulationConfig};
use cell_dt_core::components::{CentriolePair, CellCycleStateExtended};
use centriole_module::CentrioleModule;
use cell_cycle_module::{CellCycleModule, CellCycleParams};
use human_development_module::{
    HumanDevelopmentModule, HumanDevelopmentParams,
    HumanDevelopmentComponent,
};
use myeloid_shift_module::{MyeloidShiftModule, MyeloidShiftComponent};
use cell_dt_core::components::InflammagingState;
use std::io::Write;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Cell DT Platform — CDATA Myeloid Shift Simulation ===\n");
    println!("Theory: Centriolar Damage → Myeloid Bias (CDATA, Tkemaladze 2007–2023)\n");

    let config = SimulationConfig {
        max_steps: 40_000,
        dt: 1.0,
        checkpoint_interval: 3650,
        num_threads: Some(4),
        seed: Some(42),
        parallel_modules: false,
    };

    let mut sim = SimulationManager::new(config);

    // 1. Центриольный модуль (заглушка)
    sim.register_module(Box::new(CentrioleModule::with_parallel(true)))?;

    // 2. Клеточный цикл
    let cell_cycle_params = CellCycleParams {
        base_cycle_time:           24.0,
        growth_factor_sensitivity: 0.3,
        stress_sensitivity:        0.2,
        checkpoint_strictness:     0.1,
        enable_apoptosis:          true,
        nutrient_availability:     0.9,
        growth_factor_level:       0.8,
        random_variation:          0.2,
    };
    sim.register_module(Box::new(CellCycleModule::with_params(cell_cycle_params)))?;

    // 3. Модуль развития человека (CDATA-ядро)
    let dev_params = HumanDevelopmentParams {
        time_acceleration:       1.0,
        enable_aging:            true,
        enable_morphogenesis:    true,
        tissue_detail_level:     3,
        mother_inducer_count:    10,
        daughter_inducer_count:  8,
        base_detach_probability: 0.002,
        mother_bias:             0.6,
        age_bias_coefficient:    0.003,
    };
    sim.register_module(Box::new(HumanDevelopmentModule::with_params(dev_params)))?;

    // 4. Модуль миелоидного сдвига (регистрируется ПОСЛЕ human_development_module)
    sim.register_module(Box::new(MyeloidShiftModule::new()))?;

    // Создаём ниши
    initialize_niches(&mut sim, 5)?;

    println!("{:<6} {:<20} {:>7} {:>8} {:>8} {:>7} {:<16} {:>8}",
        "Year", "Stage", "Damage", "Spindle", "Cilia",
        "mBias", "Phenotype", "ROSboost");
    println!("{}", "-".repeat(90));

    sim.initialize()?;

    for year in 0usize..100 {
        for _ in 0..365 {
            sim.step()?;
        }
        if year % 10 == 0 || year == 99 {
            print_year_status(year, &sim);
            std::io::stdout().flush()?;
        }
    }

    println!("\n=== Simulation completed ===");
    print_final_status(&sim);

    Ok(())
}

fn initialize_niches(
    sim: &mut SimulationManager,
    count: usize,
) -> Result<(), cell_dt_core::SimulationError> {
    println!("Spawning {} stem cell niches...", count);
    let world = sim.world_mut();
    for i in 0..count {
        let _ = world.spawn((
            CentriolePair::default(),
            CellCycleStateExtended::new(),
        ));
        println!("  Niche {} spawned", i + 1);
    }
    Ok(())
}

fn print_year_status(year: usize, sim: &SimulationManager) {
    let world = sim.world();

    // Ищем первую живую нишу
    let mut dev_query = world.query::<&HumanDevelopmentComponent>();
    let myeloid_data: Vec<_> = {
        let mut m_query = world.query::<&MyeloidShiftComponent>();
        m_query.iter().map(|(e, m)| (e, m.clone())).collect()
    };
    let infl_data: Vec<_> = {
        let mut i_query = world.query::<&InflammagingState>();
        i_query.iter().map(|(e, i)| (e, i.clone())).collect()
    };

    if let Some((entity, dev)) = dev_query.iter().find(|(_, d)| d.is_alive) {
        let stage_str = stage_name(dev.stage);
        let damage    = dev.damage_score();
        let spindle   = dev.centriolar_damage.spindle_fidelity;
        let cilia     = dev.centriolar_damage.ciliary_function;

        let myeloid_bias = myeloid_data.iter()
            .find(|(e, _)| *e == entity)
            .map(|(_, m)| m.myeloid_bias)
            .unwrap_or(0.0);

        let phenotype_str = myeloid_data.iter()
            .find(|(e, _)| *e == entity)
            .map(|(_, m)| format!("{:?}", m.phenotype))
            .unwrap_or_else(|| "N/A".to_string());

        let ros_boost = infl_data.iter()
            .find(|(e, _)| *e == entity)
            .map(|(_, i)| i.ros_boost)
            .unwrap_or(0.0);

        println!("{:<6} {:<20} {:>7.3} {:>8.3} {:>8.3} {:>7.3} {:<16} {:>8.4}",
            year, stage_str, damage, spindle, cilia,
            myeloid_bias, phenotype_str, ros_boost);
    } else {
        println!("{:<6} [all niches exhausted]", year);
    }
}

fn print_final_status(sim: &SimulationManager) {
    let world = sim.world();

    println!("\n{:<12} {:<14} {:>8} {:>8} {:>8} {:>7} {:<16} {:>10}",
        "Tissue", "Status",
        "Age(yr)", "Damage", "Spindle",
        "mBias", "Phenotype", "ImmuneIdx");
    println!("{}", "-".repeat(90));

    let myeloid_map: std::collections::HashMap<_, _> = {
        let mut q = world.query::<&MyeloidShiftComponent>();
        q.iter().map(|(e, m)| (e, m.clone())).collect()
    };

    let mut alive = 0u32;
    let mut dead  = 0u32;

    let mut dev_query = world.query::<&HumanDevelopmentComponent>();
    for (entity, dev) in dev_query.iter() {
        let tissue  = format!("{:?}", dev.tissue_type);
        let status  = if dev.is_alive { "alive" } else { "dead" };
        let myeloid = myeloid_map.get(&entity);

        println!("{:<12} {:<14} {:>8.1} {:>8.3} {:>8.3} {:>7.3} {:<16} {:>10.3}",
            tissue, status,
            dev.age_years(),
            dev.damage_score(),
            dev.centriolar_damage.spindle_fidelity,
            myeloid.map_or(0.0, |m| m.myeloid_bias),
            myeloid.map_or("N/A".to_string(), |m| format!("{:?}", m.phenotype)),
            myeloid.map_or(0.0, |m| m.immune_senescence));

        if dev.is_alive { alive += 1; } else { dead += 1; }
    }

    println!("\nAlive niches: {}   Dead niches: {}", alive, dead);

    // Детали первой живой ниши
    let mut dev_query2 = world.query::<&HumanDevelopmentComponent>();
    if let Some((entity, dev)) = dev_query2.iter().find(|(_, d)| d.is_alive) {
        if let Some(m) = myeloid_map.get(&entity) {
            println!("\n=== Myeloid shift details (niche {:?}) ===", dev.tissue_type);
            println!("  Myeloid bias:         {:.3}  ({:?})", m.myeloid_bias, m.phenotype);
            println!("  Lymphoid deficit:     {:.3}", m.lymphoid_deficit);
            println!("  Inflammaging index:   {:.3}", m.inflammaging_index);
            println!("  Immune senescence:    {:.3}", m.immune_senescence);
        }
        println!("\n=== Centriolar damage (niche {:?}) ===", dev.tissue_type);
        let d = &dev.centriolar_damage;
        println!("  Protein carbonylation:    {:.3}", d.protein_carbonylation);
        println!("  Tubulin hyperacetylation: {:.3}", d.tubulin_hyperacetylation);
        println!("  Protein aggregates:       {:.3}", d.protein_aggregates);
        println!("  Phospho-dysregulation:    {:.3}", d.phosphorylation_dysregulation);
        println!("  ROS level:                {:.3}", d.ros_level);
        println!("  CEP164 integrity:         {:.3}", d.cep164_integrity);
        println!("  Ciliary function (Trk A): {:.3}", d.ciliary_function);
        println!("  Spindle fidelity (Trk B): {:.3}", d.spindle_fidelity);
        println!("  Frailty index:            {:.3}", dev.frailty());
        println!("  Active phenotypes ({}):", dev.active_phenotypes.len());
        for ph in &dev.active_phenotypes {
            println!("    - {:?}", ph);
        }
    }
}

fn stage_name(stage: human_development_module::HumanDevelopmentalStage) -> &'static str {
    use human_development_module::HumanDevelopmentalStage::*;
    match stage {
        Zygote        => "Zygote",
        Cleavage      => "Cleavage",
        Morula        => "Morula",
        Blastocyst    => "Blastocyst",
        Implantation  => "Implantation",
        Gastrulation  => "Gastrulation",
        Neurulation   => "Neurulation",
        Organogenesis => "Organogenesis",
        Fetal         => "Fetal",
        Newborn       => "Newborn",
        Childhood     => "Childhood",
        Adolescence   => "Adolescence",
        Adult         => "Adult",
        MiddleAge     => "Middle Age",
        Elderly       => "Elderly",
    }
}
