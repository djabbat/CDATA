//! Модуль клеточного цикла с форсированными чекпоинтами
//!
//! ## Чекпоинты
//!
//! Читает standalone `CentriolarDamageState` (синхронизируется из
//! `human_development_module` каждый шаг). При `checkpoint_strictness > 0`:
//!
//! | Переход | Чекпоинт | Условие ареста |
//! |---------|----------|----------------|
//! | G1 → S  | `G1SRestriction`  | `total_damage_score() > strictness` |
//! | G2 → M  | `SpindleAssembly` | `spindle_fidelity < (1 - strictness)` |
//!
//! При `checkpoint_strictness = 0.0` (default) поведение как раньше — нет арестов.
//! При `checkpoint_strictness = 0.3` — умеренная строгость (рекомендуется с CDATA).
//!
//! ## Выход из ареста
//!
//! Клетка выходит автоматически когда повреждения снижаются ниже порога.
//! Лаг на один шаг (обратная связь через InflammagingState → DamageParams).

use cell_dt_core::{
    SimulationModule, SimulationResult,
    components::{
        CentriolePair, CellCycleState, CellCycleStateExtended,
        CentriolarDamageState, Phase, Checkpoint,
    },
    hecs::World,
};
use serde_json::{json, Value};
use log::{info, debug};

/// Параметры модуля клеточного цикла
#[derive(Debug, Clone)]
pub struct CellCycleParams {
    pub base_cycle_time: f32,
    pub growth_factor_sensitivity: f32,
    pub stress_sensitivity: f32,
    /// Порог строгости чекпоинтов [0..1].
    /// 0.0 — никогда не арестовывать (обратная совместимость).
    /// 0.3 — умеренная строгость (рекомендуется при CDATA).
    pub checkpoint_strictness: f32,
    pub enable_apoptosis: bool,
    pub nutrient_availability: f32,
    pub growth_factor_level: f32,
    pub random_variation: f32,
}

impl Default for CellCycleParams {
    fn default() -> Self {
        Self {
            base_cycle_time: 24.0,
            growth_factor_sensitivity: 0.3,
            stress_sensitivity: 0.2,
            // 0.0 → нет арестов (совместимо с предыдущим поведением)
            checkpoint_strictness: 0.0,
            enable_apoptosis: true,
            nutrient_availability: 0.9,
            growth_factor_level: 0.8,
            random_variation: 0.2,
        }
    }
}

/// Модуль клеточного цикла
pub struct CellCycleModule {
    params: CellCycleParams,
    step_count: u64,
    cells_arrested: usize,
    cells_divided: usize,
}

impl CellCycleModule {
    pub fn new() -> Self {
        Self {
            params: CellCycleParams::default(),
            step_count: 0,
            cells_arrested: 0,
            cells_divided: 0,
        }
    }

    pub fn with_params(params: CellCycleParams) -> Self {
        Self {
            params,
            step_count: 0,
            cells_arrested: 0,
            cells_divided: 0,
        }
    }

