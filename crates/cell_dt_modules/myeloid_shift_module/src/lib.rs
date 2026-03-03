//! Модуль миелоидного сдвига (Myeloid Shift / Myeloid Bias)
//!
//! ## Биологический контекст
//!
//! С возрастом гематопоэтические и другие стволовые клетки постепенно смещают
//! дифференцировку от лимфоидного пути к миелоидному. В контексте CDATA этот
//! сдвиг **напрямую определяется накопленными повреждениями центриоли**:
//!
//! | Компонент CDATA         | Механизм                                            | Вклад |
//! |-------------------------|-----------------------------------------------------|-------|
//! | `spindle_fidelity ↓`    | Веретено не сегрегирует Numb/aPKC → оба потомка миелоидные | 45% |
//! | `ciliary_function ↓`    | Нет реснички → Wnt/Notch/Shh↓ → PU.1 побеждает    | 30%   |
//! | `ros_level ↑`           | ROS → NF-κB → IL-6/TNF-α → миелоидная среда        | 15%   |
//! | `protein_aggregates ↑`  | Агрегаты захватывают Ikaros (IKZF1) → миелоид     | 10%   |
//!
//! ## Обратные связи на CDATA
//!
//! Миелоидные клетки производят больше ROS и секретируют SASP-факторы,
//! которые повреждают нишу и ускоряют накопление повреждений центриоли.
//! Эта петля реализована через [`InflammagingState`]:
//!
//! ```text
//! myeloid_bias ↑ → inflammaging_index ↑ → InflammagingState { ros_boost, niche_impairment }
//!                                       → human_development_module ускоряет ROS-повреждения
//! ```
//!
//! ## Порядок модулей
//!
//! Регистрировать **после** `HumanDevelopmentModule`, так как модуль читает
//! `CentriolarDamageState` (синхронизируется human_development_module в step()).

