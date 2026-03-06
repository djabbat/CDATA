//! Модуль иерархии стволовых клеток
//!
//! Потентность читается из `CentriolarDamageState` (spindle_fidelity как прокси)
//! и синхронизируется с `StemCellHierarchyState` на каждом шаге.
//!
//! `PotencyLevel` определён в `cell_dt_core::components` и переэкспортируется
//! здесь для обратной совместимости.

use cell_dt_core::{
    SimulationModule, SimulationResult,
    components::*,
    hecs::World,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use log::{info, debug};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

// PotencyLevel определён в cell_dt_core::components (glob-импорт выше).
// Переэкспортируем для совместимости с существующими примерами.
pub use cell_dt_core::components::PotencyLevel;

/// Линии дифференцировки
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CellLineage {
    EmbryonicStem,
    HematopoieticStem,
    NeuralStem,
}

/// Состояние клетки в иерархии потентности
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StemCellHierarchyState {
    pub potency_level: PotencyLevel,
    pub potency_score: f32,
    pub lineage: Option<CellLineage>,
    pub master_regulator_levels: std::collections::HashMap<String, f32>,
    /// Число событий дедифференцировки (Oligopotent → Pluripotent) за жизнь клетки
    pub dedifferentiation_count: u32,
}

impl StemCellHierarchyState {
    pub fn new() -> Self {
        let mut regs = std::collections::HashMap::new();
        regs.insert("OCT4".to_string(),  0.9);
        regs.insert("NANOG".to_string(), 0.9);
        regs.insert("SOX2".to_string(),  0.9);
        Self {
            potency_level: PotencyLevel::Pluripotent,
            potency_score: 0.8,
            lineage: None,
            master_regulator_levels: regs,
            dedifferentiation_count: 0,
        }
    }

    /// Установить потентность и пересчитать potency_score и мастер-регуляторы.
    pub fn set_potency(&mut self, level: PotencyLevel) {
        self.potency_level = level;
        self.potency_score = match level {
            PotencyLevel::Totipotent  => 1.0,
            PotencyLevel::Pluripotent => 0.8,
            PotencyLevel::Oligopotent => 0.4,
            PotencyLevel::Unipotent   => 0.2,
            PotencyLevel::Apoptosis   => 0.0,
        };
        // Мастер-регуляторы медленно дрейфуют к текущему уровню потентности
        let target = self.potency_score;
        for val in self.master_regulator_levels.values_mut() {
            *val = (*val * 0.99 + target * 0.01).clamp(0.0, 1.0);
        }
    }
}

impl Default for StemCellHierarchyState {
    fn default() -> Self { Self::new() }
}

/// Параметры модуля иерархии
#[derive(Debug, Clone)]
pub struct StemCellHierarchyParams {
    pub initial_potency: PotencyLevel,
    pub enable_plasticity: bool,
    pub plasticity_rate: f32,
    pub differentiation_threshold: f32,
}

impl Default for StemCellHierarchyParams {
    fn default() -> Self {
        Self {
            initial_potency: PotencyLevel::Pluripotent,
            enable_plasticity: true,
            plasticity_rate: 0.01,
            differentiation_threshold: 0.7,
        }
    }
}

/// Модуль иерархии стволовых клеток
pub struct StemCellHierarchyModule {
    params: StemCellHierarchyParams,
    step_count: u64,
    rng: StdRng,
}

impl StemCellHierarchyModule {
    pub fn new() -> Self {
        Self { params: StemCellHierarchyParams::default(), step_count: 0, rng: StdRng::from_entropy() }
    }

    pub fn with_params(params: StemCellHierarchyParams) -> Self {
        Self { params, step_count: 0, rng: StdRng::from_entropy() }
    }
}

impl SimulationModule for StemCellHierarchyModule {
    fn name(&self) -> &str { "stem_cell_hierarchy_module" }

    fn set_seed(&mut self, seed: u64) {
        self.rng = StdRng::seed_from_u64(seed);
    }

    /// Синхронизирует `StemCellHierarchyState` с молекулярным состоянием центриоли.
    ///
    /// Использует `spindle_fidelity` как прокси потентности:
    /// высокая точность веретена → клетка сохраняет стволовость.
    ///
    /// При `enable_plasticity=true`: клетки на уровне `Oligopotent` могут
    /// дедифференцироваться обратно в `Pluripotent`, если веретено восстановилось
    /// (spindle_fidelity > `differentiation_threshold`).
    fn step(&mut self, world: &mut World, _dt: f64) -> SimulationResult<()> {
        self.step_count += 1;
        debug!("Stem cell hierarchy step {}", self.step_count);

        let enable_plasticity    = self.params.enable_plasticity;
        let plasticity_rate      = self.params.plasticity_rate;
        let plasticity_threshold = self.params.differentiation_threshold;

        for (_, (hierarchy, damage)) in
            world.query_mut::<(&mut StemCellHierarchyState, &CentriolarDamageState)>()
        {
            // Пластичность: Oligopotent → Pluripotent при восстановлении веретена
            if enable_plasticity
                && hierarchy.potency_level == PotencyLevel::Oligopotent
                && damage.spindle_fidelity > plasticity_threshold
                && self.rng.gen::<f32>() < plasticity_rate
            {
                hierarchy.set_potency(PotencyLevel::Pluripotent);
                hierarchy.dedifferentiation_count += 1;
                debug!(
                    "Dedifferentiation event: Oligopotent → Pluripotent \
                     (spindle={:.3}, count={})",
                    damage.spindle_fidelity, hierarchy.dedifferentiation_count
                );
                continue; // уже обновили потентность
            }

            // Стандартная синхронизация потентности из spindle_fidelity
            let new_potency = if damage.spindle_fidelity > 0.95 {
                PotencyLevel::Totipotent
            } else if damage.spindle_fidelity > 0.75 {
                PotencyLevel::Pluripotent
            } else if damage.spindle_fidelity > 0.45 {
                PotencyLevel::Oligopotent
            } else if damage.spindle_fidelity > 0.15 {
                PotencyLevel::Unipotent
            } else {
                PotencyLevel::Apoptosis
            };

            if new_potency != hierarchy.potency_level {
                hierarchy.set_potency(new_potency);
            }
        }

        Ok(())
    }

    fn get_params(&self) -> Value {
        json!({
            "initial_potency":           format!("{:?}", self.params.initial_potency),
            "enable_plasticity":         self.params.enable_plasticity,
            "plasticity_rate":           self.params.plasticity_rate,
            "differentiation_threshold": self.params.differentiation_threshold,
            "step_count":                self.step_count,
            // Порог восстановления spindle_fidelity для дедифференцировки
            // (то же поле что differentiation_threshold, но с явным именем)
            "plasticity_spindle_threshold": self.params.differentiation_threshold,
        })
    }

    fn set_params(&mut self, params: &Value) -> SimulationResult<()> {
        if let Some(v) = params.get("enable_plasticity").and_then(|v| v.as_bool()) {
            self.params.enable_plasticity = v;
        }
        if let Some(v) = params.get("plasticity_rate").and_then(|v| v.as_f64()) {
            self.params.plasticity_rate = v as f32;
        }
        if let Some(v) = params.get("differentiation_threshold").and_then(|v| v.as_f64()) {
            self.params.differentiation_threshold = v as f32;
        }
        Ok(())
    }

    fn initialize(&mut self, world: &mut World) -> SimulationResult<()> {
        info!("Initializing stem cell hierarchy module");

        let entities: Vec<_> = world
            .query::<&CellCycleStateExtended>()
            .iter()
            .map(|(e, _)| e)
            .collect();

        let count = entities.len();
        for &entity in &entities {
            if !world.contains(entity) { continue; }
            let mut state = StemCellHierarchyState::new();
            state.set_potency(self.params.initial_potency);
            world.insert_one(entity, state)?;
        }

        info!("Initialized hierarchy for {} cells (initial: {:?})",
              count, self.params.initial_potency);
        Ok(())
    }
}

impl Default for StemCellHierarchyModule {
    fn default() -> Self { Self::new() }
}

/// Фабрики для создания стволовых клеток разных типов
pub mod factories {
    use super::*;

    pub fn create_embryonic_stem_cell() -> StemCellHierarchyState {
        let mut state = StemCellHierarchyState::new();
        state.set_potency(PotencyLevel::Pluripotent);
        state.master_regulator_levels.insert("OCT4".to_string(),  1.0);
        state.master_regulator_levels.insert("NANOG".to_string(), 1.0);
        state.master_regulator_levels.insert("SOX2".to_string(),  1.0);
        state
    }

    pub fn create_hematopoietic_stem_cell() -> StemCellHierarchyState {
        let mut state = StemCellHierarchyState::new();
        state.set_potency(PotencyLevel::Oligopotent);
        state.lineage = Some(CellLineage::HematopoieticStem);
        state
    }

    pub fn create_neural_stem_cell() -> StemCellHierarchyState {
        let mut state = StemCellHierarchyState::new();
        state.set_potency(PotencyLevel::Oligopotent);
        state.lineage = Some(CellLineage::NeuralStem);
        state
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cell_dt_core::components::{CentriolarDamageState, CellCycleStateExtended};
    use cell_dt_core::{SimulationManager, SimulationConfig};

    fn make_damaged_damage_state(spindle: f32) -> CentriolarDamageState {
        let mut d = CentriolarDamageState::pristine();
        // Принудительно выставляем spindle_fidelity через производные поля
        // (spindle_fidelity вычисляется в update_functional_metrics)
        // Для теста проще выставить напрямую:
        d.spindle_fidelity = spindle;
        d
    }

    /// Dedifferentiation: Oligopotent → Pluripotent при восстановлении веретена
    #[test]
    fn test_plasticity_dedifferentiation_occurs() {
        let params = StemCellHierarchyParams {
            enable_plasticity: true,
            plasticity_rate: 1.0,         // 100% вероятность для детерминизма
            differentiation_threshold: 0.6,
            initial_potency: PotencyLevel::Oligopotent,
        };
        let mut module = StemCellHierarchyModule::with_params(params);

        let config = SimulationConfig::default();
        let mut sim = SimulationManager::new(config);

        // Спавним сущность с CellCycleStateExtended (нужен для initialize)
        let entity = sim.world_mut().spawn((
            CellCycleStateExtended::new(),
            make_damaged_damage_state(0.8), // spindle > threshold (0.6)
        ));
        module.initialize(sim.world_mut()).unwrap();

        // Ставим Oligopotent вручную
        {
            let mut q = sim.world_mut().query::<&mut StemCellHierarchyState>();
            for (_, h) in q.iter() {
                h.set_potency(PotencyLevel::Oligopotent);
                h.dedifferentiation_count = 0;
            }
        }

        // Один шаг модуля — должна произойти дедифференцировка
        module.step(sim.world_mut(), 1.0).unwrap();

        let mut q = sim.world_mut().query::<&StemCellHierarchyState>();
        let (_, h) = q.iter().find(|(e, _)| *e == entity).unwrap();
        assert_eq!(h.potency_level, PotencyLevel::Pluripotent,
            "При spindle_fidelity > threshold и plasticity_rate=1.0 должна быть дедифференцировка");
        assert_eq!(h.dedifferentiation_count, 1,
            "dedifferentiation_count должен быть 1");
    }

    /// Без пластичности Oligopotent не поднимается до Pluripotent
    #[test]
    fn test_no_plasticity_when_disabled() {
        let params = StemCellHierarchyParams {
            enable_plasticity: false,
            plasticity_rate: 1.0,
            differentiation_threshold: 0.6,
            initial_potency: PotencyLevel::Oligopotent,
        };
        let mut module = StemCellHierarchyModule::with_params(params);

        let config = SimulationConfig::default();
        let mut sim = SimulationManager::new(config);

        let entity = sim.world_mut().spawn((
            CellCycleStateExtended::new(),
            make_damaged_damage_state(0.8),
        ));
        module.initialize(sim.world_mut()).unwrap();

        // Вручную ставим Oligopotent (spindle=0.8 → initialize даст Pluripotent, сбиваем)
        {
            let mut q = sim.world_mut().query::<&mut StemCellHierarchyState>();
            for (_, h) in q.iter() {
                h.potency_level = PotencyLevel::Oligopotent;
                h.dedifferentiation_count = 0;
            }
        }

        module.step(sim.world_mut(), 1.0).unwrap();

        // При enable_plasticity=false и spindle=0.8: нормальная синхронизация
        // spindle=0.8 > 0.75 → Pluripotent (через стандартный путь), count не меняется
        let mut q = sim.world_mut().query::<&StemCellHierarchyState>();
        let (_, h) = q.iter().find(|(e, _)| *e == entity).unwrap();
        assert_eq!(h.dedifferentiation_count, 0,
            "Без enable_plasticity dedifferentiation_count не должен увеличиваться");
    }
}