    /// Обновить клеточный цикл за один шаг с проверкой чекпоинтов.
    ///
    /// Возвращает `true` если клетка только что завершила митоз (M → G1).
    fn update_cell_cycle(
        &self,
        cell_cycle: &mut CellCycleStateExtended,
        _centriole: Option<&CentriolePair>,
        damage: Option<&CentriolarDamageState>,
        dt: f32,
    ) -> bool {
        let strictness = self.params.checkpoint_strictness;

        cell_cycle.time_in_current_phase += dt;
        cell_cycle.total_time += dt;

        // --- Проверка выхода из ареста ---
        if let Some(checkpoint) = cell_cycle.current_checkpoint {
            let can_exit = damage.map_or(true, |d| match checkpoint {
                Checkpoint::G1SRestriction | Checkpoint::DNARepair =>
                    d.total_damage_score() <= strictness,
                Checkpoint::SpindleAssembly | Checkpoint::G2MCheckpoint =>
                    d.spindle_fidelity >= (1.0 - strictness),
            });
            if can_exit {
                // Арест снят — прогресс ≥ 1.0, поэтому переход произойдёт ниже
                cell_cycle.current_checkpoint = None;
            } else {
                return false; // ещё в аресте
            }
        }

        // --- Нормальное продвижение прогресса ---
        let phase_duration = match cell_cycle.phase {
            Phase::G1 => 10.0,
            Phase::S  =>  8.0,
            Phase::G2 =>  4.0,
            Phase::M  =>  1.0,
        };

        // Прогресс не превышает 1.0 — держим клетку у порога до выхода из ареста
        cell_cycle.progress = (cell_cycle.progress + dt / phase_duration).min(1.0);

        if cell_cycle.progress < 1.0 {
            return false;
        }

        // --- Переход фазы с проверкой чекпоинтов ---
        match cell_cycle.phase {
            Phase::G1 => {
                // G1/S Restriction Point: повреждения слишком высоки?
                if strictness > 0.0 {
                    if let Some(dmg) = damage {
                        if dmg.total_damage_score() > strictness {
                            cell_cycle.current_checkpoint = Some(Checkpoint::G1SRestriction);
                            debug!("G1/S checkpoint: damage={:.3} > strictness={:.3}",
                                dmg.total_damage_score(), strictness);
                            return false;
                        }
                    }
                }
                cell_cycle.progress = 0.0;
                cell_cycle.time_in_current_phase = 0.0;
                cell_cycle.phase = Phase::S;
            }
            Phase::S => {
                cell_cycle.progress = 0.0;
                cell_cycle.time_in_current_phase = 0.0;
                cell_cycle.phase = Phase::G2;
            }
            Phase::G2 => {
                // Spindle Assembly Checkpoint: веретено нарушено?
                if strictness > 0.0 {
                    if let Some(dmg) = damage {
                        if dmg.spindle_fidelity < (1.0 - strictness) {
                            cell_cycle.current_checkpoint = Some(Checkpoint::SpindleAssembly);
                            debug!("G2/M checkpoint: spindle_fidelity={:.3} < {:.3}",
                                dmg.spindle_fidelity, 1.0 - strictness);
                            return false;
                        }
                    }
                }
                cell_cycle.progress = 0.0;
                cell_cycle.time_in_current_phase = 0.0;
                cell_cycle.phase = Phase::M;
            }
            Phase::M => {
                cell_cycle.progress = 0.0;
                cell_cycle.time_in_current_phase = 0.0;
                cell_cycle.phase = Phase::G1;
                cell_cycle.cycle_count += 1;
                return true; // клетка поделилась
            }
        }
        false
    }
}

impl SimulationModule for CellCycleModule {
    fn name(&self) -> &str { "cell_cycle_module" }

    fn step(&mut self, world: &mut World, dt: f64) -> SimulationResult<()> {
        self.step_count += 1;
        let dt_f32 = dt as f32;

        debug!("Cell cycle module step {}", self.step_count);

        self.cells_arrested = 0;
        self.cells_divided  = 0;

        // Читаем CentriolarDamageState опционально — работает и без CDATA-модулей.
        let mut query = world.query::<(
            &mut CellCycleStateExtended,
            Option<&CentriolePair>,
            Option<&CentriolarDamageState>,
        )>();

        for (_, (cell_cycle, centriole_opt, damage_opt)) in query.iter() {
            // Синхронизируем GrowthFactors с актуальным состоянием повреждений
            if let Some(dmg) = damage_opt {
                cell_cycle.growth_factors.dna_damage      = dmg.total_damage_score();
                cell_cycle.growth_factors.oxidative_stress = dmg.ros_level;
                cell_cycle.growth_factors.stress_level     =
                    (dmg.total_damage_score() * self.params.stress_sensitivity).min(1.0);
            }

            let divided = self.update_cell_cycle(cell_cycle, centriole_opt, damage_opt, dt_f32);

            if cell_cycle.current_checkpoint.is_some() {
                self.cells_arrested += 1;
            }
            if divided {
                self.cells_divided += 1;
            }
        }

        Ok(())
    }

