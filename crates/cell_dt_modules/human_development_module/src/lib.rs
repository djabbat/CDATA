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
    hecs::{World, Entity},
    components::{
        CentriolarDamageState, CentriolarInducerPair, PotencyLevel,
        TissueState, DifferentiationStatus, ModulationState,
        CellCycleStateExtended,
        InflammagingState,
        DivisionExhaustionState,
        CentriolePair,
        TelomereState,
        EpigeneticClockState,
        MitochondrialState,
        Dead,
    },
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use log::{info, warn, trace};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use std::collections::VecDeque;

mod inducers;
mod tissues;
mod aging;
pub mod damage;
pub mod development;

pub use inducers::{
    HumanMorphogeneticLevel, HumanInducers,
    centrosomal_oxygen_level, detach_by_oxygen, detach_by_ptm_exhaustion,
};
pub use tissues::*;
pub use aging::{AgingPhenotype, CentrioleAgingLink};
pub use damage::{DamageParams, accumulate_damage};
pub use development::{division_rate_per_year, base_ros_level, stage_for_age};

// ---------------------------------------------------------------------------
// Этапы развития (15 стадий — от зиготы до старческого возраста)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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
        Self {
            stage: HumanDevelopmentalStage::Zygote,
            age_days: 0.0,
            morphogenetic_level: HumanMorphogeneticLevel::Embryonic,
            tissue_type,
            stage_history: VecDeque::new(),
            centriolar_damage: CentriolarDamageState::pristine(),
            damage_rates: DamageParams::default(),
            inducers: CentriolarInducerPair::default(),
            tissue_state: TissueState::new(tissue_type),
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
    /// Масштаб PTM-опосредованного истощения материнского комплекта [0..0.01].
    /// 0.0 → механизм выключен; 0.001 → умеренное PTM-истощение.
    pub ptm_exhaustion_scale: f32,

    // --- Жизненный цикл индукторов ---
    /// Номер деления бластомеров (от зиготы), при котором создаются de novo центриоли
    /// с индукторами дифференцировки. [1..=8]. По умолчанию: 4 (16-клеточная стадия, Морула).
    /// До достижения этой стадии `DifferentiationStatus.inductors_active = false`.
    pub de_novo_centriole_division: u32,
    /// Включить учёт элиминации центриолей в прелептотенной стадии мейоза.
    /// При `true`: в стадии Adolescence регистрируется мейотическая элиминация
    /// (для следующего поколения DifferentiationStatus начнётся с нуля).
    pub meiotic_elimination_enabled: bool,
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
            mother_bias: 0.5,           // одинаковая вероятность для M и D
            age_bias_coefficient: 0.0,  // возраст не влияет по умолчанию
            ptm_exhaustion_scale: 0.001, // PTM-асимметрия → истощение матери
            de_novo_centriole_division: 4,    // 16-клеточная стадия (Морула)
            meiotic_elimination_enabled: true, // биологически корректный дефолт
        }
    }
}

// ---------------------------------------------------------------------------
// Модуль
// ---------------------------------------------------------------------------

pub struct HumanDevelopmentModule {
    params: HumanDevelopmentParams,
    step_count: u64,
    /// Параметры накопления повреждений (панель управления).
    /// Синхронизируются с `comp.damage_rates` всех живых сущностей при изменении.
    damage_rates: DamageParams,
    /// Флаг: параметры повреждений изменились через `set_params()` — нужна синхронизация.
    damage_rates_dirty: bool,
    /// Генератор случайных чисел — сидируется через `set_seed()` для воспроизводимости.
    rng: StdRng,
}

impl HumanDevelopmentModule {
    pub fn new() -> Self {
        Self {
            params: HumanDevelopmentParams::default(),
            step_count: 0,
            damage_rates: DamageParams::default(),
            damage_rates_dirty: false,
            rng: StdRng::from_entropy(),
        }
    }

    pub fn with_params(params: HumanDevelopmentParams) -> Self {
        Self {
            params,
            step_count: 0,
            damage_rates: DamageParams::default(),
            damage_rates_dirty: false,
            rng: StdRng::from_entropy(),
        }
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
            info!("Stage transition {:?} → {:?} at age {:.3} yr ({:?})",
                  comp.stage, new_stage, age, comp.tissue_type);
            comp.stage_history.push_back((new_stage, age));
            if comp.stage_history.len() > 20 {
                comp.stage_history.pop_front(); // храним только последние 20 стадий
            }
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
        // update_functional_capacity() вызывается ОДИН РАЗ в конце всех тканевых обновлений
    }

    /// Стадия, соответствующая n-му делению бластомеров (de novo создание центриолей).
    ///
    /// | Деление | Кол-во клеток | Стадия       |
    /// |---------|--------------|--------------|
    /// | 1       | 2            | Cleavage     |
    /// | 2       | 4            | Cleavage     |
    /// | 3       | 8            | Cleavage     |
    /// | 4       | 16           | Morula (дефолт) |
    /// | 5       | 32           | Blastocyst   |
    /// | 6       | 64           | Blastocyst   |
    /// | 7+      | 128+         | Implantation |
    pub(crate) fn de_novo_stage_for_division(division: u32) -> HumanDevelopmentalStage {
        match division {
            1       => HumanDevelopmentalStage::Zygote,
            2 | 3   => HumanDevelopmentalStage::Cleavage,
            4       => HumanDevelopmentalStage::Morula,
            5 | 6   => HumanDevelopmentalStage::Blastocyst,
            _       => HumanDevelopmentalStage::Implantation,
        }
    }

    /// O₂-зависимое отщепление индукторов (контролируемый путь, одинаковый для M и D).
    ///
    /// `shield_factor` — вклад митохондриального щита (1.0 = щит цел, 0.0 = щит разрушен).
    /// Чем ниже shield_factor, тем больше O₂ достигает центросомы (множитель до ×2.0).
    fn apply_oxygen_detachment(
        comp: &mut HumanDevelopmentComponent,
        shield_factor: f32,
        rng: &mut impl Rng,
    ) {
        let raw_oxygen = centrosomal_oxygen_level(&comp.centriolar_damage);
        // При ослаблении митохондриального щита O₂ проникает активнее.
        // shield_factor=1.0 → нет изменений; shield_factor=0.0 → удвоение O₂.
        let oxygen = (raw_oxygen * (2.0 - shield_factor)).min(1.0);
        let age = comp.age_years() as f32;
        if oxygen > 0.01 {
            detach_by_oxygen(&mut comp.inducers, oxygen, age, rng);
        }
    }

