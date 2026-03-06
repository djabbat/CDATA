//! Митохондриальный модуль — Трек E теории CDATA.
//!
//! ## Биологическая основа
//!
//! Митохондрии формируют **кислородный щит** вокруг центросомы:
//! они поглощают O₂ у периферии клетки, не давая ему проникнуть
//! к центриолям и отщепить индукторы дифференцировки.
//!
//! С возрастом накапливаются мутации мтДНК → электрон-транспортная
//! цепь разобщается → суперпродукция ROS → петля положительной
//! обратной связи → щит слабеет.
//!
//! ## Петли обратной связи
//!
//! ```text
//! mtdna_mutations ↑ → ros_production ↑ → mtdna_mutations ↑  (цикл I)
//! ros_production ↑  → fusion_index ↓  → митофагия хуже      (цикл II)
//! ros_production ↑  → ros_boost        → CDA.ros_level ↑    (выход → human_dev)
//! ```
//!
//! ## Связь с CDATA
//!
//! `mito_shield_contribution` снижает эффективную концентрацию O₂
//! у центросомы (читается `human_development_module` через `Option<&MitochondrialState>`).
//!
//! ## Калибровка
//!
//! | Возраст | ros_production | mtdna_mutations |
//! |---------|---------------|-----------------|
//! | 0       | ≈ 0.00        | 0.00            |
//! | 40      | ≈ 0.10        | 0.12            |
//! | 70      | ≈ 0.28        | 0.30            |
//! | 90      | ≈ 0.55        | 0.52            |

use cell_dt_core::{
    SimulationModule, SimulationResult,
    hecs::World,
    components::{MitochondrialState, CentriolarDamageState},
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use log::{info, trace};

// ---------------------------------------------------------------------------
// Параметры модуля
// ---------------------------------------------------------------------------

/// Параметры митохондриального модуля.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MitochondrialParams {
    /// Базовая скорость накопления мутаций мтДНК [/год].
    /// Дефолт: 0.003/год → к 80 годам ≈ 0.24 без обратных связей.
    pub base_mutation_rate: f32,
    /// Усиление скорости мутаций от ROS [0..2].
    /// Дефолт: 0.8 → при ros_production=0.5 скорость ×1.4.
    pub ros_mtdna_feedback: f32,
    /// Скорость фрагментации (потери fusion_index) от ROS [/год].
    /// Дефолт: 0.05/год при ros_production=1.0.
    pub fission_rate: f32,
    /// Базовый поток митофагии [0..1].
    /// Снижается пропорционально membrane_potential.
    pub base_mitophagy_flux: f32,
    /// Порог ros_production, при котором митофагия перегружена [0..1].
    /// При ros_production > порога — петля начинает ускоряться.
    pub mitophagy_threshold: f32,
    /// Масштаб вклада ros_production в `ros_boost` центриолярных повреждений.
    /// 0.0 → отключить обратную связь; 0.2 → умеренный вклад.
    pub ros_production_boost: f32,
    /// После 40 лет: мультипликатор скорости мутаций (антагонистическая плейотропия).
    pub midlife_mutation_multiplier: f32,
}

impl Default for MitochondrialParams {
    fn default() -> Self {
        Self {
            base_mutation_rate:          0.003,
            ros_mtdna_feedback:          0.8,
            fission_rate:                0.05,
            base_mitophagy_flux:         0.9,
            mitophagy_threshold:         0.5,
            ros_production_boost:        0.20,
            midlife_mutation_multiplier: 1.5,
        }
    }
}

// ---------------------------------------------------------------------------
// Модуль
// ---------------------------------------------------------------------------

pub struct MitochondrialModule {
    params: MitochondrialParams,
    step_count: u64,
}

impl MitochondrialModule {
    pub fn new() -> Self {
        Self { params: MitochondrialParams::default(), step_count: 0 }
    }

    pub fn with_params(params: MitochondrialParams) -> Self {
        Self { params, step_count: 0 }
    }
}

impl Default for MitochondrialModule {
    fn default() -> Self { Self::new() }
}

impl SimulationModule for MitochondrialModule {
    fn name(&self) -> &str { "mitochondrial_module" }

    fn initialize(&mut self, world: &mut World) -> SimulationResult<()> {
        let entities: Vec<_> = world
            .query::<&CentriolarDamageState>()
            .iter()
            .map(|(e, _)| e)
            .collect();
        for entity in entities {
            // Добавляем только если компонента ещё нет (idempotent)
            if world.get::<&MitochondrialState>(entity).is_err() {
                world.insert_one(entity, MitochondrialState::pristine())?;
            }
        }
        info!("MitochondrialModule initialized: {} entities", world.len());
        Ok(())
    }

