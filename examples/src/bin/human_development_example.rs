//! Пример CDATA-симуляции развития человека от зиготы до смерти.
//!
//! ## Временной масштаб
//! * `dt = 1.0`  — каждый шаг = 1 день
//! * `time_acceleration = 1.0` — без ускорения
//! * Внешний цикл: 100 лет × 365 шагов/год = 36 500 шагов
//! * `max_steps = 40_000` — с запасом
//!
//! ## Что выводится
//! Каждые 10 лет: возраст, стадия, морфогенетический уровень,
//! суммарный ущерб центриоли, функция реснички, точность веретена,
//! индекс дряхлости, число активных фенотипов старения.

use cell_dt_core::{SimulationManager, SimulationConfig};
use cell_dt_core::components::{CentriolePair, CellCycleStateExtended};
use centriole_module::CentrioleModule;
use cell_cycle_module::{CellCycleModule, CellCycleParams};
use human_development_module::{
    HumanDevelopmentModule, HumanDevelopmentParams,
    HumanDevelopmentalStage, HumanMorphogeneticLevel,
    HumanDevelopmentComponent,
};
use std::io::Write;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Cell DT Platform — CDATA Human Development Simulation ===\n");
    println!("Theory: Centriolar Damage Accumulation Theory of Aging");
    println!("        (Jaba Tkemaladze, 2007–2023)\n");

    // --- Конфигурация ---
    // dt = 1.0 день; 100 лет × 365 дней/год = 36 500 шагов
    let config = SimulationConfig {
        max_steps: 40_000,
        dt: 1.0,           // 1 день на шаг
        checkpoint_interval: 3650,
        num_threads: Some(4),
        seed: Some(42),
        parallel_modules: false,
        cleanup_dead_interval: None,
    };

    let mut sim = SimulationManager::new(config);

    // 1. Модуль центриоли
    sim.register_module(Box::new(CentrioleModule::with_parallel(true)))?;
    println!("[OK] Centriole module registered");

    // 2. Модуль клеточного цикла
    let cell_cycle_params = CellCycleParams {
        base_cycle_time:          24.0,
        growth_factor_sensitivity: 0.3,
        stress_sensitivity:        0.2,
        checkpoint_strictness:     0.1,
        enable_apoptosis:          true,
        nutrient_availability:     0.9,
        growth_factor_level:       0.8,
        random_variation:          0.2,
    };
    sim.register_module(Box::new(CellCycleModule::with_params(cell_cycle_params)))?;
    println!("[OK] Cell cycle module registered");

    // 3. Модуль развития человека (CDATA)
    let dev_params = HumanDevelopmentParams {
        time_acceleration:       1.0,   // 1 шаг = 1 день
        enable_aging:            true,
        enable_morphogenesis:    true,
        tissue_detail_level:     3,
        mother_inducer_count:    10,
        daughter_inducer_count:  8,
        base_detach_probability: 0.002,
        mother_bias:             0.5,   // одинаковая вероятность M и D
        age_bias_coefficient:    0.0,   // возраст не влияет
        ptm_exhaustion_scale:    0.001, // PTM-асимметрия → истощение матери
        de_novo_centriole_division:   4,    // 16-клеточная стадия (Морула)
        meiotic_elimination_enabled: true,
    };
    sim.register_module(Box::new(HumanDevelopmentModule::with_params(dev_params)))?;
    println!("[OK] Human development module (CDATA) registered");

    // --- Сущности (одна на тип стволовой ниши) ---
    initialize_cells(&mut sim, 5)?;

    println!("\n=== Launching simulation: 100 years, 1 step = 1 day ===\n");
    println!("{:<6} {:<20} {:<14} {:>8} {:>8} {:>8} {:>8} {:>10}",
        "Year", "Stage", "Level",
        "Damage", "Cilia", "Spindle", "Frailty", "Phenotypes");
    println!("{}", "-".repeat(90));

    sim.initialize()?;

    // Основной цикл: 100 лет по 365 шагов
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

fn initialize_cells(
    sim: &mut SimulationManager,
    count: usize,
) -> Result<(), cell_dt_core::SimulationError> {
    println!("\nSpawning {} stem cell niches...", count);
    let world = sim.world_mut();
    for i in 0..count {
        let _ = world.spawn((
            CentriolePair::default(),
            CellCycleStateExtended::new(),
        ));
        print!("  Niche {} spawned\n", i + 1);
    }
    Ok(())
}

/// Вывести статус для одной репрезентативной ниши (первой живой).
fn print_year_status(year: usize, sim: &SimulationManager) {
    let world = sim.world();
    let mut query = world.query::<&HumanDevelopmentComponent>();

    if let Some((_, comp)) = query.iter().find(|(_, c)| c.is_alive) {
        let stage_str = stage_name(comp.stage);
        let level_str = level_name(comp.morphogenetic_level);
        let damage  = comp.damage_score();
        let cilia   = comp.centriolar_damage.ciliary_function;
        let spindle = comp.centriolar_damage.spindle_fidelity;
        let frailty = comp.frailty();
        let phenos  = comp.active_phenotypes.len();

        println!("{:<6} {:<20} {:<14} {:>7.3} {:>8.3} {:>8.3} {:>8.3} {:>10}",
            year, stage_str, level_str,
            damage, cilia, spindle, frailty, phenos);
    } else {
        println!("{:<6} [all niches exhausted]", year);
    }
}

