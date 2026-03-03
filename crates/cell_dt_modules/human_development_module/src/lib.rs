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
        DivisionExhaustionState,
        CentriolePair,
        TelomereState,
        EpigeneticClockState,
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
    centrosomal_oxygen_level, detach_by_oxygen, detach_by_ptm_exhaustion,
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
    /// Масштаб PTM-опосредованного истощения материнского комплекта [0..0.01].
    /// 0.0 → механизм выключен; 0.001 → умеренное PTM-истощение.
    pub ptm_exhaustion_scale: f32,
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
        comp.tissue_state.update_functional_capacity();
    }

    /// O₂-зависимое отщепление индукторов (контролируемый путь, одинаковый для M и D).
    fn apply_oxygen_detachment(comp: &mut HumanDevelopmentComponent, rng: &mut impl Rng) {
        let oxygen = centrosomal_oxygen_level(&comp.centriolar_damage);
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
        let mut query = world.query::<(
            &mut HumanDevelopmentComponent,
            Option<&InflammagingState>,
            Option<&DivisionExhaustionState>,
            Option<&CentriolePair>,
            Option<&mut TelomereState>,
            Option<&mut EpigeneticClockState>,
        )>();

        for (_, (comp, inflammaging_opt, exhaustion_opt, centriole_opt, mut telomere_opt, mut epigenetic_opt)) in query.iter() {
            if !comp.is_alive { continue; }

            // Предварительно извлекаем значения из InflammagingState (если модуль активен)
            let infl_ros_boost        = inflammaging_opt.map_or(0.0, |i| i.ros_boost);
            let infl_niche_impairment = inflammaging_opt.map_or(0.0, |i| i.niche_impairment);
            let infl_sasp             = inflammaging_opt.map_or(0.0, |i| i.sasp_intensity);
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
                // 3. Молекулярные повреждения (5 типов + ROS-петля)
                let age_years = comp.age_years() as f32;
                accumulate_damage(
                    &mut comp.centriolar_damage,
                    &comp.damage_rates,
                    age_years,
                    dt_years,
                );

                // 3б. PTM bridge: структурные PTM CentriolePair → функциональные повреждения.
                // Масштаб 0.002/год при PTM=1.0 → ~33% от базовой скорости накопления.
                // Лаг один шаг (centriole_module запускается до human_development_module).
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

                // 3в. Inflammaging-буст ROS (петля от миелоидного сдвига).
                // Применяем после accumulate_damage, т.к. та перезаписывает ros_level.
                // Лаг в один шаг допустим: myeloid_shift_module запускается следом.
                if infl_ros_boost > 0.0 {
                    comp.centriolar_damage.ros_level =
                        (comp.centriolar_damage.ros_level * (1.0 + infl_ros_boost)).min(1.0);
                    // Пересчитать производные метрики (spindle_fidelity, ciliary_function)
                    comp.centriolar_damage.update_functional_metrics();
                }

                // 4. O₂-зависимое отщепление индукторов (контролируемый путь, M=D=0.5)
                Self::apply_oxygen_detachment(comp, &mut rng);

                // 4б. PTM-опосредованное истощение (только мать — механизм истощения пула).
                // Независим от O₂: структурные ПТМ матери ослабляют связи индукторов.
                // Срабатывает только при наличии PTM-асимметрии мать > дочь.
                if ptm_asymmetry > 0.01 {
                    Self::apply_ptm_exhaustion(comp, ptm_asymmetry, &mut rng);
                }

                // 5. Тканевое состояние (Трек A + Трек B)
                Self::update_tissue_state(comp);

                // 5б. Niche impairment от воспаления (снижает темп регенерации)
                if infl_niche_impairment > 0.0 {
                    comp.tissue_state.regeneration_tempo =
                        (comp.tissue_state.regeneration_tempo
                            * (1.0 - infl_niche_impairment)).max(0.0);
                    comp.tissue_state.update_functional_capacity();
                }

                // 5в. Истощение пула из-за симметричных дифф. делений
                // exhaustion_ratio → уменьшает stem_cell_pool на 0.0002/шаг × ratio
                // Скорость 0.0002 мала: заметный эффект накапливается за годы активного деления
                if exhaustion_ratio > 0.0 {
                    const POOL_DEPLETION_RATE: f32 = 0.0002;
                    comp.tissue_state.stem_cell_pool = (comp.tissue_state.stem_cell_pool
                        - exhaustion_ratio * POOL_DEPLETION_RATE * dt_years as f32).max(0.0);
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

                // 6в. Трек C: укорачивание теломер
                // Скорость = shortening_per_division × division_rate × spindle_factor × ros_factor
                if let Some(ref mut tel) = telomere_opt {
                    let div_rate: f32 = match comp.stage {
                        HumanDevelopmentalStage::Zygote
                        | HumanDevelopmentalStage::Cleavage     => 365.0 * 2.0,
                        HumanDevelopmentalStage::Morula
                        | HumanDevelopmentalStage::Blastocyst   => 365.0 * 1.0,
                        HumanDevelopmentalStage::Implantation
                        | HumanDevelopmentalStage::Gastrulation => 365.0 * 0.5,
                        HumanDevelopmentalStage::Neurulation
                        | HumanDevelopmentalStage::Organogenesis => 365.0 * 0.3,
                        HumanDevelopmentalStage::Fetal           => 52.0,
                        HumanDevelopmentalStage::Newborn
                        | HumanDevelopmentalStage::Childhood     => 24.0,
                        HumanDevelopmentalStage::Adolescence
                        | HumanDevelopmentalStage::Adult         => 12.0,
                        HumanDevelopmentalStage::MiddleAge       => 6.0,
                        HumanDevelopmentalStage::Elderly         => 2.0,
                    };
                    let base = tel.shortening_per_division * div_rate * dt_years;
                    let spindle_f = 1.0 + (1.0 - comp.centriolar_damage.spindle_fidelity) * 0.5;
                    let ros_f    = 1.0 + comp.centriolar_damage.ros_level * 0.3;
                    tel.mean_length = (tel.mean_length - base * spindle_f * ros_f).max(0.0);
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
                if let Some(ref mut epi) = epigenetic_opt {
                    let damage = comp.centriolar_damage.total_damage_score();
                    epi.clock_acceleration = 1.0 + damage * 0.5;
                    epi.methylation_age += dt_years * epi.clock_acceleration;
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
            "ptm_exhaustion_scale":    self.params.ptm_exhaustion_scale,
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
            accumulate_damage(&mut damage_ctrl, &params, age_yr, DT_YEARS);
        }

        // Клетка с PTM bridge: высокое ацетилирование
        let mut damage_ptm = CentriolarDamageState::pristine();
        for _ in 0..365 {
            accumulate_damage(&mut damage_ptm, &params, age_yr, DT_YEARS);
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
            accumulate_damage(&mut damage_ctrl, &params, age_yr, DT_YEARS);
            accumulate_damage(&mut damage_ptm,  &params, age_yr, DT_YEARS);
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
            accumulate_damage(&mut damage_ctrl, &params, age_yr, DT_YEARS);
            accumulate_damage(&mut damage_zero, &params, age_yr, DT_YEARS);
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
            accumulate_damage(&mut damage_ctrl, &params, age_yr, DT_YEARS);
            accumulate_damage(&mut damage_ptm,  &params, age_yr, DT_YEARS);
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