    fn step(&mut self, world: &mut World, dt_days: f64) -> SimulationResult<()> {
        self.step_count += 1;
        trace!("MitochondrialModule step {} (dt={:.4} days)", self.step_count, dt_days);

        let dt_years = (dt_days / 365.25) as f32;
        let p = &self.params;

        // Lazy-init: добавляем MitochondrialState сущностям, у которых его ещё нет,
        // но есть CentriolarDamageState (т.е. HumanDevelopmentModule уже инициализировал их).
        {
            let needs_mito: Vec<_> = world
                .query::<&CentriolarDamageState>()
                .iter()
                .filter(|(e, _)| world.get::<&MitochondrialState>(*e).is_err())
                .map(|(e, _)| e)
                .collect();
            for entity in needs_mito {
                let _ = world.insert_one(entity, MitochondrialState::pristine());
            }
        }

        for (_, (mito, damage)) in
            world.query_mut::<(&mut MitochondrialState, &CentriolarDamageState)>()
        {
            // --- Возраст сущности (прокси через ros_level как прогрессию повреждений) ---
            // Используем total_damage_score как индикатор биологического возраста
            let bio_age_proxy = damage.total_damage_score();

            // --- Мультипликатор среднего возраста ---
            // Антагонистическая плейотропия: после bio_age > 0.25 (~40 лет) скорость растёт
            let age_mult = if bio_age_proxy > 0.25 { p.midlife_mutation_multiplier } else { 1.0 };

            // --- 1. Накопление мутаций мтДНК ---
            // Базовая скорость + ROS-обратная связь + возрастной мультипликатор
            let mut_rate = p.base_mutation_rate
                * age_mult
                * (1.0 + mito.ros_production * p.ros_mtdna_feedback);
            mito.mtdna_mutations = (mito.mtdna_mutations + mut_rate * dt_years).min(1.0);

            // --- 2. Продукция ROS: функция от мутаций и фрагментации ---
            // ROS растёт с мутациями (дефекты ЭТЦ) и фрагментацией (снижение fusion_index)
            let frag_contribution = (1.0 - mito.fusion_index) * 0.25;
            mito.ros_production =
                (mito.mtdna_mutations * 0.6 + frag_contribution + damage.ros_level * 0.1)
                    .clamp(0.0, 1.0);

            // --- 3. Фрагментация (потеря fusion_index) ---
            // ROS → активация DRP1 → митохондриальное деление (фрагментация)
            // Митофагия частично компенсирует: удаляет наиболее фрагментированные,
            // но при перегрузке не справляется.
            let fission = p.fission_rate * mito.ros_production;
            let mitophagy_recovery = mito.mitophagy_flux * 0.0001; // очень слабое восстановление
            mito.fusion_index = (mito.fusion_index - fission * dt_years + mitophagy_recovery * dt_years)
                .clamp(0.0, 1.0);

            // --- 4. Мембранный потенциал ΔΨm ---
            // Снижается с мутациями и ROS; частично восстанавливается митофагией
            mito.membrane_potential =
                (1.0 - mito.mtdna_mutations * 0.5 - mito.ros_production * 0.3)
                    .clamp(0.0, 1.0);

            // --- 5. Поток митофагии ---
            // PINK1/Parkin зависит от ΔΨm: при деполяризации → митофагия активируется
            // Но при перегрузке (ros_production > threshold) — поток подавляется
            let base_flux = p.base_mitophagy_flux * mito.membrane_potential;
            let overload_penalty = if mito.ros_production > p.mitophagy_threshold {
                // Перегрузка митофагии: каждые 0.1 сверх порога снижает поток на 15%
                let excess = (mito.ros_production - p.mitophagy_threshold) / 0.1;
                (0.85_f32).powf(excess)
            } else {
                1.0
            };
            mito.mitophagy_flux = (base_flux * overload_penalty).clamp(0.0, 1.0);

            // --- 6. Вклад в кислородный щит ---
            // Здоровые митохондрии (высокий fusion, высокий potential, низкий ros_production)
            // образуют плотный щит вокруг центросомы
            mito.mito_shield_contribution =
                (mito.fusion_index * 0.4
                    + mito.membrane_potential * 0.35
                    + (1.0 - mito.ros_production) * 0.25)
                    .clamp(0.0, 1.0);
        }

        Ok(())
    }