fn print_final_status(sim: &SimulationManager) {
    let world = sim.world();
    let mut query = world.query::<&HumanDevelopmentComponent>();

    println!("\n{:<12} {:<14} {:>8} {:>8} {:>8} {:>8} {:>8} {:>6}/{:<6} {:>14}",
        "Tissue", "Status",
        "Age(yr)", "Damage", "Cilia", "Spindle", "Frailty", "M", "D", "Potency");
    println!("{}", "-".repeat(95));

    let mut alive = 0u32;
    let mut dead  = 0u32;

    for (_, comp) in query.iter() {
        let tissue = format!("{:?}", comp.tissue_type);
        let status = if comp.is_alive { "alive" } else { "dead" };
        let potency = format!("{:?}", comp.inducers.potency_level());
        println!("{:<12} {:<14} {:>8.1} {:>8.3} {:>8.3} {:>8.3} {:>8.3} {:>6}/{:<6} {:>14}",
            tissue, status,
            comp.age_years(),
            comp.damage_score(),
            comp.centriolar_damage.ciliary_function,
            comp.centriolar_damage.spindle_fidelity,
            comp.frailty(),
            comp.inducers.mother_set.remaining,
            comp.inducers.daughter_set.remaining,
            potency);

        if comp.is_alive { alive += 1; } else { dead += 1; }
    }

    println!("\nAlive niches: {}   Dead niches: {}", alive, dead);

    // Показать какие фенотипы активны у первой живой ниши
    let mut query2 = world.query::<&HumanDevelopmentComponent>();
    if let Some((_, comp)) = query2.iter().find(|(_, c)| c.is_alive) {
        if !comp.active_phenotypes.is_empty() {
            println!("\nActive aging phenotypes (niche {:?}):", comp.tissue_type);
            for ph in &comp.active_phenotypes {
                println!("  - {:?}", ph);
            }
        }
        println!("\nCentriolar damage details (niche {:?}):", comp.tissue_type);
        let d = &comp.centriolar_damage;
        println!("  Protein carbonylation:       {:.3}", d.protein_carbonylation);
        println!("  Tubulin hyperacetylation:    {:.3}", d.tubulin_hyperacetylation);
        println!("  Protein aggregates:          {:.3}", d.protein_aggregates);
        println!("  Phospho-dysregulation:       {:.3}", d.phosphorylation_dysregulation);
        println!("  CEP164 integrity:            {:.3}", d.cep164_integrity);
        println!("  CEP89  integrity:            {:.3}", d.cep89_integrity);
        println!("  Ninein integrity:            {:.3}", d.ninein_integrity);
        println!("  CEP170 integrity:            {:.3}", d.cep170_integrity);
        println!("  ROS level:                   {:.3}", d.ros_level);
        println!("  Ciliary function (Track A):  {:.3}", d.ciliary_function);
        println!("  Spindle fidelity (Track B):  {:.3}", d.spindle_fidelity);
        println!("  M-inducers remaining:        {}", comp.inducers.mother_set.remaining);
        println!("  D-inducers remaining:        {}", comp.inducers.daughter_set.remaining);
        println!("  Potency level:               {:?}", comp.inducers.potency_level());
        println!("  Stem cell pool:              {:.3}", comp.tissue_state.stem_cell_pool);
        println!("  Senescent fraction:          {:.3}", comp.tissue_state.senescent_fraction);
        println!("  Frailty index:               {:.3}", comp.frailty());
    }
}

fn stage_name(stage: HumanDevelopmentalStage) -> &'static str {
    match stage {
        HumanDevelopmentalStage::Zygote       => "Zygote",
        HumanDevelopmentalStage::Cleavage     => "Cleavage",
        HumanDevelopmentalStage::Morula       => "Morula",
        HumanDevelopmentalStage::Blastocyst   => "Blastocyst",
        HumanDevelopmentalStage::Implantation => "Implantation",
        HumanDevelopmentalStage::Gastrulation => "Gastrulation",
        HumanDevelopmentalStage::Neurulation  => "Neurulation",
        HumanDevelopmentalStage::Organogenesis => "Organogenesis",
        HumanDevelopmentalStage::Fetal        => "Fetal",
        HumanDevelopmentalStage::Newborn      => "Newborn",
        HumanDevelopmentalStage::Childhood    => "Childhood",
        HumanDevelopmentalStage::Adolescence  => "Adolescence",
        HumanDevelopmentalStage::Adult        => "Adult",
        HumanDevelopmentalStage::MiddleAge    => "Middle Age",
        HumanDevelopmentalStage::Elderly      => "Elderly",
    }
}

fn level_name(level: HumanMorphogeneticLevel) -> &'static str {
    match level {
        HumanMorphogeneticLevel::Embryonic => "Embryonic",
        HumanMorphogeneticLevel::Fetal     => "Fetal",
        HumanMorphogeneticLevel::Prenatal  => "Prenatal",
        HumanMorphogeneticLevel::Postnatal => "Postnatal",
        HumanMorphogeneticLevel::Adult     => "Adult",
        HumanMorphogeneticLevel::Aging     => "Aging",
    }
}