    fn get_params(&self) -> Value {
        json!({
            "base_cycle_time":          self.params.base_cycle_time,
            "growth_factor_sensitivity":self.params.growth_factor_sensitivity,
            "stress_sensitivity":       self.params.stress_sensitivity,
            "checkpoint_strictness":    self.params.checkpoint_strictness,
            "enable_apoptosis":         self.params.enable_apoptosis,
            "nutrient_availability":    self.params.nutrient_availability,
            "growth_factor_level":      self.params.growth_factor_level,
            "random_variation":         self.params.random_variation,
            "step_count":               self.step_count,
            "cells_arrested":           self.cells_arrested,
            "cells_divided":            self.cells_divided,
        })
    }

    fn set_params(&mut self, params: &Value) -> SimulationResult<()> {
        macro_rules! set_f32 {
            ($key:literal, $field:expr) => {
                if let Some(v) = params.get($key).and_then(|v| v.as_f64()) {
                    $field = v as f32;
                }
            };
        }
        macro_rules! set_bool {
            ($key:literal, $field:expr) => {
                if let Some(v) = params.get($key).and_then(|v| v.as_bool()) {
                    $field = v;
                }
            };
        }
        set_f32!("base_cycle_time",           self.params.base_cycle_time);
        set_f32!("growth_factor_sensitivity", self.params.growth_factor_sensitivity);
        set_f32!("stress_sensitivity",        self.params.stress_sensitivity);
        set_f32!("checkpoint_strictness",     self.params.checkpoint_strictness);
        set_f32!("nutrient_availability",     self.params.nutrient_availability);
        set_f32!("growth_factor_level",       self.params.growth_factor_level);
        set_f32!("random_variation",          self.params.random_variation);
        set_bool!("enable_apoptosis",         self.params.enable_apoptosis);
        Ok(())
    }

    fn initialize(&mut self, world: &mut World) -> SimulationResult<()> {
        info!("Initializing cell cycle module (checkpoint_strictness={})",
            self.params.checkpoint_strictness);

        let states: Vec<_> = world.query::<&CellCycleState>()
            .iter()
            .map(|(entity, state)| (entity, state.clone()))
            .collect();

        for (entity, old_state) in states {
            let mut new_state = CellCycleStateExtended::new();
            new_state.phase    = old_state.phase;
            new_state.progress = old_state.progress;
            let _ = world.remove_one::<CellCycleState>(entity);
            let _ = world.insert_one(entity, new_state);
        }

        info!("Cell cycle module initialized");
        Ok(())
    }
}

impl Default for CellCycleModule {
    fn default() -> Self { Self::new() }
}

