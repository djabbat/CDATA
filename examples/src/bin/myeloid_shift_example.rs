//! Пример CDATA-симуляции с миелоидным сдвигом и мониторингом индукторов.
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
//! Year  Stage          Damage  Spindle   Cilia  M-ind  ΔM  D-ind  ΔD  Potency         mBias  Phenotype
//! ```
//! Столбцы ΔM/ΔD показывают изменение числа индукторов за 10-летний интервал.

use cell_dt_core::{SimulationManager, SimulationConfig};
use cell_dt_core::components::{CentriolePair, CellCycleStateExtended};
use centriole_module::CentrioleModule;
use cell_cycle_module::{CellCycleModule, CellCycleParams};
use human_development_module::{
    HumanDevelopmentModule, HumanDevelopmentParams,
    HumanDevelopmentComponent,
};
use myeloid_shift_module::{MyeloidShiftModule, MyeloidShiftComponent};
use cell_dt_core::components::{InflammagingState, TelomereState};
use std::io::Write;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Cell DT Platform — CDATA Myeloid Shift + Inductor Monitoring ===\n");
    println!("Theory: Centriolar Damage → Inductor Loss → Myeloid Bias (CDATA, Tkemaladze)\n");

    let config = SimulationConfig {
        max_steps: 40_000,
        dt: 1.0,
        checkpoint_interval: 3650,
        num_threads: Some(4),
        seed: Some(42),
        parallel_modules: false,
        cleanup_dead_interval: None,
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
        mother_bias:             0.5,   // одинаковая вероятность M и D
        age_bias_coefficient:    0.0,   // возраст не влияет
        ptm_exhaustion_scale:    0.001, // PTM-асимметрия → истощение матери
        de_novo_centriole_division:   4,    // 16-клеточная стадия (Морула)
        meiotic_elimination_enabled: true,
    };
    sim.register_module(Box::new(HumanDevelopmentModule::with_params(dev_params)))?;

    // 4. Модуль миелоидного сдвига (регистрируется ПОСЛЕ human_development_module)
    sim.register_module(Box::new(MyeloidShiftModule::new()))?;

    // Создаём ниши
    initialize_niches(&mut sim, 5)?;

    println!("{:<6} {:<14} {:>7} {:>7} {:>7} {:>6} {:>4} {:>6} {:>4} {:<14} {:>6} {:>6} {:<14}",
        "Year", "Stage", "Damage", "Spindle", "Cilia",
        "M-ind", "ΔM", "D-ind", "ΔD", "Potency", "mBias", "Tel", "Phenotype");
    println!("{}", "-".repeat(118));

    sim.initialize()?;

    // Начальные значения индукторов для расчёта дельты
    let mut prev_m: i32 = 10;
    let mut prev_d: i32 = 8;

    for year in 0usize..100 {
        for _ in 0..365 {
            sim.step()?;
        }
        if year % 10 == 0 || year == 99 {
            let (cur_m, cur_d) = print_year_status(year, &sim, prev_m, prev_d);
            prev_m = cur_m;
            prev_d = cur_d;
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

/// Вывести строку таблицы за один год.
/// Возвращает текущие значения M и D индукторов для расчёта дельты на следующем шаге.
fn print_year_status(
    year: usize,
    sim: &SimulationManager,
    prev_m: i32,
    prev_d: i32,
) -> (i32, i32) {
    let world = sim.world();

    let myeloid_data: Vec<_> = {
        let mut q = world.query::<&MyeloidShiftComponent>();
        q.iter().map(|(e, m)| (e, m.clone())).collect()
    };
    let infl_data: Vec<_> = {
        let mut q = world.query::<&InflammagingState>();
        q.iter().map(|(e, i)| (e, i.clone())).collect()
    };
    let telomere_data: Vec<_> = {
        let mut q = world.query::<&TelomereState>();
        q.iter().map(|(e, t)| (e, t.clone())).collect()
    };

    let mut dev_query = world.query::<&HumanDevelopmentComponent>();
    if let Some((entity, dev)) = dev_query.iter().find(|(_, d)| d.is_alive) {
        let stage_str = stage_name(dev.stage);
        let damage    = dev.damage_score();
        let spindle   = dev.centriolar_damage.spindle_fidelity;
        let cilia     = dev.centriolar_damage.ciliary_function;

        let m_ind = dev.inducers.mother_set.remaining as i32;
        let d_ind = dev.inducers.daughter_set.remaining as i32;
        let delta_m = m_ind - prev_m;
        let delta_d = d_ind - prev_d;
        let potency_str = format!("{:?}", dev.inducers.potency_level());

        let myeloid_bias = myeloid_data.iter()
            .find(|(e, _)| *e == entity)
            .map(|(_, m)| m.myeloid_bias)
            .unwrap_or(0.0);

        let phenotype_str = myeloid_data.iter()
            .find(|(e, _)| *e == entity)
            .map(|(_, m)| format!("{:?}", m.phenotype))
            .unwrap_or_else(|| "N/A".to_string());

        let _ros_boost = infl_data.iter()
            .find(|(e, _)| *e == entity)
            .map(|(_, i)| i.ros_boost)
            .unwrap_or(0.0);

        let tel_len = telomere_data.iter()
            .find(|(e, _)| *e == entity)
            .map(|(_, t)| t.mean_length)
            .unwrap_or(1.0);

        let dm_str = if delta_m == 0 { "=".to_string() } else { format!("{:+}", delta_m) };
        let dd_str = if delta_d == 0 { "=".to_string() } else { format!("{:+}", delta_d) };

        println!(
            "{:<6} {:<14} {:>7.3} {:>7.3} {:>7.3} {:>6} {:>4} {:>6} {:>4} {:<14} {:>6.3} {:>6.3} {:<14}",
            year, stage_str, damage, spindle, cilia,
            m_ind, dm_str, d_ind, dd_str,
            potency_str, myeloid_bias, tel_len, phenotype_str,
        );

        (m_ind, d_ind)
    } else {
        println!("{:<6} [all niches exhausted]", year);
        (prev_m, prev_d)
    }
}

fn print_final_status(sim: &SimulationManager) {
    let world = sim.world();

    println!("\n{:<12} {:<14} {:>8} {:>8} {:>8} {:>7} {:>6} {:>6} {:<14} {:<14}",
        "Tissue", "Status",
        "Age(yr)", "Damage", "Spindle",
        "mBias", "M-ind", "D-ind", "Potency", "Phenotype");
    println!("{}", "-".repeat(105));

    let myeloid_map: std::collections::HashMap<_, _> = {
        let mut q = world.query::<&MyeloidShiftComponent>();
        q.iter().map(|(e, m)| (e, m.clone())).collect()
    };

    let mut alive = 0u32;
    let mut dead  = 0u32;

    let mut dev_query = world.query::<&HumanDevelopmentComponent>();
    for (entity, dev) in dev_query.iter() {
        let tissue   = format!("{:?}", dev.tissue_type);
        let status   = if dev.is_alive { "alive" } else { "dead" };
        let myeloid  = myeloid_map.get(&entity);
        let potency  = format!("{:?}", dev.inducers.potency_level());
        let phenotype = myeloid
            .map(|m| format!("{:?}", m.phenotype))
            .unwrap_or_else(|| "N/A".to_string());

        println!("{:<12} {:<14} {:>8.1} {:>8.3} {:>8.3} {:>7.3} {:>6} {:>6} {:<14} {:<14}",
            tissue, status,
            dev.age_years(),
            dev.damage_score(),
            dev.centriolar_damage.spindle_fidelity,
            myeloid.map_or(0.0, |m| m.myeloid_bias),
            dev.inducers.mother_set.remaining,
            dev.inducers.daughter_set.remaining,
            potency,
            phenotype);

        if dev.is_alive { alive += 1; } else { dead += 1; }
    }

    println!("\nAlive niches: {}   Dead niches: {}", alive, dead);

    // Детали первой живой ниши
    let mut dev_query2 = world.query::<&HumanDevelopmentComponent>();
    if let Some((entity, dev)) = dev_query2.iter().find(|(_, d)| d.is_alive) {
        // Индукторы
        println!("\n=== Inductor system (niche {:?}) ===", dev.tissue_type);
        println!("  Mother set  (M): {:>3} / {:>3} inherited  (fraction: {:.3})",
            dev.inducers.mother_set.remaining,
            dev.inducers.mother_set.inherited_count,
            if dev.inducers.mother_set.inherited_count > 0 {
                dev.inducers.mother_set.remaining as f32
                    / dev.inducers.mother_set.inherited_count as f32
            } else { 0.0 });
        println!("  Daughter set (D): {:>3} / {:>3} inherited  (fraction: {:.3})",
            dev.inducers.daughter_set.remaining,
            dev.inducers.daughter_set.inherited_count,
            if dev.inducers.daughter_set.inherited_count > 0 {
                dev.inducers.daughter_set.remaining as f32
                    / dev.inducers.daughter_set.inherited_count as f32
            } else { 0.0 });
        println!("  Division count:   {}", dev.inducers.division_count);
        println!("  Potency level:    {:?}", dev.inducers.potency_level());

        // Миелоидный сдвиг
        if let Some(m) = myeloid_map.get(&entity) {
            println!("\n=== Myeloid shift (niche {:?}) ===", dev.tissue_type);
            println!("  Myeloid bias:       {:.3}  ({:?})", m.myeloid_bias, m.phenotype);
            println!("  Lymphoid deficit:   {:.3}", m.lymphoid_deficit);
            println!("  Inflammaging index: {:.3}", m.inflammaging_index);
            println!("  Immune senescence:  {:.3}", m.immune_senescence);
        }

        // Центриолярные повреждения
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

        if !dev.active_phenotypes.is_empty() {
            println!("\n  Active aging phenotypes ({}):", dev.active_phenotypes.len());
            for ph in &dev.active_phenotypes {
                println!("    - {:?}", ph);
            }
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
