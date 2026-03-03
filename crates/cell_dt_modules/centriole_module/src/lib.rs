use cell_dt_core::{
    SimulationModule, SimulationResult,
    hecs::World,
};
use cell_dt_core::components::{CentriolePair, CellCycleStateExtended, Phase};
use serde_json::{json, Value};
use log::info;

/// Parameters governing per-step PTM accumulation on the `CentriolePair`.
///
/// These rates are deliberately small (per simulation step ≈ 1 day) so that
/// visible damage accumulates over years, not hours.
///
/// Biological rationale:
/// - Mother centriole is older → higher basal acetylation / oxidation.
/// - M-phase boost: tubulin is maximally exposed to PTM enzymes during mitosis.
/// - Daughter bias: daughter accumulates PTMs at `daughter_ptm_factor` of mother's rate.
#[derive(Debug, Clone)]
pub struct CentrioleParams {
    /// Per-step tubulin hyper-acetylation rate for mother [0..1/step]
    pub acetylation_rate: f32,
    /// Per-step oxidation rate for mother [0..1/step]
    pub oxidation_rate: f32,
    /// Per-step methylation rate for mother [0..1/step]
    pub methylation_rate: f32,
    /// Per-step phosphorylation rate for mother [0..1/step]
    pub phosphorylation_rate: f32,
    /// Daughter accumulates PTMs at this fraction of mother's rate [0..1]
    pub daughter_ptm_factor: f32,
    /// Extra multiplier applied during M-phase (spindle stress) [1.0..]
    pub m_phase_boost: f32,
    /// Whether cells run in parallel (legacy flag, kept for API compat)
    pub parallel_cells: bool,
}

impl Default for CentrioleParams {
    fn default() -> Self {
        Self {
            // ~1-2% per day baseline (very small; significant only over years)
            acetylation_rate:    0.0002,
            oxidation_rate:      0.0001,
            methylation_rate:    0.00005,
            phosphorylation_rate: 0.0001,
            daughter_ptm_factor: 0.4,
            m_phase_boost:       3.0,
            parallel_cells:      true,
        }
    }
}

pub struct CentrioleModule {
    params: CentrioleParams,
    step_count: u64,
}

impl CentrioleModule {
    pub fn new() -> Self {
        Self {
            params: CentrioleParams::default(),
            step_count: 0,
        }
    }

    pub fn with_parallel(parallel_cells: bool) -> Self {
        Self {
            params: CentrioleParams { parallel_cells, ..Default::default() },
            step_count: 0,
        }
    }

    pub fn with_params(params: CentrioleParams) -> Self {
        Self { params, step_count: 0 }
    }

    /// Apply PTM accumulation to a single `CentriolePair`.
    ///
    /// - Mother always accumulates (it's older).
    /// - Daughter accumulates at `daughter_ptm_factor` of mother's rate.
    /// - M-phase applies an extra `m_phase_boost` multiplier.
    /// - All levels are clamped to [0, 1].
    fn accumulate_ptm(
        &self,
        pair: &mut CentriolePair,
        in_m_phase: bool,
        dt: f32,
    ) {
        let boost = if in_m_phase { self.params.m_phase_boost } else { 1.0 };
        let p = &self.params;

        // --- Mother ---
        let m = &mut pair.mother.ptm_signature;
        m.acetylation_level    = (m.acetylation_level    + p.acetylation_rate    * boost * dt).min(1.0);
        m.oxidation_level      = (m.oxidation_level      + p.oxidation_rate      * boost * dt).min(1.0);
        m.methylation_level    = (m.methylation_level    + p.methylation_rate    * boost * dt).min(1.0);
        m.phosphorylation_level= (m.phosphorylation_level+ p.phosphorylation_rate* boost * dt).min(1.0);

        // --- Daughter (younger → slower PTM accumulation) ---
        let f = p.daughter_ptm_factor;
        let d = &mut pair.daughter.ptm_signature;
        d.acetylation_level    = (d.acetylation_level    + p.acetylation_rate    * f * boost * dt).min(1.0);
        d.oxidation_level      = (d.oxidation_level      + p.oxidation_rate      * f * boost * dt).min(1.0);
        d.methylation_level    = (d.methylation_level    + p.methylation_rate    * f * boost * dt).min(1.0);
        d.phosphorylation_level= (d.phosphorylation_level+ p.phosphorylation_rate* f * boost * dt).min(1.0);
    }
}

impl SimulationModule for CentrioleModule {
    fn name(&self) -> &str {
        "centriole_module"
    }

    /// Per-step PTM accumulation on every `CentriolePair` in the ECS world.
    ///
    /// Reads `CellCycleStateExtended` (optional) to detect M-phase.
    /// Does NOT touch `CentriolarDamageState` — that belongs to
    /// `HumanDevelopmentModule` to avoid double-counting.
    fn step(&mut self, world: &mut World, dt: f64) -> SimulationResult<()> {
        self.step_count += 1;
        let dt_f32 = dt as f32;

        let mut query = world.query::<(&mut CentriolePair, Option<&CellCycleStateExtended>)>();
        for (_entity, (pair, cycle)) in query.iter() {
            let in_m_phase = cycle
                .map(|c| c.phase == Phase::M)
                .unwrap_or(false);
            self.accumulate_ptm(pair, in_m_phase, dt_f32);
        }

        Ok(())
    }

    fn get_params(&self) -> Value {
        json!({
            "acetylation_rate":     self.params.acetylation_rate,
            "oxidation_rate":       self.params.oxidation_rate,
            "methylation_rate":     self.params.methylation_rate,
            "phosphorylation_rate": self.params.phosphorylation_rate,
            "daughter_ptm_factor":  self.params.daughter_ptm_factor,
            "m_phase_boost":        self.params.m_phase_boost,
            "parallel_cells":       self.params.parallel_cells,
        })
    }