// ---------------------------------------------------------------------------
// Тесты
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use cell_dt_core::components::{CellCycleStateExtended, CentriolarDamageState, Phase, Checkpoint};
    use cell_dt_core::hecs::World;

    fn module(strictness: f32) -> CellCycleModule {
        CellCycleModule::with_params(CellCycleParams {
            checkpoint_strictness: strictness,
            ..Default::default()
        })
    }

    /// Создаёт мир с одной сущностью у порога перехода G1→S
    fn world_at_g1_boundary(damage: CentriolarDamageState) -> World {
        let mut world = World::new();
        let mut cycle = CellCycleStateExtended::new();
        cycle.phase    = Phase::G1;
        cycle.progress = 0.99; // почти у порога
        world.spawn((cycle, damage));
        world
    }

    fn world_at_g2_boundary(damage: CentriolarDamageState) -> World {
        let mut world = World::new();
        let mut cycle = CellCycleStateExtended::new();
        cycle.phase    = Phase::G2;
        cycle.progress = 0.99;
        world.spawn((cycle, damage));
        world
    }

    // --- Вспомогательные конструкторы повреждений ---

    fn high_mol_damage() -> CentriolarDamageState {
        // total_damage_score = (0.8 + appendage_loss=0) / 2 = 0.4 > 0.3
        let mut d = CentriolarDamageState::pristine();
        d.protein_carbonylation       = 0.8;
        d.tubulin_hyperacetylation    = 0.8;
        d.protein_aggregates          = 0.8;
        d.phosphorylation_dysregulation = 0.8;
        d.update_functional_metrics();
        d
    }

    fn broken_spindle() -> CentriolarDamageState {
        // spindle_fidelity = (1 - structural) * (1 - phospho*0.3)
        // structural = (0.8 + 0.8) / 2 = 0.8 → spindle = 0.2 < (1-0.3) = 0.7
        let mut d = CentriolarDamageState::pristine();
        d.protein_carbonylation = 0.8;
        d.protein_aggregates    = 0.8;
        d.update_functional_metrics();
        d
    }

    // --- Тесты ---

    #[test]
    fn test_pristine_cell_advances_g1_to_s() {
        let mut m = module(0.3);
        let mut world = world_at_g1_boundary(CentriolarDamageState::pristine());

        m.step(&mut world, 1.0).unwrap();

        let mut q = world.query::<&CellCycleStateExtended>();
        let (_, c) = q.iter().next().unwrap();
        assert_eq!(c.phase, Phase::S, "pristine cell should advance G1→S");
        assert!(c.current_checkpoint.is_none());
    }

    #[test]
    fn test_damaged_cell_arrested_at_g1s() {
        let mut m = module(0.3);
        let mut world = world_at_g1_boundary(high_mol_damage());

        m.step(&mut world, 1.0).unwrap();

        let mut q = world.query::<&CellCycleStateExtended>();
        let (_, c) = q.iter().next().unwrap();
        assert_eq!(c.phase, Phase::G1, "damaged cell must stay in G1");
        assert_eq!(c.current_checkpoint, Some(Checkpoint::G1SRestriction));
        assert_eq!(m.cells_arrested, 1);
    }

    #[test]
    fn test_broken_spindle_arrested_at_g2m() {
        let mut m = module(0.3);
        let mut world = world_at_g2_boundary(broken_spindle());

        m.step(&mut world, 1.0).unwrap();

        let mut q = world.query::<&CellCycleStateExtended>();
        let (_, c) = q.iter().next().unwrap();
        assert_eq!(c.phase, Phase::G2, "broken spindle must stay in G2");
        assert_eq!(c.current_checkpoint, Some(Checkpoint::SpindleAssembly));
        assert_eq!(m.cells_arrested, 1);
    }

    #[test]
    fn test_zero_strictness_never_arrests() {
        // strictness=0.0 → ветка `if strictness > 0.0` не выполняется → нет ареста
        let mut m = module(0.0);
        let mut world = world_at_g1_boundary(high_mol_damage());

        m.step(&mut world, 1.0).unwrap();

        let mut q = world.query::<&CellCycleStateExtended>();
        let (_, c) = q.iter().next().unwrap();
        assert!(c.current_checkpoint.is_none(), "strictness=0 → no arrest");
        assert_eq!(c.phase, Phase::S, "should still advance to S");
        assert_eq!(m.cells_arrested, 0);
    }

    #[test]
    fn test_arrest_releases_when_damage_clears() {
        let mut m = module(0.3);
        let mut world = World::new();

        // Шаг 1: выставляем арест вручную
        let mut cycle = CellCycleStateExtended::new();
        cycle.phase    = Phase::G1;
        cycle.progress = 1.0;
        cycle.current_checkpoint = Some(Checkpoint::G1SRestriction);
        let damage = CentriolarDamageState::pristine(); // повреждений нет → выйдет из ареста
        world.spawn((cycle, damage));

        m.step(&mut world, 0.1).unwrap();

        let mut q = world.query::<&CellCycleStateExtended>();
        let (_, c) = q.iter().next().unwrap();
        // Арест снят, произошёл переход G1→S
        assert!(c.current_checkpoint.is_none(), "arrest should clear");
        assert_eq!(c.phase, Phase::S, "should advance to S after arrest clears");
    }

    #[test]
    fn test_cells_divided_counter() {
        let mut m = module(0.0);
        let mut world = World::new();

        let mut cycle = CellCycleStateExtended::new();
        cycle.phase    = Phase::M;
        cycle.progress = 0.99;
        world.spawn((cycle, CentriolarDamageState::pristine()));

        m.step(&mut world, 1.0).unwrap();

        assert_eq!(m.cells_divided, 1, "one cell should have divided");
        let mut q = world.query::<&CellCycleStateExtended>();
        let (_, c) = q.iter().next().unwrap();
        assert_eq!(c.cycle_count, 1);
        assert_eq!(c.phase, Phase::G1);
    }
}
