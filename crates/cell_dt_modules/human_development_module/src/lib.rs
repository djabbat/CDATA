//! Модуль развития человека — CDATA (Centriolar Damage Accumulation Theory of Aging)
//!
//! Интегрирует эмбриологию, геронтологию и теорию накопления
//! центриолярных повреждений Дж. Ткемаладзе в единую ECS-симуляцию.
//!
//! ## Ключевые механизмы
//!
//! * **Система индукторов (CentriolarInducerPair)** — у каждой центриоли свой
//!   комплект индукторов (M и D). O₂, проникая к центриолям, отщепляет их
//!   необратимо. Потентность = функция от остатка обоих комплектов.
//! * **Трек A** — дефект реснички (CEP164↓ → Shh/Wnt-сигнал↓ → ниша не
//!   восстанавливается).
//! * **Трек B** — дефект веретена (spindle_fidelity↓ → асимметрия нарушена →
//!   истощение пула стволовых клеток).
//! * **Петля ROS** — повреждённая центриоль → митофагия↓ → ROS↑ →
//!   карбонилирование и агрегаты растут → щит ослабевает → больше O₂.

use cell_dt_core::{
    SimulationModule, SimulationResult,
    hecs::World,
    components::{
        CentriolarDamageState, CentriolarInducerPair, PotencyLevel,
        TissueState, TissueType,
        CellCycleStateExtended,
        InflammagingState,
    },
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use log::{info, debug};
use rand::Rng;
use std::collections::VecDeque;

mod inducers;
mod tissues;
mod aging;
pub mod damage;
pub mod development;

pub use inducers::{
    HumanMorphogeneticLevel, HumanInducers,
    centrosomal_oxygen_level, detach_by_oxygen,
};
pub use tissues::*;
pub use aging::{AgingPhenotype, CentrioleAgingLink};
pub use damage::{DamageParams, accumulate_damage};
pub use development::{division_rate_per_year, base_ros_level, stage_for_age};

// ---------------------------------------------------------------------------
// Этапы развития (15 стадий — от зиготы до старческого возраста)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HumanDevelopmentalStage {
    Zygote,
    Cleavage,
    Morula,
    Blastocyst,
    Implantation,
    Gastrulation,
    Neurulation,
    Organogenesis,
    Fetal,
    Newborn,
    Childhood,
    Adolescence,
    Adult,
    MiddleAge,
    Elderly,
}

// ---------------------------------------------------------------------------
// Главный компонент ECS — прикрепляется к каждой стволовой нише
// ---------------------------------------------------------------------------

/// Полный CDATA-компонент развития и старения человека.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanDevelopmentComponent {
    // --- Стадии развития ---
    pub stage: HumanDevelopmentalStage,
    pub age_days: f64,
    pub morphogenetic_level: HumanMorphogeneticLevel,
    pub tissue_type: HumanTissueType,
    pub stage_history: VecDeque<(HumanDevelopmentalStage, f64)>,

    // --- CDATA: молекулярные повреждения материнской центриоли ---
    pub centriolar_damage: CentriolarDamageState,
    pub damage_rates: DamageParams,

    // --- CDATA: система индукторов (M и D комплекты на двух центриолях) ---
    /// Пара индукторных комплектов. Потентность определяется их остатком.
    pub inducers: CentriolarInducerPair,

    // --- Тканевое состояние (интегральные метрики ниши) ---
    pub tissue_state: TissueState,

    // --- Фенотипы старения ---
    pub centriole_aging: CentrioleAgingLink,
    pub active_phenotypes: Vec<AgingPhenotype>,

    pub is_alive: bool,
}

impl HumanDevelopmentComponent {
    pub fn for_tissue(tissue_type: HumanTissueType) -> Self {
        let core_type = map_tissue_type(tissue_type);
        Self {
            stage: HumanDevelopmentalStage::Zygote,
            age_days: 0.0,
            morphogenetic_level: HumanMorphogeneticLevel::Embryonic,
            tissue_type,
            stage_history: VecDeque::new(),
            centriolar_damage: CentriolarDamageState::pristine(),
            damage_rates: DamageParams::default(),
            inducers: CentriolarInducerPair::default(),
            tissue_state: TissueState::new(core_type),
            centriole_aging: CentrioleAgingLink::default(),
            active_phenotypes: Vec::new(),
            is_alive: true,
        }
    }

