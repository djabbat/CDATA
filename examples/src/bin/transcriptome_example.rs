use cell_dt_core::{
    SimulationManager, SimulationConfig,
    components::{CentriolePair, CellCycleStateExtended},
};
use centriole_module::CentrioleModule;
use cell_cycle_module::{CellCycleModule, CellCycleParams};
use transcriptome_module::{
    TranscriptomeModule, TranscriptomeParams, 
    TranscriptionFactor, SignalingPathway, TranscriptomeState
};
use cell_dt_viz::{
    VisualizationManager,
    ScatterPlotVisualizer,
    TimeSeriesVisualizer,
};
use std::io::Write;
use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Cell DT Platform - Transcriptome Module ===\n");
    
    std::fs::create_dir_all("transcriptome_output")?;
    
    let config = SimulationConfig {
        max_steps: 2000,
        dt: 0.05,
        checkpoint_interval: 500,
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
    
    // Регистрируем модуль клеточного цикла
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
    let cell_cycle_module = CellCycleModule::with_params(cell_cycle_params);
    sim.register_module(Box::new(cell_cycle_module))?;
    
    // Регистрируем модуль транскриптома
    let transcriptome_params = TranscriptomeParams {
        mutation_rate: 0.001,
        noise_level: 0.05,
        signaling_strength: 1.0,
        enable_epigenetics: true,
        stemness_maintenance: true,
    };
    let transcriptome_module = TranscriptomeModule::with_params(transcriptome_params);
    sim.register_module(Box::new(transcriptome_module))?;
    
    // Инициализируем клетки
    initialize_cells(&mut sim, 20)?;
    
    // Настраиваем визуализацию
    let mut viz_manager = VisualizationManager::new(50);
    viz_manager.add_visualizer(Box::new(ScatterPlotVisualizer::new("transcriptome_output/scatter")));
    
    let data_history = viz_manager.data_history.clone();
    viz_manager.add_visualizer(Box::new(TimeSeriesVisualizer::new("transcriptome_output/timeseries", data_history)));
    
    println!("\n🚀 Starting simulation with transcriptome...");
    println!("   Cells will express genes and respond to signals\n");
    
    sim.initialize()?;
    
    for step in 0..max_steps {
        sim.step()?;
        
        if step % 100 == 0 {
            viz_manager.update(sim.world(), sim.current_step(), sim.current_time())?;
            print_transcriptome_stats(step, max_steps, &sim);
        }
    }
    
    println!("\n✅ Simulation completed!");
    print_final_stats(&sim);
    
    Ok(())
}

fn initialize_cells(sim: &mut SimulationManager, count: usize) -> Result<(), cell_dt_core::SimulationError> {
    println!("Initializing {} cells with transcriptomes...", count);
    
    let world = sim.world_mut();
    
    for i in 0..count {
        let _entity = world.spawn((
            CentriolePair::default(),
            CellCycleStateExtended::new(),
        ));
        
        if i % 5 == 0 {
            print!(".");
            std::io::stdout().flush()?;
        }
    }
    println!(" done!");
    
    Ok(())
}

fn print_transcriptome_stats(step: u64, max_steps: u64, sim: &SimulationManager) {
    let world = sim.world();
    let mut query = world.query::<&TranscriptomeState>();
    
    let mut total_expression = 0.0;
    let mut total_pathways = 0;
    let mut stem_cells = 0;
    let mut cell_types = HashMap::new();
    let mut total_cells = 0;
    
    for (_, transcriptome) in query.iter() {
        total_cells += 1;
        total_expression += transcriptome.total_expression;
        total_pathways += transcriptome.active_pathways;
        
        if transcriptome.is_stem_cell() {
            stem_cells += 1;
        }
        
        let cell_type = transcriptome.get_cell_type();
        *cell_types.entry(cell_type).or_insert(0) += 1;
    }
    
    let progress = step as f32 / max_steps as f32 * 100.0;
    println!("\n📊 Step {}/{} ({:.1}%)", step, max_steps, progress);
    if total_cells > 0 {
        println!("   Avg expression: {:.2}", total_expression / total_cells as f32);
        println!("   Avg active pathways: {:.2}", total_pathways as f32 / total_cells as f32);
        println!("   Stem cells: {}/{}", stem_cells, total_cells);
        println!("   Cell types: {:?}", cell_types);
    }
}

fn print_final_stats(sim: &SimulationManager) {
    let world = sim.world();
    let mut query = world.query::<&TranscriptomeState>();
    
    let mut total_cells = 0;
    let mut stem_cells = 0;
    let mut cell_types = HashMap::new();
    let mut yap_activity = 0.0;
    let mut stat3_activity = 0.0;
    let mut p53_activity = 0.0;
    let mut wnt_activity = 0.0;
    
    for (_, transcriptome) in query.iter() {
        total_cells += 1;
        
        if transcriptome.is_stem_cell() {
            stem_cells += 1;
        }
        
        let cell_type = transcriptome.get_cell_type();
        *cell_types.entry(cell_type).or_insert(0) += 1;
        
        if let Some(&yap) = transcriptome.transcription_factors.get(&TranscriptionFactor::YAP) {
            yap_activity += yap;
        }
        if let Some(&stat3) = transcriptome.transcription_factors.get(&TranscriptionFactor::STAT3) {
            stat3_activity += stat3;
        }
        if let Some(&p53) = transcriptome.transcription_factors.get(&TranscriptionFactor::P53) {
            p53_activity += p53;
        }
        if let Some(wnt) = transcriptome.pathways.get(&SignalingPathway::Wnt) {
            wnt_activity += wnt.activity;
        }
    }
    
    println!("\n=== Final Transcriptome Statistics ===");
    println!("Total cells: {}", total_cells);
    println!("Stem cells: {}", stem_cells);
    println!("Cell types: {:?}", cell_types);
    
    if total_cells > 0 {
        println!("\n=== Signaling Activity ===");
        println!("Average YAP activity: {:.3}", yap_activity / total_cells as f32);
        println!("Average STAT3 activity: {:.3}", stat3_activity / total_cells as f32);
        println!("Average p53 activity: {:.3}", p53_activity / total_cells as f32);
        println!("Average Wnt pathway: {:.3}", wnt_activity / total_cells as f32);
    }
    
    // Показываем экспрессию ключевых генов для первой клетки
    if let Some((_, transcriptome)) = query.iter().next() {
        println!("\n=== Sample Cell Gene Expression ===");
        let key_genes = vec!["CCND1", "CCNE1", "CCNA2", "CCNB1", "CETN1", "PCNT", "TP53", "NANOG"];
        for gene_name in key_genes {
            if let Some(gene) = transcriptome.genes.get(gene_name) {
                println!("   {}: {:.3}", gene_name, gene.expression_level);
            }
        }
    }
}
