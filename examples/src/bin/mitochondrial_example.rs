//! Пример CDATA-симуляции с митохондриальным треком (Трек E).
//!
//! Демонстрирует как накопление мутаций мтДНК → суперпродукция ROS →
//! фрагментация митохондрий → ослабление кислородного щита → ускорение
//! центриолярных повреждений.
//!
//! ## Порядок модулей
//! 1. `CentrioleModule`          — PTM-накопление в центриолях
//! 2. `CellCycleModule`          — фазы клеточного цикла
//! 3. `MitochondrialModule`      — мтДНК, ROS-продукция, митофагия
//! 4. `HumanDevelopmentModule`   — CDATA + читает MitochondrialState
//! 5. `MyeloidShiftModule`       — myeloid_bias + InflammagingState
//!
//! ## Вывод каждые 10 лет
//! ```text
//! Year  Stage     Damage  Spindle  mtDNA  mitoROS  Fusion  Shield  mBias
//! ```

use cell_dt_core::{SimulationManager, SimulationConfig};
use cell_dt_core::components::{CentriolePair, CellCycleStateExtended, MitochondrialState};
use centriole_module::CentrioleModule;
use cell_cycle_module::{CellCycleModule, CellCycleParams};
use mitochondrial_module::MitochondrialModule;
use human_development_module::{
    HumanDevelopmentModule, HumanDevelopmentParams,
    HumanDevelopmentComponent,
};
use myeloid_shift_module::{MyeloidShiftModule, MyeloidShiftComponent};
use cell_dt_core::components::TissueType;
use std::io::Write;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    let config = SimulationConfig {
        max_steps: 100,
        dt: 365.25,
        checkpoint_interval: 36500,
        num_threads: Some(1),
        seed: Some(42),
        parallel_modules: false,
        cleanup_dead_interval: None,
    };
    let mut sim = SimulationManager::new(config);

    // --- Регистрация модулей ---
    sim.register_module(Box::new(CentrioleModule::new()))?;
    sim.register_module(Box::new(CellCycleModule::with_params(CellCycleParams {
        checkpoint_strictness: 0.0,
        ..Default::default()
    })))?;
    sim.register_module(Box::new(MitochondrialModule::new()))?;

    let dev_params = HumanDevelopmentParams {
        time_acceleration:       1.0,
        enable_aging:            true,
        enable_morphogenesis:    true,
        tissue_detail_level:     3,
        mother_inducer_count:    10,
        daughter_inducer_count:  8,
        base_detach_probability: 0.002,
        mother_bias:             0.5,
        age_bias_coefficient:    0.0,
        ptm_exhaustion_scale:    0.001,
        de_novo_centriole_division:   4,
        meiotic_elimination_enabled: true,
    };
    sim.register_module(Box::new(HumanDevelopmentModule::with_params(dev_params)))?;
    sim.register_module(Box::new(MyeloidShiftModule::new()))?;

    // --- Создаём ниши ---
    initialize_niches(&mut sim, 3)?;

    println!("\n=== Mitochondrial Track E — 100 years (1 step = 1 year) ===\n");
    println!("{:<6} {:<12} {:>7} {:>8} {:>7} {:>8} {:>7} {:>7} {:>7}",
             "Year", "Stage", "Damage", "Spindle", "mtDNA", "mitoROS", "Fusion", "Shield", "mBias");
    println!("{}", "-".repeat(75));

    sim.initialize()?;

    let mut last_print = 0u32;

    for step in 0..100 {
        sim.step()?;

        let year = step + 1;
        if year % 10 == 0 || year == 1 {
            last_print = year;
            print_state(&sim, year);
        }
    }
    if last_print != 100 {
        print_state(&sim, 100);
    }

    println!("\n=== Simulation complete ===");
    Ok(())
}

fn initialize_niches(sim: &mut SimulationManager, count: usize) -> Result<(), Box<dyn std::error::Error>> {
    use cell_dt_core::components::InflammagingState;
    let tissues = [TissueType::Neural, TissueType::Blood, TissueType::Muscle];
    for i in 0..count {
        let tissue = tissues[i % tissues.len()];
        sim.world_mut().spawn((
            CellCycleStateExtended::new(),
            CentriolePair::default(),
            InflammagingState::default(),
        ));
        let _ = tissue; // tissue присваивается через HumanDevelopmentModule.initialize()
    }
    Ok(())
}

fn print_state(sim: &SimulationManager, year: u32) {
    let world = sim.world();

    // Усредняем по живым нишам
    let mut count = 0;
    let mut sum_damage = 0.0_f32;
    let mut sum_spindle = 0.0_f32;
    let mut sum_mtdna = 0.0_f32;
    let mut sum_mito_ros = 0.0_f32;
    let mut sum_fusion = 0.0_f32;
    let mut sum_shield = 0.0_f32;
    let mut sum_mbias = 0.0_f32;
    let mut stage_str = "?".to_string();

    for (_, dev) in world.query::<&HumanDevelopmentComponent>().iter() {
        if !dev.is_alive { continue; }
        count += 1;
        sum_damage  += dev.centriolar_damage.total_damage_score();
        sum_spindle += dev.centriolar_damage.spindle_fidelity;
        if count == 1 { stage_str = format!("{:?}", dev.stage); }
    }
    for (_, mito) in world.query::<&MitochondrialState>().iter() {
        sum_mtdna   += mito.mtdna_mutations;
        sum_mito_ros += mito.ros_production;
        sum_fusion  += mito.fusion_index;
        sum_shield  += mito.mito_shield_contribution;
    }
    for (_, shift) in world.query::<&MyeloidShiftComponent>().iter() {
        sum_mbias += shift.myeloid_bias;
    }

    if count == 0 {
        println!("{:<6} (все ниши умерли)", year);
        return;
    }

    let n = count as f32;
    print!("{:<6} {:<12} {:>7.3} {:>8.3} {:>7.3} {:>8.3} {:>7.3} {:>7.3} {:>7.3}",
           year, stage_str,
           sum_damage / n, sum_spindle / n,
           sum_mtdna / n, sum_mito_ros / n,
           sum_fusion / n, sum_shield / n,
           sum_mbias / n);
    println!();
    std::io::stdout().flush().ok();
}