    pub fn age_years(&self) -> f64 { self.age_days / 365.25 }
    pub fn frailty(&self) -> f32 { 1.0 - self.tissue_state.functional_capacity }
    pub fn damage_score(&self) -> f32 { self.centriolar_damage.total_damage_score() }
    pub fn potency(&self) -> PotencyLevel { self.inducers.potency_level() }
}

impl Default for HumanDevelopmentComponent {
    fn default() -> Self { Self::for_tissue(HumanTissueType::Epithelial) }
}

// ---------------------------------------------------------------------------
// Параметры модуля
// ---------------------------------------------------------------------------

/// Параметры модуля — все поля доступны через панель управления (get/set_params).
#[derive(Debug, Clone)]
pub struct HumanDevelopmentParams {
    /// Ускорение времени: 1.0 → 1 шаг = 1 день
    pub time_acceleration: f64,
    pub enable_aging: bool,
    pub enable_morphogenesis: bool,
    pub tissue_detail_level: usize,

    // --- Параметры индукторов (панель управления) ---
    /// Начальный размер M-комплекта (материнская центриоль) [1..100]
    pub mother_inducer_count: u32,
    /// Начальный размер D-комплекта (дочерняя центриоль) [1..100]
    pub daughter_inducer_count: u32,
    /// Базовая вероятность отщепления на шаг при oxygen_level=1.0 [0..1]
    pub base_detach_probability: f32,
    /// Доля вероятности, приходящаяся на материнскую центриоль [0..1]
    /// Возраст не является причиной — только корректирует этот параметр.
    pub mother_bias: f32,
    /// Вклад возраста (лет) в mother_bias (вспомогательный параметр)
    pub age_bias_coefficient: f32,
}

impl Default for HumanDevelopmentParams {
    fn default() -> Self {
        Self {
            time_acceleration: 1.0,
            enable_aging: true,
            enable_morphogenesis: true,
            tissue_detail_level: 3,
            mother_inducer_count: 10,
            daughter_inducer_count: 8,
            base_detach_probability: 0.002,
            mother_bias: 0.6,
            age_bias_coefficient: 0.003,
        }
    }
}

// ---------------------------------------------------------------------------
// Модуль
// ---------------------------------------------------------------------------

pub struct HumanDevelopmentModule {
    params: HumanDevelopmentParams,
    step_count: u64,
}

impl HumanDevelopmentModule {
    pub fn new() -> Self {
        Self { params: HumanDevelopmentParams::default(), step_count: 0 }
    }

    pub fn with_params(params: HumanDevelopmentParams) -> Self {
        Self { params, step_count: 0 }
    }

    // -----------------------------------------------------------------------

    fn update_stage(comp: &mut HumanDevelopmentComponent) {
        let age = comp.age_days;
        let new_stage = if      age < 1.0   { HumanDevelopmentalStage::Zygote }
        else if age < 3.0   { HumanDevelopmentalStage::Cleavage }
        else if age < 4.0   { HumanDevelopmentalStage::Morula }
        else if age < 7.0   { HumanDevelopmentalStage::Blastocyst }
        else if age < 14.0  { HumanDevelopmentalStage::Implantation }
        else if age < 21.0  { HumanDevelopmentalStage::Gastrulation }
        else if age < 28.0  { HumanDevelopmentalStage::Neurulation }
        else if age < 56.0  { HumanDevelopmentalStage::Organogenesis }
        else if age < 280.0 { HumanDevelopmentalStage::Fetal }
        else if age < 365.0 { HumanDevelopmentalStage::Newborn }
        else if age < 4380.0  { HumanDevelopmentalStage::Childhood }   // 12 лет
        else if age < 6570.0  { HumanDevelopmentalStage::Adolescence } // 18 лет
        else if age < 18250.0 { HumanDevelopmentalStage::Adult }       // 50 лет
        else if age < 25550.0 { HumanDevelopmentalStage::MiddleAge }   // 70 лет
        else                  { HumanDevelopmentalStage::Elderly };

        if new_stage != comp.stage {
            comp.stage_history.push_back((new_stage, age));
            comp.stage = new_stage;
        }
    }