    fn set_params(&mut self, params: &Value) -> SimulationResult<()> {
        if let Some(v) = params.get("acetylation_rate").and_then(|v| v.as_f64()) {
            self.params.acetylation_rate = v as f32;
        }
        if let Some(v) = params.get("oxidation_rate").and_then(|v| v.as_f64()) {
            self.params.oxidation_rate = v as f32;
        }
        if let Some(v) = params.get("methylation_rate").and_then(|v| v.as_f64()) {
            self.params.methylation_rate = v as f32;
        }
        if let Some(v) = params.get("phosphorylation_rate").and_then(|v| v.as_f64()) {
            self.params.phosphorylation_rate = v as f32;
        }
        if let Some(v) = params.get("daughter_ptm_factor").and_then(|v| v.as_f64()) {
            self.params.daughter_ptm_factor = v as f32;
        }
        if let Some(v) = params.get("m_phase_boost").and_then(|v| v.as_f64()) {
            self.params.m_phase_boost = v as f32;
        }
        if let Some(v) = params.get("parallel_cells").and_then(|v| v.as_bool()) {
            self.params.parallel_cells = v;
        }
        Ok(())
    }

    fn initialize(&mut self, _world: &mut World) -> SimulationResult<()> {
        info!("Initializing centriole module (PTM accumulation enabled)");
        Ok(())
    }
}

impl Default for CentrioleModule {
    fn default() -> Self {
        Self::new()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use cell_dt_core::hecs::World;
    use cell_dt_core::components::{CentriolePair, CellCycleStateExtended, Phase};

    fn make_world_with_pair() -> (World, cell_dt_core::hecs::Entity) {
        let mut world = World::new();
        let entity = world.spawn((CentriolePair::default(),));
        (world, entity)
    }

    #[test]
    fn test_ptm_starts_at_zero() {
        let (world, entity) = make_world_with_pair();
        let pair = world.get::<&CentriolePair>(entity).unwrap();
        assert_eq!(pair.mother.ptm_signature.acetylation_level, 0.0);
        assert_eq!(pair.daughter.ptm_signature.oxidation_level, 0.0);
    }

    #[test]
    fn test_ptm_increases_after_steps() {
        let (mut world, entity) = make_world_with_pair();
        let mut module = CentrioleModule::new();

        // Run 1000 steps (dt=1.0 each)
        for _ in 0..1000 {
            module.step(&mut world, 1.0).unwrap();
        }

        let pair = world.get::<&CentriolePair>(entity).unwrap();
        assert!(pair.mother.ptm_signature.acetylation_level > 0.0,
            "mother acetylation should increase");
        assert!(pair.daughter.ptm_signature.acetylation_level > 0.0,
            "daughter acetylation should increase");
    }

    #[test]
    fn test_mother_accumulates_faster_than_daughter() {
        let (mut world, entity) = make_world_with_pair();
        let mut module = CentrioleModule::new();

        for _ in 0..1000 {
            module.step(&mut world, 1.0).unwrap();
        }

        let pair = world.get::<&CentriolePair>(entity).unwrap();
        let mother_acet = pair.mother.ptm_signature.acetylation_level;
        let daughter_acet = pair.daughter.ptm_signature.acetylation_level;
        assert!(mother_acet > daughter_acet,
            "mother ({}) should have more acetylation than daughter ({})",
            mother_acet, daughter_acet);
    }

    #[test]
    fn test_m_phase_boosts_ptm() {
        // Two worlds: one stays in G1, one is in M phase
        let mut world_g1 = World::new();
        let ent_g1 = world_g1.spawn((CentriolePair::default(), CellCycleStateExtended::new()));

        let mut world_m = World::new();
        let ent_m = world_m.spawn((CentriolePair::default(), {
            let mut c = CellCycleStateExtended::new();
            c.phase = Phase::M;
            c
        }));

        let mut module = CentrioleModule::new();
        for _ in 0..100 {
            module.step(&mut world_g1, 1.0).unwrap();
            module.step(&mut world_m,  1.0).unwrap();
        }

        let acet_g1 = world_g1.get::<&CentriolePair>(ent_g1).unwrap()
            .mother.ptm_signature.acetylation_level;
        let acet_m  = world_m .get::<&CentriolePair>(ent_m ).unwrap()
            .mother.ptm_signature.acetylation_level;

        assert!(acet_m > acet_g1,
            "M-phase ({}) should accumulate more PTM than G1 ({})", acet_m, acet_g1);
    }

    #[test]
    fn test_ptm_clamped_at_one() {
        let (mut world, entity) = make_world_with_pair();
        let mut params = CentrioleParams::default();
        params.acetylation_rate = 10.0; // extreme rate
        let mut module = CentrioleModule::with_params(params);

        module.step(&mut world, 1.0).unwrap();

        let pair = world.get::<&CentriolePair>(entity).unwrap();
        assert_eq!(pair.mother.ptm_signature.acetylation_level, 1.0,
            "PTM level must be clamped at 1.0");
    }

    #[test]
    fn test_daughter_factor_zero_no_daughter_ptm() {
        let (mut world, entity) = make_world_with_pair();
        let mut params = CentrioleParams::default();
        params.daughter_ptm_factor = 0.0;
        let mut module = CentrioleModule::with_params(params);

        for _ in 0..1000 {
            module.step(&mut world, 1.0).unwrap();
        }

        let pair = world.get::<&CentriolePair>(entity).unwrap();
        assert_eq!(pair.daughter.ptm_signature.acetylation_level, 0.0,
            "With factor=0.0 daughter should have no PTM");
        assert!(pair.mother.ptm_signature.acetylation_level > 0.0,
            "Mother should still accumulate");
    }
}