    fn get_params(&self) -> Value {
        json!({
            "base_mutation_rate":          self.params.base_mutation_rate,
            "ros_mtdna_feedback":          self.params.ros_mtdna_feedback,
            "fission_rate":                self.params.fission_rate,
            "base_mitophagy_flux":         self.params.base_mitophagy_flux,
            "mitophagy_threshold":         self.params.mitophagy_threshold,
            "ros_production_boost":        self.params.ros_production_boost,
            "midlife_mutation_multiplier": self.params.midlife_mutation_multiplier,
            "step_count":                  self.step_count,
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
        set_f32!("base_mutation_rate",          self.params.base_mutation_rate);
        set_f32!("ros_mtdna_feedback",          self.params.ros_mtdna_feedback);
        set_f32!("fission_rate",                self.params.fission_rate);
        set_f32!("base_mitophagy_flux",         self.params.base_mitophagy_flux);
        set_f32!("mitophagy_threshold",         self.params.mitophagy_threshold);
        set_f32!("ros_production_boost",        self.params.ros_production_boost);
        set_f32!("midlife_mutation_multiplier", self.params.midlife_mutation_multiplier);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Тесты
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use cell_dt_core::hecs::World;
    use cell_dt_core::components::{MitochondrialState, CentriolarDamageState};

    fn make_world_with_entity() -> (World, cell_dt_core::hecs::Entity) {
        let mut world = World::new();
        let entity = world.spawn((
            CentriolarDamageState::default(),
            MitochondrialState::pristine(),
        ));
        (world, entity)
    }

    /// Свежие митохондрии: все показатели здоровья = 1.0 / 0.0
    #[test]
    fn test_pristine_state() {
        let s = MitochondrialState::pristine();
        assert_eq!(s.mtdna_mutations, 0.0);
        assert_eq!(s.fusion_index, 1.0);
        assert_eq!(s.ros_production, 0.0);
        assert_eq!(s.membrane_potential, 1.0);
        assert_eq!(s.mitophagy_flux, 1.0);
        assert_eq!(s.mito_shield_contribution, 1.0);
    }

    /// После 30 шагов (≈30 лет) мутации мтДНК накапливаются
    #[test]
    fn test_mutations_accumulate_over_time() {
        let mut world = World::new();
        world.spawn((CentriolarDamageState::default(), MitochondrialState::pristine()));
        let mut module = MitochondrialModule::new();
        // 30 шагов × 365.25 дней = ~30 лет
        for _ in 0..30 {
            module.step(&mut world, 365.25).unwrap();
        }
        let state: Vec<f32> = world
            .query::<&MitochondrialState>()
            .iter()
            .map(|(_, s)| s.mtdna_mutations)
            .collect();
        assert!(!state.is_empty());
        // base_rate=0.003/yr × 30yr + ROS-feedback → ожидаем ≥ 0.07
        assert!(state[0] > 0.07, "за 30 лет мутации должны накопиться: {}", state[0]);
    }

    /// ROS растёт вместе с мутациями
    #[test]
    fn test_ros_production_increases_with_mutations() {
        let (mut world, _) = make_world_with_entity();
        let mut module = MitochondrialModule::new();
        for _ in 0..50 {
            module.step(&mut world, 365.25).unwrap();
        }
        let (ros, mutations): (Vec<f32>, Vec<f32>) = world
            .query::<&MitochondrialState>()
            .iter()
            .map(|(_, s)| (s.ros_production, s.mtdna_mutations))
            .unzip();
        // После 50 лет: mutations ~0.15+ → ros_production = mutations*0.6 > 0.04
        assert!(ros[0] > 0.04, "ROS должен значимо вырасти за 50 лет: {}", ros[0]);
        assert!(mutations[0] > 0.10, "мутации за 50 лет: {}", mutations[0]);
    }

    /// mito_shield_contribution ∈ [0, 1]
    #[test]
    fn test_shield_contribution_bounded() {
        let (mut world, _) = make_world_with_entity();
        let mut module = MitochondrialModule::new();
        for _ in 0..80 {
            module.step(&mut world, 365.25).unwrap();
        }
        for (_, s) in world.query::<&MitochondrialState>().iter() {
            assert!(s.mito_shield_contribution >= 0.0);
            assert!(s.mito_shield_contribution <= 1.0);
        }
    }

    /// ros_boost() возвращает ros_production × scale
    #[test]
    fn test_ros_boost_scaling() {
        let mut s = MitochondrialState::default();
        s.ros_production = 0.4;
        let boost = s.ros_boost(0.5);
        assert!((boost - 0.2).abs() < 1e-5);
    }

    /// Все метрики остаются в [0, 1] после 90 шагов (≈90 лет)
    #[test]
    fn test_all_metrics_stay_bounded() {
        let (mut world, _) = make_world_with_entity();
        let mut module = MitochondrialModule::new();
        for _ in 0..90 {
            module.step(&mut world, 365.25).unwrap();
        }
        for (_, s) in world.query::<&MitochondrialState>().iter() {
            assert!(s.mtdna_mutations  >= 0.0 && s.mtdna_mutations  <= 1.0, "mtdna={}", s.mtdna_mutations);
            assert!(s.fusion_index     >= 0.0 && s.fusion_index     <= 1.0, "fusion={}", s.fusion_index);
            assert!(s.ros_production   >= 0.0 && s.ros_production   <= 1.0, "ros={}", s.ros_production);
            assert!(s.membrane_potential >= 0.0 && s.membrane_potential <= 1.0);
            assert!(s.mitophagy_flux   >= 0.0 && s.mitophagy_flux   <= 1.0);
            assert!(s.mito_shield_contribution >= 0.0 && s.mito_shield_contribution <= 1.0);
        }
    }

    /// fusion_index снижается при высоком ROS (фрагментация)
    #[test]
    fn test_fusion_decreases_over_time() {
        let (mut world, _) = make_world_with_entity();
        let mut module = MitochondrialModule::new();
        for _ in 0..60 {
            module.step(&mut world, 365.25).unwrap();
        }
        for (_, s) in world.query::<&MitochondrialState>().iter() {
            // fission_rate=0.05 × ros_production × 60yr → снижение на ~0.15+
            assert!(s.fusion_index < 0.90, "за 60 лет fusion_index должен заметно снизиться: {}", s.fusion_index);
        }
    }
}