    /// Трек A + Трек B: обновить тканевое состояние из молекулярных повреждений.
    fn update_tissue_state(comp: &mut HumanDevelopmentComponent) {
        let dam = &comp.centriolar_damage;

        // Трек A: функция реснички определяет темп регенерации
        comp.tissue_state.regeneration_tempo = dam.ciliary_function;

        // Трек B: spindle_fidelity↓ → вероятность про-дифф. деления → потеря пула
        let pool_loss = dam.pool_exhaustion_probability();
        comp.tissue_state.stem_cell_pool = (1.0 - pool_loss).max(0.0);

        // Апоптоз (M=0, D=0) — пул обнуляется немедленно
        if comp.inducers.is_apoptotic() {
            comp.tissue_state.stem_cell_pool = 0.0;
        }

        comp.tissue_state.senescent_fraction =
            (dam.total_damage_score() * 0.85).min(1.0);
        comp.tissue_state.update_functional_capacity();
    }

    /// O₂-зависимое отщепление индукторов.
    ///
    /// Кислород проникает к центриолям тем сильнее, чем слабее
    /// митохондриальный щит (ослабленный ROS и агрегатами).
    fn apply_oxygen_detachment(comp: &mut HumanDevelopmentComponent, rng: &mut impl Rng) {
        let oxygen = centrosomal_oxygen_level(&comp.centriolar_damage);
        let age = comp.age_years() as f32;
        if oxygen > 0.01 {
            detach_by_oxygen(&mut comp.inducers, oxygen, age, rng);
        }
    }

    fn update_aging_phenotypes(comp: &mut HumanDevelopmentComponent) {
        let dam = &comp.centriolar_damage;

        comp.centriole_aging.cilia_loss =
            (1.0 - dam.ciliary_function).max(0.0);
        comp.centriole_aging.ptm_accumulation =
            (dam.tubulin_hyperacetylation + dam.phosphorylation_dysregulation) / 2.0;
        comp.centriole_aging.cycle_dysregulation =
            (1.0 - dam.spindle_fidelity).max(0.0);
        comp.centriole_aging.asymmetry_loss = dam.symmetric_division_probability();
        comp.centriole_aging.satellite_accumulation = dam.protein_aggregates;

        comp.active_phenotypes.clear();
        let total = dam.total_damage_score();

        if total > 0.10 {
            comp.active_phenotypes.push(AgingPhenotype::ReducedProliferation);
        }
        if dam.protein_aggregates > 0.20 {
            comp.active_phenotypes.push(AgingPhenotype::ProteinAggregation);
        }
        if dam.ros_level > 0.30 {
            comp.active_phenotypes.push(AgingPhenotype::MitochondrialDysfunction);
        }
        if comp.centriole_aging.ptm_accumulation > 0.15 {
            comp.active_phenotypes.push(AgingPhenotype::EpigeneticChanges);
        }
        if comp.tissue_state.senescent_fraction > 0.30 {
            comp.active_phenotypes.push(AgingPhenotype::SenescentAccumulation);
        }
        if comp.centriole_aging.cilia_loss > 0.30 {
            comp.active_phenotypes.push(AgingPhenotype::SignalingDysregulation);
        }
        if dam.protein_aggregates > 0.30 {
            comp.active_phenotypes.push(AgingPhenotype::ProteostasisLoss);
        }
        if comp.tissue_state.stem_cell_pool < 0.50 {
            comp.active_phenotypes.push(AgingPhenotype::StemCellExhaustion);
        }
        if total > 0.50 {
            comp.active_phenotypes.push(AgingPhenotype::AlteredCommunication);
        }
    }
}

impl Default for HumanDevelopmentModule {
    fn default() -> Self { Self::new() }
}

impl SimulationModule for HumanDevelopmentModule {
    fn name(&self) -> &str { "human_development_module" }