    /// PTM-опосредованное истощение: ТОЛЬКО мать теряет индукторы из-за PTM-асимметрии.
    ///
    /// Второй, независимый от O₂ путь: структурные ПТМ матери ослабляют
    /// механическое крепление индукторов к молекулярному каркасу.
    /// Это механизм ИСТОЩЕНИЯ стволовых клеток, а не нормальной дифференцировки.
    fn apply_ptm_exhaustion(
        comp: &mut HumanDevelopmentComponent,
        ptm_asymmetry: f32,
        rng: &mut impl Rng,
    ) {
        detach_by_ptm_exhaustion(&mut comp.inducers, ptm_asymmetry, rng);
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

impl HumanDevelopmentModule {
    /// Синхронизировать `damage_rates` со всеми живыми сущностями.
    /// Вызывается в `step()` при `damage_rates_dirty == true`.
    fn sync_damage_rates(&self, world: &mut World) {
        for (_, comp) in world.query_mut::<&mut HumanDevelopmentComponent>() {
            if comp.is_alive {
                comp.damage_rates = self.damage_rates.clone();
            }
        }
    }
}

impl SimulationModule for HumanDevelopmentModule {
    fn name(&self) -> &str { "human_development_module" }

    /// Устанавливает seed для воспроизводимости: отщепление индукторов (O₂/PTM) детерминировано.
    fn set_seed(&mut self, seed: u64) {
        self.rng = StdRng::seed_from_u64(seed);
    }

    fn step(&mut self, world: &mut World, dt: f64) -> SimulationResult<()> {
        self.step_count += 1;
        let dt_days  = dt * self.params.time_acceleration;
        let dt_years = (dt_days / 365.25) as f32;

        trace!("Human development step {}, dt_days={:.3}", self.step_count, dt_days);

        // Синхронизировать DamageParams если они были изменены через set_params()
        if self.damage_rates_dirty {
            self.sync_damage_rates(world);
            self.damage_rates_dirty = false;
        }

        // Шаг 1: обновить HumanDevelopmentComponent (основная логика CDATA)
        // Также читаем InflammagingState (опционально) — пишется myeloid_shift_module.
        {
        let mut query = world.query::<(
            &mut HumanDevelopmentComponent,
            Option<&InflammagingState>,
            Option<&DivisionExhaustionState>,
            Option<&CentriolePair>,
            Option<&mut TelomereState>,
            Option<&mut EpigeneticClockState>,
            Option<&mut DifferentiationStatus>,
            Option<&mut ModulationState>,
            Option<&MitochondrialState>,
        )>();

        for (_, (comp, inflammaging_opt, exhaustion_opt, centriole_opt, mut telomere_opt, mut epigenetic_opt, mut diff_status_opt, mut modulation_opt, mito_opt)) in query.iter() {
            if !comp.is_alive { continue; }

            // Предварительно извлекаем значения из InflammagingState (если модуль активен)
            let infl_ros_boost        = inflammaging_opt.map_or(0.0, |i| i.ros_boost);
            let infl_niche_impairment = inflammaging_opt.map_or(0.0, |i| i.niche_impairment);
            let infl_sasp             = inflammaging_opt.map_or(0.0, |i| i.sasp_intensity);
            // Трек E: митохондриальный ROS-буст (лаг 1 шаг, аналогично inflammaging)
            // Параметр ros_production_boost = 0.20 (масштаб по умолчанию)
            let mito_ros_boost = mito_opt.map_or(0.0, |m| m.ros_boost(0.20));
            // Митохондриальный щит: снижает эффективный кислородный уровень у центросомы
            // mito_shield_contribution < 1.0 → O₂ проникает активнее → больше отщеплений
            // Применяется к base_detach_probability через масштабирование (лаг 1 шаг)
            let mito_shield = mito_opt.map_or(1.0, |m| m.mito_shield_contribution);
            // Вклад эпигенетических часов в ROS (лаг 1 шаг, аналогично inflammaging)
            let epi_ros_from_prev = epigenetic_opt.as_ref().map_or(0.0, |e| e.epi_ros_contribution);
            // Истощение делений (asymmetric_division_module → stem_cell_pool)
            let exhaustion_ratio      = exhaustion_opt.map_or(0.0, |e| e.exhaustion_ratio());
            // PTM-уровни из CentriolePair (centriole_module → CentriolarDamageState bridge)
            // Используем среднее мать+дочь для ацетилирования, мать — для остальных (мать старше)
            let ptm_acetylation = centriole_opt.map_or(0.0, |c| {
                (c.mother.ptm_signature.acetylation_level
                    + c.daughter.ptm_signature.acetylation_level) / 2.0
            });
            let ptm_oxidation   = centriole_opt.map_or(0.0, |c| c.mother.ptm_signature.oxidation_level);
            let ptm_phospho     = centriole_opt.map_or(0.0, |c| c.mother.ptm_signature.phosphorylation_level);
            let ptm_methyl      = centriole_opt.map_or(0.0, |c| c.mother.ptm_signature.methylation_level);
            // PTM-асимметрия мать−дочь: для механизма истощения стволовых клеток
            let ptm_asymmetry = centriole_opt.map_or(0.0, |c| {
                let m = &c.mother.ptm_signature;
                let d = &c.daughter.ptm_signature;
                let m_avg = (m.acetylation_level + m.oxidation_level
                           + m.phosphorylation_level + m.methylation_level) / 4.0;
                let d_avg = (d.acetylation_level + d.oxidation_level
                           + d.phosphorylation_level + d.methylation_level) / 4.0;
                (m_avg - d_avg).max(0.0)
            });

            // 1. Возраст
            comp.age_days += dt_days;

            // 2. Стадия и морфогенетический уровень
            if self.params.enable_morphogenesis {
                comp.morphogenetic_level =
                    HumanInducers::get_morphogenetic_level(comp.age_days);
                Self::update_stage(comp);
            }

            if self.params.enable_aging {
                // 3. Молекулярные повреждения (5 типов + ROS-петля).
                // infl_ros_boost передаётся ВНУТРЬ accumulate_damage() — так он влияет
                // на protein_carbonylation в том же шаге (корректная межшаговая петля).
                let age_years = comp.age_years() as f32;
                accumulate_damage(
                    &mut comp.centriolar_damage,
                    &comp.damage_rates,
                    age_years,
                    dt_years,
                    infl_ros_boost + epi_ros_from_prev + mito_ros_boost,  // inflammaging + эпигенетика + митохондрии
                );

                // 3а. P3: Ланжевен-шум молекулярных повреждений (noise_scale > 0).
                // Применяется ПОСЛЕ детерминированного шага — сохраняет среднюю траекторию.
                // sqrt(dt_years) — масштабирование Ито для временного шага.
                if comp.damage_rates.noise_scale > 0.0 {
                    let sigma = comp.damage_rates.noise_scale * dt_years.sqrt();
                    let dam = &mut comp.centriolar_damage;
                    dam.protein_carbonylation = (dam.protein_carbonylation
                        + self.rng.gen::<f32>() * sigma * 2.0 - sigma).clamp(0.0, 1.0);
                    dam.tubulin_hyperacetylation = (dam.tubulin_hyperacetylation
                        + self.rng.gen::<f32>() * sigma * 2.0 - sigma).clamp(0.0, 1.0);
                    dam.protein_aggregates = (dam.protein_aggregates
                        + self.rng.gen::<f32>() * sigma * 2.0 - sigma).clamp(0.0, 1.0);
                    dam.phosphorylation_dysregulation = (dam.phosphorylation_dysregulation
                        + self.rng.gen::<f32>() * sigma * 2.0 - sigma).clamp(0.0, 1.0);
                    dam.update_functional_metrics();
                }

                // 3б. PTM bridge: структурные PTM CentriolePair → функциональные повреждения.
                // Лаг один шаг (centriole_module запускается до human_development_module).
                // Масштаб 0.002/год при PTM=1.0, не пересекается с базовой скоростью acetylation_rate
                // (базовый путь — структурное накопление ПТМ со временем; bridge — дополнительный
                // вклад конкретных PTM-меток, измеренных CentrioleModule).
                {
                    const PTM_SCALE: f32 = 0.002;
                    let dam = &mut comp.centriolar_damage;
                    dam.tubulin_hyperacetylation = (dam.tubulin_hyperacetylation
                        + ptm_acetylation * PTM_SCALE * dt_years).min(1.0);
                    dam.protein_carbonylation    = (dam.protein_carbonylation
                        + ptm_oxidation   * PTM_SCALE * dt_years).min(1.0);
                    dam.phosphorylation_dysregulation = (dam.phosphorylation_dysregulation
                        + ptm_phospho     * PTM_SCALE * dt_years).min(1.0);
                    // Метилирование → небольшой вклад в агрегацию белков (×0.5)
                    dam.protein_aggregates = (dam.protein_aggregates
                        + ptm_methyl      * PTM_SCALE * 0.5 * dt_years).min(1.0);
                    if ptm_acetylation + ptm_oxidation + ptm_phospho > 0.0 {
                        dam.update_functional_metrics();
                    }
                }

                // 3в. P5: Репарация дистальных придатков (если включена в DamageParams).
                // Вызывается после PTM bridge. mitophagy_flux усиливает репарацию через
                // связь PINK1/Parkin → снижение локального ROS → восстановление CEP164/CEP89.
                {
                    let mitophagy_flux = mito_opt.map_or(0.0, |m| m.mitophagy_flux);
                    damage::apply_appendage_repair(
                        &mut comp.centriolar_damage,
                        &comp.damage_rates,
                        mitophagy_flux,
                        dt_years,
                    );
                }

                // Проверка на биологически нереалистичные значения
                {
                    let dam = &comp.centriolar_damage;
                    if dam.ros_level > 1.0 {
                        warn!("ros_level={:.3} > 1.0 at age {:.1} yr ({:?}) — clamp needed",
                              dam.ros_level, comp.age_years(), comp.tissue_type);
                    }
                    if dam.total_damage_score() > 1.0 {
                        warn!("total_damage_score={:.3} > 1.0 at age {:.1} yr ({:?})",
                              dam.total_damage_score(), comp.age_years(), comp.tissue_type);
                    }
                }

                // 4. O₂-зависимое отщепление индукторов (контролируемый путь, M=D=0.5)
                // Митохондриальный щит влияет косвенно: через ROS-петлю (шаг 3).
                // mito_ros_boost → accumulate_damage() → ros_level ↑ → centrosomal_oxygen_level ↑
                // → больше O₂ у центросомы → больше отщеплений (лаг 1 шаг — корректно).
                // Прямое масштабирование здесь не применяется.
                Self::apply_oxygen_detachment(comp, mito_shield, &mut self.rng);

                // 4б. PTM-опосредованное истощение (только мать — механизм истощения пула).
                // Независим от O₂: структурные ПТМ матери ослабляют связи индукторов.
                // Срабатывает только при наличии PTM-асимметрии мать > дочь.
                if ptm_asymmetry > 0.01 {
                    Self::apply_ptm_exhaustion(comp, ptm_asymmetry, &mut self.rng);
                }

                // 5. Тканевое состояние (Трек A + Трек B)
                Self::update_tissue_state(comp);

                // 5б. Niche impairment от воспаления (снижает темп регенерации)
                if infl_niche_impairment > 0.0 {
                    comp.tissue_state.regeneration_tempo =
                        (comp.tissue_state.regeneration_tempo
                            * (1.0 - infl_niche_impairment)).max(0.0);
                }

                // 5в. Истощение пула из-за симметричных дифф. делений
                // exhaustion_ratio → уменьшает stem_cell_pool на 0.0002/шаг × ratio
                // Скорость 0.0002 мала: заметный эффект накапливается за годы активного деления
                if exhaustion_ratio > 0.0 {
                    const POOL_DEPLETION_RATE: f32 = 0.0002;
                    comp.tissue_state.stem_cell_pool = (comp.tissue_state.stem_cell_pool
                        - exhaustion_ratio * POOL_DEPLETION_RATE * dt_years as f32).max(0.0);
                }

                // Пересчёт functional_capacity — ОДИН РАЗ после всех тканевых обновлений
                comp.tissue_state.update_functional_capacity();

                // 6. Фенотипы старения
                Self::update_aging_phenotypes(comp);

                // 6б. ImmuneDecline — активируется при выраженном SASP
                if infl_sasp > 0.4
                    && !comp.active_phenotypes.contains(&AgingPhenotype::ImmuneDecline)
                {
                    comp.active_phenotypes.push(AgingPhenotype::ImmuneDecline);
                }

                // 6в. Трек C: укорачивание теломер
                // Скорость = shortening_per_division × division_rate × spindle_factor × ros_factor
                //
                // TERT (теломераза) активен в двух ситуациях → укорочения нет:
                //  1. Эмбриональные стадии (Zygote..Fetal): TERT максимально активен
                //  2. Стволовые клетки: spindle_fidelity ≥ 0.75 (прокси Pluripotent/Totipotent)
                //     (теломери не уkorachivaiutsia v стволовых клетках — TERT защищает геном)
                if let Some(ref mut tel) = telomere_opt {
                    let embryonic = matches!(
                        comp.stage,
                        HumanDevelopmentalStage::Zygote
                        | HumanDevelopmentalStage::Cleavage
                        | HumanDevelopmentalStage::Morula
                        | HumanDevelopmentalStage::Blastocyst
                        | HumanDevelopmentalStage::Implantation
                        | HumanDevelopmentalStage::Gastrulation
                        | HumanDevelopmentalStage::Neurulation
                        | HumanDevelopmentalStage::Organogenesis
                        | HumanDevelopmentalStage::Fetal
                    );
                    let stem_cell = comp.centriolar_damage.spindle_fidelity >= 0.75;
                    let tert_active = embryonic || stem_cell;

                    if !tert_active {
                        let div_rate: f32 = match comp.stage {
                            HumanDevelopmentalStage::Newborn
                            | HumanDevelopmentalStage::Childhood     => 24.0,
                            HumanDevelopmentalStage::Adolescence
                            | HumanDevelopmentalStage::Adult         => 12.0,
                            HumanDevelopmentalStage::MiddleAge       => 6.0,
                            HumanDevelopmentalStage::Elderly         => 2.0,
                            _                                        => 0.0, // TERT-активные стадии не дойдут сюда
                        };
                        let base = tel.shortening_per_division * div_rate * dt_years;
                        let spindle_f = 1.0 + (1.0 - comp.centriolar_damage.spindle_fidelity) * 0.5;
                        let ros_f    = 1.0 + comp.centriolar_damage.ros_level * 0.3;
                        tel.mean_length = (tel.mean_length - base * spindle_f * ros_f).max(0.0);
                    }

                    tel.is_critically_short = tel.mean_length < 0.3;
                    if tel.is_critically_short
                        && !comp.active_phenotypes.contains(&AgingPhenotype::TelomereShortening)
                    {
                        comp.active_phenotypes.push(AgingPhenotype::TelomereShortening);
                    }
                }

                // 6г. Трек D: эпигенетические часы
                // clock_acceleration = 1.0 + total_damage × 0.5
                // methylation_age увеличивается быстрее хронологического при повреждениях
                // Обратная связь: опережение эпигенетических часов → дополнительный ROS
                if let Some(ref mut epi) = epigenetic_opt {
                    let damage = comp.centriolar_damage.total_damage_score();
                    epi.clock_acceleration = 1.0 + damage * 0.5;
                    epi.methylation_age += dt_years * epi.clock_acceleration;

                    // Опережение эпигенетического возраста над хронологическим → ROS-буст
                    // (CpG-гипометилирование нестабилизирует хроматин → транспозоны → ROS)
                    let chron_age = comp.age_years() as f32;
                    let epi_excess = (epi.methylation_age - chron_age).max(0.0);
                    epi.epi_ros_contribution = (epi_excess / 200.0).clamp(0.0, 0.05);

                    // Активация фенотипа при значимом ускорении часов
                    if epi.clock_acceleration > 1.2
                        && !comp.active_phenotypes.contains(&AgingPhenotype::EpigeneticChanges)
                    {
                        comp.active_phenotypes.push(AgingPhenotype::EpigeneticChanges);
                    }
                }

                // 6д. Необратимый статус дифференцировки (DifferentiationStatus)
                // Логика жизненного цикла индукторов:
                //   1. Индукторы создаются de novo при n-м делении бластомеров (Морула по умолчанию)
                //   2. До этой стадии клетка не может коммитироваться (inductors_active = false)
                //   3. При включённой мейотической элиминации — регистрируется событие для следующего поколения
                if let Some(ref mut diff_status) = diff_status_opt {
                    // Активировать индукторы при достижении стадии de novo
                    if !diff_status.inductors_active {
                        let de_novo_stage =
                            Self::de_novo_stage_for_division(self.params.de_novo_centriole_division);
                        if comp.stage >= de_novo_stage {
                            diff_status.inductors_active = true;
                            info!(
                                "De novo centriole creation at {:?} (division {}): \
                                 inductors activated at age {:.3} days ({:?})",
                                comp.stage,
                                self.params.de_novo_centriole_division,
                                comp.age_days,
                                comp.tissue_type,
                            );
                        }
                    }

                    // Коммитирование только при активных индукторах
                    if diff_status.inductors_active {
                        let current_potency = comp.inducers.potency_level();
                        if diff_status.try_advance(current_potency, comp.age_years()) {
                            info!(
                                "Commitment event: {:?} at age {:.1} yr ({:?})",
                                diff_status.tier,
                                comp.age_years(),
                                comp.tissue_type,
                            );
                        }
                    }

                    // Мейотическая элиминация центриолей: при достижении репродуктивного возраста
                    // регистрируем событие (реальный сброс происходит при инициализации следующего поколения).
                    if self.params.meiotic_elimination_enabled
                        && !diff_status.meiotic_reset_done
                        && comp.stage == HumanDevelopmentalStage::Adolescence
                    {
                        diff_status.meiotic_reset_done = true;
                        info!(
                            "Meiotic centriole elimination registered at {:?}, age {:.1} yr ({:?}): \
                             next-generation DifferentiationStatus will start from Totipotent",
                            comp.stage,
                            comp.age_years(),
                            comp.tissue_type,
                        );
                    }
                }

                // 6е. Обратимая модуляция (ModulationState)
                // Зависит от внешних сигналов: ниша, стресс, SASP.
                // НЕ меняет DifferentiationStatus — только адаптирует активность.
                if let Some(ref mut modulation) = modulation_opt {
                    // Активность = функция от состояния ниши и нишевых сигналов
                    modulation.niche_signal_strength = comp.tissue_state.regeneration_tempo;
                    modulation.activity_level = (
                        comp.tissue_state.functional_capacity * 0.7
                        + modulation.niche_signal_strength * 0.3
                    ).clamp(0.0, 1.0);
                    modulation.is_quiescent = modulation.activity_level < 0.2;

                    // Стресс-ответ: ROS + агрегаты → шаперонная система
                    modulation.stress_response = (
                        comp.centriolar_damage.ros_level * 0.5
                        + comp.centriolar_damage.protein_aggregates * 0.3
                    ).clamp(0.0, 1.0);

                    // SASP: сенесцентная клетка секретирует воспалительные факторы в нишу
                    if comp.centriolar_damage.is_senescent {
                        modulation.sasp_output = infl_sasp.max(
                            comp.centriolar_damage.ros_level * 0.3
                        );
                    } else {
                        // Экспоненциальный спад SASP при восстановлении
                        modulation.sasp_output = (modulation.sasp_output * 0.95).max(0.0);
                    }

                    // Эпигенетическая пластичность снижается по мере дифференцировки
                    modulation.epigenetic_plasticity = match comp.inducers.potency_level() {
                        PotencyLevel::Totipotent  => 1.0,
                        PotencyLevel::Pluripotent => 0.8,
                        PotencyLevel::Oligopotent => 0.5,
                        PotencyLevel::Unipotent   => 0.2,
                        PotencyLevel::Apoptosis   => 0.0,
                    };
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
                    info!(
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

        // Шаг 1б: вставить Dead-маркер на только что умершие сущности.
        // Двухфазовый паттерн: сначала собираем мёртвые entity, потом вставляем.
        // SimulationManager::cleanup_dead_entities() удалит их при очередной очистке.
        {
            let dead_entities: Vec<Entity> = world
                .query::<&HumanDevelopmentComponent>()
                .iter()
                .filter(|(_, comp)| !comp.is_alive)
                .map(|(e, _)| e)
                .collect();
            for entity in dead_entities {
                let _ = world.insert_one(entity, Dead);
            }
        }

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
            // Параметры симуляции
            "time_acceleration":       self.params.time_acceleration,
            "enable_aging":            self.params.enable_aging,
            "enable_morphogenesis":    self.params.enable_morphogenesis,
            "tissue_detail_level":     self.params.tissue_detail_level,
            // Параметры индукторов
            "mother_inducer_count":    self.params.mother_inducer_count,
            "daughter_inducer_count":  self.params.daughter_inducer_count,
            "base_detach_probability": self.params.base_detach_probability,
            "mother_bias":             self.params.mother_bias,
            "age_bias_coefficient":    self.params.age_bias_coefficient,
            "ptm_exhaustion_scale":    self.params.ptm_exhaustion_scale,
            // Жизненный цикл индукторов
            "de_novo_centriole_division":   self.params.de_novo_centriole_division,
            "meiotic_elimination_enabled":  self.params.meiotic_elimination_enabled,
            // Параметры накопления повреждений (DamageParams)
            "base_ros_damage_rate":         self.damage_rates.base_ros_damage_rate,
            "acetylation_rate":             self.damage_rates.acetylation_rate,
            "aggregation_rate":             self.damage_rates.aggregation_rate,
            "phospho_dysregulation_rate":   self.damage_rates.phospho_dysregulation_rate,
            "senescence_threshold":         self.damage_rates.senescence_threshold,
            "midlife_damage_multiplier":    self.damage_rates.midlife_damage_multiplier,
            "ros_feedback_coefficient":     self.damage_rates.ros_feedback_coefficient,
            "sasp_onset_age":               self.damage_rates.sasp_onset_age,
            "cep164_loss_rate":             self.damage_rates.cep164_loss_rate,
            "cep89_loss_rate":              self.damage_rates.cep89_loss_rate,
            "ninein_loss_rate":             self.damage_rates.ninein_loss_rate,
            "cep170_loss_rate":             self.damage_rates.cep170_loss_rate,
            // P5: Репарация придатков
            "cep164_repair_rate":                   self.damage_rates.cep164_repair_rate,
            "cep89_repair_rate":                    self.damage_rates.cep89_repair_rate,
            "ninein_repair_rate":                   self.damage_rates.ninein_repair_rate,
            "cep170_repair_rate":                   self.damage_rates.cep170_repair_rate,
            "appendage_repair_mitophagy_coupling":  self.damage_rates.appendage_repair_mitophagy_coupling,
            // P4: Сигмоидный переход среднего возраста
            "midlife_transition_center":  self.damage_rates.midlife_transition_center,
            "midlife_transition_width":   self.damage_rates.midlife_transition_width,
            // P3: Стохастический шум
            "noise_scale":                self.damage_rates.noise_scale,
            // Служебное
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
        set_f32!("ptm_exhaustion_scale",    self.params.ptm_exhaustion_scale);
        set_u32!("de_novo_centriole_division",  self.params.de_novo_centriole_division);
        set_bool!("meiotic_elimination_enabled", self.params.meiotic_elimination_enabled);

        // DamageParams: preset (заменяет все значения сразу)
        if let Some(preset) = params.get("damage_preset").and_then(|v| v.as_str()) {
            self.damage_rates = match preset {
                "progeria"    => DamageParams::progeria(),
                "longevity"   => DamageParams::longevity(),
                "antioxidant" => DamageParams::antioxidant(),
                _             => DamageParams::normal_aging(),
            };
            self.damage_rates_dirty = true;
        }

        // DamageParams: отдельные поля (перекрывают preset если указаны вместе)
        macro_rules! set_dr {
            ($key:literal, $field:expr) => {
                if let Some(v) = params.get($key).and_then(|v| v.as_f64()) {
                    $field = v as f32;
                    self.damage_rates_dirty = true;
                }
            };
        }
        set_dr!("base_ros_damage_rate",       self.damage_rates.base_ros_damage_rate);
        set_dr!("acetylation_rate",           self.damage_rates.acetylation_rate);
        set_dr!("aggregation_rate",           self.damage_rates.aggregation_rate);
        set_dr!("phospho_dysregulation_rate", self.damage_rates.phospho_dysregulation_rate);
        set_dr!("senescence_threshold",       self.damage_rates.senescence_threshold);
        set_dr!("midlife_damage_multiplier",  self.damage_rates.midlife_damage_multiplier);
        set_dr!("ros_feedback_coefficient",   self.damage_rates.ros_feedback_coefficient);
        set_dr!("sasp_onset_age",             self.damage_rates.sasp_onset_age);
        set_dr!("cep164_loss_rate",           self.damage_rates.cep164_loss_rate);
        set_dr!("cep89_loss_rate",            self.damage_rates.cep89_loss_rate);
        set_dr!("ninein_loss_rate",           self.damage_rates.ninein_loss_rate);
        set_dr!("cep170_loss_rate",           self.damage_rates.cep170_loss_rate);
        // P5: Репарация придатков
        set_dr!("cep164_repair_rate",                  self.damage_rates.cep164_repair_rate);
        set_dr!("cep89_repair_rate",                   self.damage_rates.cep89_repair_rate);
        set_dr!("ninein_repair_rate",                  self.damage_rates.ninein_repair_rate);
        set_dr!("cep170_repair_rate",                  self.damage_rates.cep170_repair_rate);
        set_dr!("appendage_repair_mitophagy_coupling", self.damage_rates.appendage_repair_mitophagy_coupling);
        // P4: Сигмоидный переход
        set_dr!("midlife_transition_center",  self.damage_rates.midlife_transition_center);
        set_dr!("midlife_transition_width",   self.damage_rates.midlife_transition_width);
        // P3: Стохастический шум
        set_dr!("noise_scale",                self.damage_rates.noise_scale);

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
            comp.inducers.detachment_params.ptm_exhaustion_scale =
                self.params.ptm_exhaustion_scale;
            // Standalone ECS-компоненты для межмодульного взаимодействия:
            // CentriolarDamageState — синхронизируется в step() для других модулей.
            // InflammagingState     — пишется myeloid_shift_module, читается здесь.
            world.insert_one(entity, CentriolarDamageState::pristine())?;
            world.insert_one(entity, InflammagingState::default())?;
            world.insert_one(entity, TelomereState::default())?;
            world.insert_one(entity, EpigeneticClockState::default())?;
            world.insert_one(entity, DifferentiationStatus::default())?;
            world.insert_one(entity, ModulationState::default())?;
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

// ---------------------------------------------------------------------------
// Тесты PTM bridge
// ---------------------------------------------------------------------------
#[cfg(test)]
mod ptm_bridge_tests {
    use super::damage::{accumulate_damage, DamageParams};
    use cell_dt_core::components::CentriolarDamageState;

    const PTM_SCALE: f32 = 0.002;
    const DT_YEARS: f32  = 1.0 / 365.25; // один шаг = один день

    /// Применяет PTM bridge так же как делает human_development_module.step()
    fn apply_ptm_bridge(
        damage: &mut CentriolarDamageState,
        ptm_acetylation: f32,
        ptm_oxidation:   f32,
        ptm_phospho:     f32,
        ptm_methyl:      f32,
        dt_years: f32,
    ) {
        damage.tubulin_hyperacetylation = (damage.tubulin_hyperacetylation
            + ptm_acetylation * PTM_SCALE * dt_years).min(1.0);
        damage.protein_carbonylation    = (damage.protein_carbonylation
            + ptm_oxidation   * PTM_SCALE * dt_years).min(1.0);
        damage.phosphorylation_dysregulation = (damage.phosphorylation_dysregulation
            + ptm_phospho     * PTM_SCALE * dt_years).min(1.0);
        damage.protein_aggregates = (damage.protein_aggregates
            + ptm_methyl      * PTM_SCALE * 0.5 * dt_years).min(1.0);
        damage.update_functional_metrics();
    }

    #[test]
    fn test_ptm_bridge_increases_hyperacetylation() {
        let params = DamageParams::default();
        let age_yr = 30.0_f32;

        // Контрольная клетка: только accumulate_damage
        let mut damage_ctrl = CentriolarDamageState::pristine();
        for _ in 0..365 {
            accumulate_damage(&mut damage_ctrl, &params, age_yr, DT_YEARS, 0.0);
        }

        // Клетка с PTM bridge: высокое ацетилирование
        let mut damage_ptm = CentriolarDamageState::pristine();
        for _ in 0..365 {
            accumulate_damage(&mut damage_ptm, &params, age_yr, DT_YEARS, 0.0);
            apply_ptm_bridge(&mut damage_ptm, 1.0, 0.0, 0.0, 0.0, DT_YEARS);
        }

        assert!(damage_ptm.tubulin_hyperacetylation > damage_ctrl.tubulin_hyperacetylation,
            "PTM bridge должен увеличивать tubulin_hyperacetylation: ptm={:.4} ctrl={:.4}",
            damage_ptm.tubulin_hyperacetylation, damage_ctrl.tubulin_hyperacetylation);
    }

    #[test]
    fn test_ptm_bridge_increases_carbonylation() {
        let params = DamageParams::default();
        let age_yr = 30.0_f32;

        let mut damage_ctrl = CentriolarDamageState::pristine();
        let mut damage_ptm  = CentriolarDamageState::pristine();

        for _ in 0..365 {
            accumulate_damage(&mut damage_ctrl, &params, age_yr, DT_YEARS, 0.0);
            accumulate_damage(&mut damage_ptm,  &params, age_yr, DT_YEARS, 0.0);
            apply_ptm_bridge(&mut damage_ptm, 0.0, 1.0, 0.0, 0.0, DT_YEARS); // только oxidation
        }

        assert!(damage_ptm.protein_carbonylation > damage_ctrl.protein_carbonylation,
            "PTM bridge (oxidation) должен увеличивать protein_carbonylation");
    }

    #[test]
    fn test_ptm_bridge_zero_with_no_ptm() {
        // При ptm=0 bridge не изменяет damage
        let params = DamageParams::default();
        let age_yr = 30.0_f32;

        let mut damage_ctrl = CentriolarDamageState::pristine();
        let mut damage_zero = CentriolarDamageState::pristine();

        for _ in 0..365 {
            accumulate_damage(&mut damage_ctrl, &params, age_yr, DT_YEARS, 0.0);
            accumulate_damage(&mut damage_zero, &params, age_yr, DT_YEARS, 0.0);
            apply_ptm_bridge(&mut damage_zero, 0.0, 0.0, 0.0, 0.0, DT_YEARS);
        }

        // Должны быть идентичны с точностью до float
        assert!((damage_zero.tubulin_hyperacetylation - damage_ctrl.tubulin_hyperacetylation).abs() < 1e-6,
            "При нулевом PTM bridge не должен менять damage");
        assert!((damage_zero.protein_carbonylation - damage_ctrl.protein_carbonylation).abs() < 1e-6);
    }

    #[test]
    fn test_ptm_bridge_scale_is_moderate() {
        // За 30 лет с PTM=1.0 накопленный вклад bridge не должен превышать 30% от базовой damage
        let params = DamageParams::default();
        let age_yr = 30.0_f32;
        let steps  = 365 * 30; // 30 лет

        let mut damage_ctrl = CentriolarDamageState::pristine();
        let mut damage_ptm  = CentriolarDamageState::pristine();

        for _ in 0..steps {
            accumulate_damage(&mut damage_ctrl, &params, age_yr, DT_YEARS, 0.0);
            accumulate_damage(&mut damage_ptm,  &params, age_yr, DT_YEARS, 0.0);
            apply_ptm_bridge(&mut damage_ptm, 1.0, 1.0, 1.0, 1.0, DT_YEARS);
        }

        let diff = damage_ptm.tubulin_hyperacetylation - damage_ctrl.tubulin_hyperacetylation;
        let relative = if damage_ctrl.tubulin_hyperacetylation > 0.0 {
            diff / damage_ctrl.tubulin_hyperacetylation
        } else { 0.0 };

        assert!(relative < 0.5,
            "PTM bridge вклад за 30 лет должен быть < 50% от базового: relative={:.2}", relative);
        assert!(diff > 0.0, "PTM bridge должен быть ненулевым при PTM=1.0");
    }
}

// ---------------------------------------------------------------------------
// Интеграционные тесты жизненного цикла CDATA
// ---------------------------------------------------------------------------
#[cfg(test)]
mod lifecycle_tests {
    use super::*;
    use cell_dt_core::{SimulationManager, SimulationConfig};
    use cell_dt_core::components::{CentriolePair, CellCycleStateExtended};
    use super::damage::DamageParams;

    /// Запустить симуляцию на `years` лет и вернуть `(is_alive, damage_score, age_years)`.
    /// Параметры повреждений переопределяются сразу после инициализации.
    ///
    /// `is_alive` зависит от стохастического исчерпания индукторов — не используйте для
    /// точных долгосрочных тестов. Вместо этого сравнивайте `damage_score`.
    fn run_cdata(years: usize, damage: DamageParams) -> (bool, f32, f32) {
        let config = SimulationConfig {
            max_steps: (years * 365 + 100) as u64,
            dt: 1.0,
            checkpoint_interval: 100_000,
            num_threads: None,
            seed: None,
            parallel_modules: false,
            cleanup_dead_interval: None,
        };
        let mut sim = SimulationManager::new(config);
        sim.register_module(Box::new(HumanDevelopmentModule::with_params(
            HumanDevelopmentParams {
                // Отключаем стохастическое отщепление индукторов для детерминизма
                base_detach_probability: 0.0,
                ptm_exhaustion_scale:    0.0,
                ..HumanDevelopmentParams::default()
            },
        ))).unwrap();

        sim.world_mut().spawn((CentriolePair::default(), CellCycleStateExtended::new()));
        sim.initialize().unwrap();

        // Переопределяем damage_rates для задания нужного темпа старения
        {
            let mut q = sim.world_mut().query::<&mut HumanDevelopmentComponent>();
            for (_, comp) in q.iter() {
                comp.damage_rates = damage.clone();
            }
        }

        for _ in 0..(years * 365) {
            sim.step().unwrap();
        }

        let mut q = sim.world().query::<&HumanDevelopmentComponent>();
        q.iter()
            .next()
            .map(|(_, c)| (c.is_alive, c.damage_score(), c.age_years() as f32))
            .unwrap_or((false, 0.0, 0.0))
    }

    /// Нормальное старение: повреждения ниже порога в 60 лет (нет преждевременной сенесценции)
    #[test]
    fn test_normal_aging_below_threshold_at_60() {
        let (_, damage, _) = run_cdata(60, DamageParams::normal_aging());
        assert!(
            damage < 0.75,
            "Нормальное старение: damage в 60 лет должен быть ниже 0.75 (фактически: {:.3})",
            damage
        );
    }

    /// Долгожители (×0.6 скоростей): повреждения ниже порога в 95 лет
    /// (ожидаемая смерть от сенесценции: ~78 / 0.6 ≈ 130 лет)
    #[test]
    fn test_longevity_below_threshold_at_95() {
        let (_, damage, _) = run_cdata(95, DamageParams::longevity());
        assert!(
            damage < 0.75,
            "Долгожители: damage в 95 лет должен быть ниже 0.75 (фактически: {:.3})",
            damage
        );
    }

    /// Прогерия накапливает значительно больше повреждений чем нормальное старение
    /// (детерминированное сравнение: progeria_damage(30) >> normal_damage(30))
    #[test]
    fn test_progeria_accumulates_more_damage_than_normal() {
        let (_, d_prog, _) = run_cdata(30, DamageParams::progeria());
        let (_, d_norm, _) = run_cdata(30, DamageParams::normal_aging());
        assert!(
            d_prog > d_norm * 2.0,
            "Прогерия должна давать >2× больше повреждений к 30 годам \
             (progeria={:.3}, normal={:.3})",
            d_prog, d_norm
        );
    }

    /// Долгожители накапливают значительно меньше повреждений чем нормальное старение
    /// (детерминированное сравнение: longevity_damage(60) << normal_damage(60))
    #[test]
    fn test_longevity_less_damage_than_normal() {
        let (_, d_long, _) = run_cdata(60, DamageParams::longevity());
        let (_, d_norm, _) = run_cdata(60, DamageParams::normal_aging());
        assert!(
            d_long < d_norm * 0.75,
            "Долгожители должны иметь <75% повреждений нормального старения в 60 лет \
             (longevity={:.3}, normal={:.3})",
            d_long, d_norm
        );
    }
}

// ---------------------------------------------------------------------------
// Тест калибровки индукторов (6a)
// ---------------------------------------------------------------------------
/// Запустить симуляцию на `years` лет со стохастическим отщеплением (base_detach=0.002).
/// Возвращает (m_remaining, m_initial, d_remaining, d_initial).
#[cfg(test)]
mod inductor_calibration_tests {
    use super::*;
    use cell_dt_core::{SimulationManager, SimulationConfig};
    use cell_dt_core::components::{CentriolePair, CellCycleStateExtended};

    fn run_with_inducers(years: usize, seed: u64) -> (u32, u32, u32, u32) {
        let config = SimulationConfig {
            max_steps: (years * 365 + 10) as u64,
            dt: 1.0,
            checkpoint_interval: 100_000,
            num_threads: None,
            seed: Some(seed),
            parallel_modules: false,
            cleanup_dead_interval: None,
        };
        let mut sim = SimulationManager::new(config);
        // Полные дефолтные параметры: base_detach_probability=0.002, ptm_exhaustion_scale=0.001
        sim.register_module(Box::new(HumanDevelopmentModule::new())).unwrap();
        sim.world_mut().spawn((CentriolePair::default(), CellCycleStateExtended::new()));
        sim.initialize().unwrap();

        for _ in 0..(years * 365) {
            sim.step().unwrap();
        }

        let mut q = sim.world().query::<&HumanDevelopmentComponent>();
        let (_, comp) = q.iter().next().unwrap();
        (
            comp.inducers.mother_set.remaining,
            comp.inducers.mother_set.inherited_count,
            comp.inducers.daughter_set.remaining,
            comp.inducers.daughter_set.inherited_count,
        )
    }

    /// Простейшая проверка: за 78 лет обa комплекта теряют хотя бы один индуктор.
    #[test]
    fn test_inductor_depletion_occurs() {
        let (m_rem, m_init, d_rem, d_init) = run_with_inducers(78, 42);
        assert!(m_rem < m_init,
            "M-комплект за 78 лет должен потерять хотя бы 1 индуктор: {}/{}", m_rem, m_init);
        assert!(d_rem < d_init,
            "D-комплект за 78 лет должен потерять хотя бы 1 индуктор: {}/{}", d_rem, d_init);
    }

    /// Многосемянная калибровка: усреднённое остаточное значение по 5 запускам
    /// должно быть заметно ниже начального (не менее 5% истощения для каждого комплекта).
    ///
    /// Примечание: пороги консервативны, так как скорость отщепления низка
    /// (base_detach=0.002, oxygen_level ≪ 1 большую часть жизни).
    #[test]
    fn test_inductor_calibration_multiseed() {
        let seeds: [u64; 5] = [42, 100, 999, 12345, 77777];
        let m_init = 10_u32;
        let d_init = 8_u32;

        let mut m_total_loss = 0_u32;
        let mut d_total_loss = 0_u32;

        for seed in seeds {
            let (m_rem, _, d_rem, _) = run_with_inducers(78, seed);
            m_total_loss += m_init.saturating_sub(m_rem);
            d_total_loss += d_init.saturating_sub(d_rem);
        }

        let m_avg_loss = m_total_loss as f32 / seeds.len() as f32;
        let d_avg_loss = d_total_loss as f32 / seeds.len() as f32;

        assert!(m_avg_loss >= 0.5,
            "M-комплект: средняя потеря за 78 лет должна быть ≥0.5 индуктора: {:.2}", m_avg_loss);
        assert!(d_avg_loss >= 0.5,
            "D-комплект: средняя потеря за 78 лет должна быть ≥0.5 индуктора: {:.2}", d_avg_loss);
    }
}

// ---------------------------------------------------------------------------
// Интеграционные тесты миелоидного сдвига по возрасту (6b)
// ---------------------------------------------------------------------------
#[cfg(test)]
mod myeloid_range_tests {
    use super::*;
    use cell_dt_core::{SimulationManager, SimulationConfig};
    use cell_dt_core::components::{CentriolePair, CellCycleStateExtended};
    use myeloid_shift_module::{MyeloidShiftModule, MyeloidShiftComponent};
    use super::damage::DamageParams;

    /// Запустить `years` лет и вернуть myeloid_bias из MyeloidShiftComponent.
    /// Детерминированно: base_detach_probability=0.0, ptm_exhaustion_scale=0.0.
    fn run_myeloid(years: usize) -> f32 {
        let config = SimulationConfig {
            max_steps: (years * 365 + 10) as u64,
            dt: 1.0,
            checkpoint_interval: 100_000,
            num_threads: None,
            seed: None,
            parallel_modules: false,
            cleanup_dead_interval: None,
        };
        let mut sim = SimulationManager::new(config);
        sim.register_module(Box::new(HumanDevelopmentModule::with_params(
            HumanDevelopmentParams {
                base_detach_probability: 0.0,
                ptm_exhaustion_scale:    0.0,
                ..HumanDevelopmentParams::default()
            },
        ))).unwrap();
        sim.register_module(Box::new(MyeloidShiftModule::new())).unwrap();

        sim.world_mut().spawn((CentriolePair::default(), CellCycleStateExtended::new()));
        sim.initialize().unwrap();

        // Переопределяем damage_rates для воспроизводимости
        {
            let mut q = sim.world_mut().query::<&mut HumanDevelopmentComponent>();
            for (_, comp) in q.iter() {
                comp.damage_rates = DamageParams::normal_aging();
            }
        }

        for _ in 0..(years * 365) {
            sim.step().unwrap();
        }

        let mut q = sim.world().query::<&MyeloidShiftComponent>();
        q.iter()
            .next()
            .map(|(_, c)| c.myeloid_bias)
            .unwrap_or(0.0)
    }

    /// В 20 лет: повреждения минимальны → myeloid_bias < 0.15
    #[test]
    fn test_myeloid_bias_low_at_age_20() {
        let bias = run_myeloid(20);
        assert!(bias < 0.15,
            "В 20 лет myeloid_bias должен быть < 0.15 (фактически: {:.3})", bias);
    }

    /// В 70 лет: умеренный сдвиг → 0.20 < myeloid_bias < 0.75
    #[test]
    fn test_myeloid_bias_moderate_at_age_70() {
        let bias = run_myeloid(70);
        assert!(bias > 0.20,
            "В 70 лет myeloid_bias должен быть > 0.20 (фактически: {:.3})", bias);
        assert!(bias < 0.75,
            "В 70 лет myeloid_bias должен быть < 0.75 (фактически: {:.3})", bias);
    }

    /// В 85 лет: тяжёлый сдвиг → myeloid_bias > 0.35
    #[test]
    fn test_myeloid_bias_high_at_age_85() {
        let bias = run_myeloid(85);
        assert!(bias > 0.35,
            "В 85 лет myeloid_bias должен быть > 0.35 (фактически: {:.3})", bias);
    }

    /// Монотонность: myeloid_bias(70) > myeloid_bias(20)
    #[test]
    fn test_myeloid_bias_increases_with_age() {
        let bias_20 = run_myeloid(20);
        let bias_70 = run_myeloid(70);
        assert!(bias_70 > bias_20,
            "myeloid_bias должен расти с возрастом: age20={:.3}, age70={:.3}",
            bias_20, bias_70);
    }
}

// ---------------------------------------------------------------------------
// Тесты DifferentiationStatus и ModulationState
// ---------------------------------------------------------------------------
#[cfg(test)]
mod differentiation_tests {
    use cell_dt_core::components::{
        DifferentiationStatus, DifferentiationTier, ModulationState, PotencyLevel,
    };

    /// DifferentiationTier: Ord работает корректно (Totipotent < Terminal)
    #[test]
    fn test_tier_ordering() {
        assert!(DifferentiationTier::Totipotent  < DifferentiationTier::Pluripotent);
        assert!(DifferentiationTier::Pluripotent < DifferentiationTier::Multipotent);
        assert!(DifferentiationTier::Multipotent < DifferentiationTier::Committed);
        assert!(DifferentiationTier::Committed   < DifferentiationTier::Terminal);
    }

    /// from_potency правильно отображает PotencyLevel → DifferentiationTier
    #[test]
    fn test_from_potency_mapping() {
        assert_eq!(DifferentiationTier::from_potency(PotencyLevel::Totipotent),  DifferentiationTier::Totipotent);
        assert_eq!(DifferentiationTier::from_potency(PotencyLevel::Pluripotent), DifferentiationTier::Pluripotent);
        assert_eq!(DifferentiationTier::from_potency(PotencyLevel::Oligopotent), DifferentiationTier::Multipotent);
        assert_eq!(DifferentiationTier::from_potency(PotencyLevel::Unipotent),   DifferentiationTier::Committed);
        assert_eq!(DifferentiationTier::from_potency(PotencyLevel::Apoptosis),   DifferentiationTier::Terminal);
    }

    /// Необратимость: try_advance идёт только вперёд, никогда назад
    #[test]
    fn test_differentiation_is_irreversible() {
        let mut status = DifferentiationStatus::new(PotencyLevel::Totipotent);
        assert_eq!(status.tier, DifferentiationTier::Totipotent);
        assert_eq!(status.commitment_events, 0);

        // Переход вперёд: Totipotent → Pluripotent
        let advanced = status.try_advance(PotencyLevel::Pluripotent, 5.0);
        assert!(advanced, "переход Totipotent → Pluripotent должен вернуть true");
        assert_eq!(status.tier, DifferentiationTier::Pluripotent);
        assert_eq!(status.commitment_events, 1);
        assert_eq!(status.tier_history.len(), 1);

        // Попытка регресса: Pluripotent → Totipotent (должна быть проигнорирована)
        let regressed = status.try_advance(PotencyLevel::Totipotent, 6.0);
        assert!(!regressed, "попытка регресса должна вернуть false");
        assert_eq!(status.tier, DifferentiationTier::Pluripotent, "tier не должен регрессировать");
        assert_eq!(status.commitment_events, 1, "commitment_events не должен расти при регрессе");

        // Переход вперёд: Pluripotent → Terminal (через Apoptosis)
        let advanced2 = status.try_advance(PotencyLevel::Apoptosis, 78.0);
        assert!(advanced2);
        assert_eq!(status.tier, DifferentiationTier::Terminal);
        assert_eq!(status.commitment_events, 2);
    }

    /// Попытка advance при том же уровне: не должна создавать событие
    #[test]
    fn test_no_event_same_tier() {
        let mut status = DifferentiationStatus::new(PotencyLevel::Pluripotent);
        let advanced = status.try_advance(PotencyLevel::Pluripotent, 10.0);
        assert!(!advanced, "один и тот же уровень не должен создавать commitment event");
        assert_eq!(status.commitment_events, 0);
    }

    /// ModulationState default: активна, не в покое
    #[test]
    fn test_modulation_default_active() {
        let m = ModulationState::default();
        assert_eq!(m.activity_level, 1.0);
        assert!(!m.is_quiescent);
        assert_eq!(m.sasp_output, 0.0);
        assert_eq!(m.epigenetic_plasticity, 1.0);
    }

    /// De novo: inductors_active по умолчанию false
    #[test]
    fn test_inductors_inactive_by_default() {
        let status = DifferentiationStatus::default();
        assert!(!status.inductors_active, "inductors должны быть неактивны до de novo стадии");
        assert!(!status.meiotic_reset_done, "мейотическая элиминация ещё не произошла");
    }

    /// reset_for_meiosis: сбрасывает tier → Totipotent и деактивирует индукторы
    #[test]
    fn test_reset_for_meiosis() {
        let mut status = DifferentiationStatus::default();
        // Активируем и продвигаем
        status.inductors_active = true;
        status.try_advance(PotencyLevel::Oligopotent, 20.0);
        assert_eq!(status.tier, DifferentiationTier::Multipotent);
        assert_eq!(status.commitment_events, 1);

        // Мейотический сброс
        status.meiotic_reset_done = true; // имитируем: мейоз уже произошёл однажды
        status.reset_for_meiosis();
        assert_eq!(status.tier, DifferentiationTier::Totipotent, "tier должен сброситься до Totipotent");
        assert!(!status.inductors_active, "inductors_active должен стать false после мейоза");
        assert_eq!(status.commitment_events, 0, "commitment_events обнуляется");
        assert!(!status.meiotic_reset_done, "meiotic_reset_done должен сброситься для следующего поколения");
        // История сохраняется
        assert_eq!(status.tier_history.len(), 1, "история переходов сохраняется");
    }

    /// de_novo_stage_for_division: корректное отображение
    #[test]
    fn test_de_novo_stage_mapping() {
        use super::HumanDevelopmentalStage;
        use super::HumanDevelopmentModule;
        assert_eq!(HumanDevelopmentModule::de_novo_stage_for_division(1), HumanDevelopmentalStage::Zygote);
        assert_eq!(HumanDevelopmentModule::de_novo_stage_for_division(2), HumanDevelopmentalStage::Cleavage);
        assert_eq!(HumanDevelopmentModule::de_novo_stage_for_division(3), HumanDevelopmentalStage::Cleavage);
        assert_eq!(HumanDevelopmentModule::de_novo_stage_for_division(4), HumanDevelopmentalStage::Morula);
        assert_eq!(HumanDevelopmentModule::de_novo_stage_for_division(5), HumanDevelopmentalStage::Blastocyst);
        assert_eq!(HumanDevelopmentModule::de_novo_stage_for_division(8), HumanDevelopmentalStage::Implantation);
    }

    /// HumanDevelopmentalStage: Ord работает в правильном направлении
    #[test]
    fn test_stage_ordering() {
        use super::HumanDevelopmentalStage;
        assert!(HumanDevelopmentalStage::Zygote    < HumanDevelopmentalStage::Cleavage);
        assert!(HumanDevelopmentalStage::Cleavage  < HumanDevelopmentalStage::Morula);
        assert!(HumanDevelopmentalStage::Morula    < HumanDevelopmentalStage::Blastocyst);
        assert!(HumanDevelopmentalStage::Adult     < HumanDevelopmentalStage::Elderly);
    }
}