use cell_dt_core::{
    SimulationModule, SimulationResult,
    hecs::World,
    components::{CentriolarDamageState, CellCycleStateExtended, InflammagingState},
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use log::{info, debug};

// ---------------------------------------------------------------------------
// Публичные типы
// ---------------------------------------------------------------------------

/// Фенотип миелоидного сдвига — качественная оценка тяжести
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MyeloidPhenotype {
    /// myeloid_bias < 0.30 — норма
    Healthy,
    /// 0.30..0.50 — субклинический сдвиг
    MildShift,
    /// 0.50..0.70 — клинически значимый (иммуносупрессия)
    ModerateShift,
    /// > 0.70 — тяжёлое иммуностарение
    SevereShift,
}

impl MyeloidPhenotype {
    pub fn from_bias(bias: f32) -> Self {
        match bias {
            b if b >= 0.70 => Self::SevereShift,
            b if b >= 0.50 => Self::ModerateShift,
            b if b >= 0.30 => Self::MildShift,
            _              => Self::Healthy,
        }
    }
}

impl Default for MyeloidPhenotype {
    fn default() -> Self { Self::Healthy }
}

/// ECS-компонент: состояние миелоидного сдвига ниши
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MyeloidShiftComponent {
    /// Доля миелоидного выхода сверх исходного [0..1]
    pub myeloid_bias: f32,
    /// Дефицит лимфоидного выхода [0..1]
    pub lymphoid_deficit: f32,
    /// Воспалительный индекс (inflammaging) [0..1]
    pub inflammaging_index: f32,
    /// Иммунное старение (exhaustion) [0..1]
    pub immune_senescence: f32,
    /// Качественный фенотип
    pub phenotype: MyeloidPhenotype,
}

impl Default for MyeloidShiftComponent {
    fn default() -> Self {
        Self {
            myeloid_bias: 0.0,
            lymphoid_deficit: 0.0,
            inflammaging_index: 0.0,
            immune_senescence: 0.0,
            phenotype: MyeloidPhenotype::Healthy,
        }
    }
}

// ---------------------------------------------------------------------------
// Параметры модуля (панель управления)
// ---------------------------------------------------------------------------

/// Параметры модуля — веса и коэффициенты обратной связи
#[derive(Debug, Clone)]
pub struct MyeloidShiftParams {
    /// Вес spindle_fidelity в формуле myeloid_bias (default 0.45)
    pub spindle_weight: f32,
    /// Вес ciliary_function (default 0.30)
    pub cilia_weight: f32,
    /// Вес ros_level (default 0.15)
    pub ros_weight: f32,
    /// Вес protein_aggregates (default 0.10)
    pub aggregate_weight: f32,
    /// Масштабирование ros_boost → InflammagingState (default 0.15)
    pub ros_boost_scale: f32,
    /// Масштабирование niche_impairment → InflammagingState (default 0.08)
    pub niche_impair_scale: f32,
}

impl Default for MyeloidShiftParams {
    fn default() -> Self {
        Self {
            spindle_weight:    0.45,
            cilia_weight:      0.30,
            ros_weight:        0.15,
            aggregate_weight:  0.10,
            ros_boost_scale:   0.15,
            niche_impair_scale: 0.08,
        }
    }
}

// ---------------------------------------------------------------------------
// Модуль
// ---------------------------------------------------------------------------

pub struct MyeloidShiftModule {
    params: MyeloidShiftParams,
    step_count: u64,
}

impl MyeloidShiftModule {
    pub fn new() -> Self {
        Self { params: MyeloidShiftParams::default(), step_count: 0 }
    }

    pub fn with_params(params: MyeloidShiftParams) -> Self {
        Self { params, step_count: 0 }
    }

    /// Вычислить myeloid_bias из молекулярных повреждений центриоли.
    ///
    /// Формула CDATA-обоснована:
    /// * spindle_fidelity↓ → (1 − sf)^1.5 × w_spindle  (нелинейность важна: малые
    ///   повреждения не мешают, но после порога эффект резкий)
    /// * ciliary_function↓ → (1 − cf) × w_cilia
    /// * ros_level        → ros × w_ros
    /// * protein_aggregates → agg × w_agg
    fn compute_myeloid_bias(&self, damage: &CentriolarDamageState) -> f32 {
        let spindle_c  = (1.0 - damage.spindle_fidelity).powf(1.5) * self.params.spindle_weight;
        let cilia_c    = (1.0 - damage.ciliary_function)            * self.params.cilia_weight;
        let ros_c      = damage.ros_level                           * self.params.ros_weight;
        let aggr_c     = damage.protein_aggregates                  * self.params.aggregate_weight;

        (spindle_c + cilia_c + ros_c + aggr_c).clamp(0.0, 1.0)
    }
}

impl Default for MyeloidShiftModule {
    fn default() -> Self { Self::new() }
}

impl SimulationModule for MyeloidShiftModule {
    fn name(&self) -> &str { "myeloid_shift_module" }

    fn step(&mut self, world: &mut World, _dt: f64) -> SimulationResult<()> {
        self.step_count += 1;
        debug!("MyeloidShift step {}", self.step_count);

        // Читаем CentriolarDamageState (standalone, синхронизирован human_development_module)
        // и пишем MyeloidShiftComponent + InflammagingState.
        for (_, (damage, myeloid, inflammaging)) in world.query_mut::<(
            &CentriolarDamageState,
            &mut MyeloidShiftComponent,
            &mut InflammagingState,
        )>() {
            let myeloid_bias = self.compute_myeloid_bias(damage);

            // Лимфоидный дефицит — линейный относительно bias
            let lymphoid_deficit = myeloid_bias;

            // Воспалительный индекс: произведение bias × дефицит × коэф.
            let inflammaging_index = (myeloid_bias * lymphoid_deficit * 0.80).clamp(0.0, 1.0);

            // Иммунное старение: вклад SASP и дефицита Т-/B-клеток
            let immune_senescence = (inflammaging_index * 0.70
                + (1.0 - damage.ciliary_function) * 0.30).clamp(0.0, 1.0);

            // Обновляем компонент
            myeloid.myeloid_bias      = myeloid_bias;
            myeloid.lymphoid_deficit  = lymphoid_deficit;
            myeloid.inflammaging_index = inflammaging_index;
            myeloid.immune_senescence  = immune_senescence;
            myeloid.phenotype          = MyeloidPhenotype::from_bias(myeloid_bias);

            // Обратная связь → human_development_module (применится на следующем шаге)
            inflammaging.ros_boost        = (inflammaging_index * self.params.ros_boost_scale)
                .clamp(0.0, 0.5);
            inflammaging.niche_impairment = (inflammaging_index * self.params.niche_impair_scale)
                .clamp(0.0, 0.5);
            inflammaging.sasp_intensity   = inflammaging_index;
        }

        Ok(())
    }

    fn get_params(&self) -> Value {
        json!({
            "spindle_weight":     self.params.spindle_weight,
            "cilia_weight":       self.params.cilia_weight,
            "ros_weight":         self.params.ros_weight,
            "aggregate_weight":   self.params.aggregate_weight,
            "ros_boost_scale":    self.params.ros_boost_scale,
            "niche_impair_scale": self.params.niche_impair_scale,
            "step_count":         self.step_count,
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
        set_f32!("spindle_weight",     self.params.spindle_weight);
        set_f32!("cilia_weight",       self.params.cilia_weight);
        set_f32!("ros_weight",         self.params.ros_weight);
        set_f32!("aggregate_weight",   self.params.aggregate_weight);
        set_f32!("ros_boost_scale",    self.params.ros_boost_scale);
        set_f32!("niche_impair_scale", self.params.niche_impair_scale);
        Ok(())
    }

    fn initialize(&mut self, world: &mut World) -> SimulationResult<()> {
        info!("Initializing myeloid shift module");

        let entities: Vec<_> = world
            .query::<&CellCycleStateExtended>()
            .iter()
            .map(|(e, _)| e)
            .collect();

        let count = entities.len();
        for &entity in &entities {
            if !world.contains(entity) { continue; }
            // MyeloidShiftComponent — основные метрики сдвига
            world.insert_one(entity, MyeloidShiftComponent::default())?;
            // InflammagingState уже добавлена human_development_module.initialize(),
            // но если этот модуль зарегистрирован без human_development_module —
            // добавляем сами (insert_one вернёт ошибку если компонент уже есть,
            // поэтому используем try + игнорируем ошибку дублирования).
            let _ = world.insert_one(entity, InflammagingState::default());
        }

        info!("MyeloidShift: initialized {} niches", count);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Тесты
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use cell_dt_core::components::CentriolarDamageState;

    fn pristine() -> CentriolarDamageState { CentriolarDamageState::pristine() }

    fn max_damage() -> CentriolarDamageState {
        let mut d = CentriolarDamageState::pristine();
        d.spindle_fidelity = 0.0;
        d.ciliary_function = 0.0;
        d.ros_level        = 1.0;
        d.protein_aggregates = 1.0;
        d
    }

    fn module() -> MyeloidShiftModule { MyeloidShiftModule::new() }

    #[test]
    fn test_pristine_no_shift() {
        let bias = module().compute_myeloid_bias(&pristine());
        // spindle=1, cilia=1, ros=0, agg=0 → bias = 0 + 0 + 0 + 0
        assert!(bias < 0.05, "pristine damage → myeloid_bias={} (expected ≈0)", bias);
    }

    #[test]
    fn test_max_damage_full_shift() {
        let bias = module().compute_myeloid_bias(&max_damage());
        // (1)^1.5×0.45 + 1×0.30 + 1×0.15 + 1×0.10 = 1.0
        assert!((bias - 1.0).abs() < 0.01,
            "max damage → myeloid_bias={} (expected ≈1.0)", bias);
    }

    #[test]
    fn test_spindle_drives_shift() {
        let mut d = pristine();
        d.spindle_fidelity = 0.0;   // только сломано веретено
        let bias = module().compute_myeloid_bias(&d);
        // (1)^1.5×0.45 = 0.45
        assert!((bias - 0.45).abs() < 0.01,
            "spindle_fidelity=0 → bias={} (expected ≈0.45)", bias);
    }

    #[test]
    fn test_cilia_drives_shift() {
        let mut d = pristine();
        d.ciliary_function = 0.0;   // только нет реснички
        let bias = module().compute_myeloid_bias(&d);
        // 1×0.30 = 0.30
        assert!((bias - 0.30).abs() < 0.01,
            "ciliary_function=0 → bias={} (expected ≈0.30)", bias);
    }

    #[test]
    fn test_calibration_age70() {
        // При типичных повреждениях в 70 лет:
        // spindle≈0.40, cilia≈0.50, ros≈0.40, agg≈0.30
        let mut d = pristine();
        d.spindle_fidelity = 0.40;
        d.ciliary_function = 0.50;
        d.ros_level        = 0.40;
        d.protein_aggregates = 0.30;
        let bias = module().compute_myeloid_bias(&d);
        // ≈ 0.6^1.5×0.45 + 0.5×0.30 + 0.4×0.15 + 0.3×0.10
        // ≈ 0.209 + 0.15 + 0.06 + 0.03 = 0.449
        assert!(bias > 0.30 && bias < 0.65,
            "age-70 damage → bias={} (expected ModerateShift ≈0.45)", bias);
    }

    #[test]
    fn test_feedback_ros_boost() {
        let m = module();
        let inflammaging_index = 0.5_f32;
        let ros_boost = (inflammaging_index * m.params.ros_boost_scale).clamp(0.0, 0.5);
        assert!(ros_boost > 0.0, "inflammaging > 0 → ros_boost > 0");
        assert!(ros_boost <= 0.5, "ros_boost must not exceed 0.5");
    }

    #[test]
    fn test_phenotype_classification() {
        assert_eq!(MyeloidPhenotype::from_bias(0.10), MyeloidPhenotype::Healthy);
        assert_eq!(MyeloidPhenotype::from_bias(0.35), MyeloidPhenotype::MildShift);
        assert_eq!(MyeloidPhenotype::from_bias(0.60), MyeloidPhenotype::ModerateShift);
        assert_eq!(MyeloidPhenotype::from_bias(0.80), MyeloidPhenotype::SevereShift);
    }
}