    fn step(&mut self, world: &mut World, dt: f64) -> SimulationResult<()> {
        self.step_count += 1;
        let dt_days  = dt * self.params.time_acceleration;
        let dt_years = (dt_days / 365.25) as f32;

        debug!("Human development step {}, dt_days={:.3}", self.step_count, dt_days);

        let mut rng = rand::thread_rng();

        // Шаг 1: обновить HumanDevelopmentComponent (основная логика CDATA)
        // Также читаем InflammagingState (опционально) — пишется myeloid_shift_module.
        {
        let mut query = world
            .query::<(&mut HumanDevelopmentComponent, Option<&InflammagingState>)>();

        for (_, (comp, inflammaging_opt)) in query.iter() {
            if !comp.is_alive { continue; }

            // Предварительно извлекаем значения из InflammagingState (если модуль активен)
            let infl_ros_boost        = inflammaging_opt.map_or(0.0, |i| i.ros_boost);
            let infl_niche_impairment = inflammaging_opt.map_or(0.0, |i| i.niche_impairment);
            let infl_sasp             = inflammaging_opt.map_or(0.0, |i| i.sasp_intensity);

            // 1. Возраст
            comp.age_days += dt_days;

            // 2. Стадия и морфогенетический уровень
            if self.params.enable_morphogenesis {
                comp.morphogenetic_level =
                    HumanInducers::get_morphogenetic_level(comp.age_days);
                Self::update_stage(comp);
            }

            if self.params.enable_aging {
                // 3. Молекулярные повреждения (5 типов + ROS-петля)
                let age_years = comp.age_years() as f32;
                accumulate_damage(
                    &mut comp.centriolar_damage,
                    &comp.damage_rates,
                    age_years,
                    dt_years,
                );

                // 3б. Inflammaging-буст ROS (петля от миелоидного сдвига).
                // Применяем после accumulate_damage, т.к. та перезаписывает ros_level.
                // Лаг в один шаг допустим: myeloid_shift_module запускается следом.
                if infl_ros_boost > 0.0 {
                    comp.centriolar_damage.ros_level =
                        (comp.centriolar_damage.ros_level * (1.0 + infl_ros_boost)).min(1.0);
                    // Пересчитать производные метрики (spindle_fidelity, ciliary_function)
                    comp.centriolar_damage.update_functional_metrics();
                }

                // 4. O₂-зависимое отщепление индукторов
                Self::apply_oxygen_detachment(comp, &mut rng);

                // 5. Тканевое состояние (Трек A + Трек B)
                Self::update_tissue_state(comp);

                // 5б. Niche impairment от воспаления (снижает темп регенерации)
                if infl_niche_impairment > 0.0 {
                    comp.tissue_state.regeneration_tempo =
                        (comp.tissue_state.regeneration_tempo
                            * (1.0 - infl_niche_impairment)).max(0.0);
                    comp.tissue_state.update_functional_capacity();
                }

                // 6. Фенотипы старения
                Self::update_aging_phenotypes(comp);

                // 6б. ImmuneDecline — активируется при выраженном SASP
                if infl_sasp > 0.4
                    && !comp.active_phenotypes.contains(&AgingPhenotype::ImmuneDecline)
                {
                    comp.active_phenotypes.push(AgingPhenotype::ImmuneDecline);
                }

                // 7. Смерть:
                //    — молекулярный сенесценс (total_damage > 0.75 ≈ 78 лет)
                //    — апоптоз через исчерпание обоих комплектов (M=0, D=0)
                //    — критическая дряхлость (frailty ≥ 0.97)
                if comp.centriolar_damage.is_senescent
                    || comp.inducers.is_apoptotic()
                    || comp.frailty() >= 0.97
                {
                    comp.is_alive = false;
                    debug!(
                        "Niche {:?} died at age {:.1} yr \
                         (senescent={}, potency={:?}, frailty={:.3})",
                        comp.tissue_type,
                        comp.age_years(),
                        comp.centriolar_damage.is_senescent,
                        comp.inducers.potency_level(),
                        comp.frailty(),
                    );
                }
            }
        }
        } // drop query — освобождаем borrow на world

        // Шаг 2: синхронизировать отдельный ECS-компонент CentriolarDamageState
        // чтобы stem_cell_hierarchy и asymmetric_division могли читать повреждения
        // без зависимости от human_development_module.
        for (_, (dev, standalone)) in
            world.query_mut::<(&HumanDevelopmentComponent, &mut CentriolarDamageState)>()
        {
            *standalone = dev.centriolar_damage.clone();
        }

        Ok(())
    }

    fn get_params(&self) -> Value {
        json!({
            "time_acceleration":       self.params.time_acceleration,
            "enable_aging":            self.params.enable_aging,
            "enable_morphogenesis":    self.params.enable_morphogenesis,
            "tissue_detail_level":     self.params.tissue_detail_level,
            "mother_inducer_count":    self.params.mother_inducer_count,
            "daughter_inducer_count":  self.params.daughter_inducer_count,
            "base_detach_probability": self.params.base_detach_probability,
            "mother_bias":             self.params.mother_bias,
            "age_bias_coefficient":    self.params.age_bias_coefficient,
            "step_count":              self.step_count,
        })
    }

    fn set_params(&mut self, params: &Value) -> SimulationResult<()> {
        macro_rules! set_f64 {
            ($key:literal, $field:expr) => {
                if let Some(v) = params.get($key).and_then(|v| v.as_f64()) {
                    $field = v;
                }
            };
        }
        macro_rules! set_f32 {
            ($key:literal, $field:expr) => {
                if let Some(v) = params.get($key).and_then(|v| v.as_f64()) {
                    $field = v as f32;
                }
            };
        }
        macro_rules! set_u32 {
            ($key:literal, $field:expr) => {
                if let Some(v) = params.get($key).and_then(|v| v.as_u64()) {
                    $field = v as u32;
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

        set_f64!("time_acceleration",       self.params.time_acceleration);
        set_bool!("enable_aging",           self.params.enable_aging);
        set_bool!("enable_morphogenesis",   self.params.enable_morphogenesis);
        set_u32!("mother_inducer_count",    self.params.mother_inducer_count);
        set_u32!("daughter_inducer_count",  self.params.daughter_inducer_count);
        set_f32!("base_detach_probability", self.params.base_detach_probability);
        set_f32!("mother_bias",             self.params.mother_bias);
        set_f32!("age_bias_coefficient",    self.params.age_bias_coefficient);
        Ok(())
    }

    fn initialize(&mut self, world: &mut World) -> SimulationResult<()> {
        info!("Initializing human development module (CDATA)");

        let entities: Vec<_> = world
            .query::<&CellCycleStateExtended>()
            .iter()
            .map(|(e, _)| e)
            .collect();

        let tissue_cycle = [
            HumanTissueType::Neural,
            HumanTissueType::Blood,
            HumanTissueType::Epithelial,
            HumanTissueType::Muscle,
            HumanTissueType::Skin,
        ];

        let count  = entities.len();
        let m_max  = self.params.mother_inducer_count;
        let d_max  = self.params.daughter_inducer_count;

        for (i, &entity) in entities.iter().enumerate() {
            if !world.contains(entity) { continue; }
            let tissue = tissue_cycle[i % tissue_cycle.len()];
            let mut comp = HumanDevelopmentComponent::for_tissue(tissue);
            comp.inducers = CentriolarInducerPair::zygote(m_max, d_max);
            comp.inducers.detachment_params.base_detach_probability =
                self.params.base_detach_probability;
            comp.inducers.detachment_params.mother_bias = self.params.mother_bias;
            comp.inducers.detachment_params.age_bias_coefficient =
                self.params.age_bias_coefficient;
            // Standalone ECS-компоненты для межмодульного взаимодействия:
            // CentriolarDamageState — синхронизируется в step() для других модулей.
            // InflammagingState     — пишется myeloid_shift_module, читается здесь.
            world.insert_one(entity, CentriolarDamageState::pristine())?;
            world.insert_one(entity, InflammagingState::default())?;
            world.insert_one(entity, comp)?;
        }

        info!(
            "Initialized CDATA for {} niches (M_max={}, D_max={}, bias={:.2})",
            count, m_max, d_max, self.params.mother_bias
        );
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Вспомогательные функции
// ---------------------------------------------------------------------------

pub fn map_tissue_type(human_type: HumanTissueType) -> TissueType {
    match human_type {
        HumanTissueType::Neural => TissueType::Neural,
        HumanTissueType::Blood  => TissueType::Hematopoietic,
        HumanTissueType::Epithelial
        | HumanTissueType::Liver
        | HumanTissueType::Kidney
        | HumanTissueType::Lung => TissueType::IntestinalCrypt,
        HumanTissueType::Muscle
        | HumanTissueType::Heart => TissueType::Muscle,
        _ => TissueType::Skin,
    }
}
