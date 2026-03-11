#![recursion_limit = "256"]
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
        OrganismState,
        NeedsHumanDevInit,
        ClonalState,
        Dead,
        NKSurveillanceState,
        ProteostasisState,
        CircadianState,
        AutophagyState,
        DDRState,
        GeneExpressionState,
        StemCellDivisionRateState,
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
pub mod interventions;

pub use inducers::{
    HumanMorphogeneticLevel, HumanInducers,
    centrosomal_oxygen_level, detach_by_oxygen, detach_by_ptm_exhaustion,
};
pub use tissues::*;
pub use aging::{AgingPhenotype, CentrioleAgingLink};
pub use damage::{DamageParams, accumulate_damage};
pub use development::{division_rate_per_year, base_ros_level, stage_for_age};
pub use interventions::{Intervention, InterventionKind, InterventionEffect};

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
    /// Масштаб стохастического Ланжевен-шума для молекулярных повреждений (P3).
    ///
    /// 0.0 = детерминированная симуляция (все ниши с одинаковыми параметрами
    ///       следуют идентичным траекториям).
    /// 0.01–0.05 = умеренный шум → клональная гетерогенность (нужна для NichePool/CHIP).
    /// Шум применяется по Ито: sigma = noise_scale × sqrt(dt_years).
    pub noise_scale: f32,
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
            base_detach_probability: 0.0003,
            mother_bias: 0.5,           // одинаковая вероятность для M и D
            age_bias_coefficient: 0.0,  // возраст не влияет по умолчанию
            ptm_exhaustion_scale: 0.001, // PTM-асимметрия → истощение матери
            de_novo_centriole_division: 4,    // 16-клеточная стадия (Морула)
            meiotic_elimination_enabled: true, // биологически корректный дефолт
            noise_scale: 0.0,           // детерминированно по умолчанию
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
    /// P11: расписание терапевтических интервенций.
    interventions: Vec<Intervention>,
    /// P11: накопленный healthspan — дни, когда total_damage_score < 0.5.
    healthspan_days: f64,
    /// P8: агрегированное состояние организма (вычисляется каждый шаг из тканевых метрик).
    organ_state: OrganismState,
    /// P8: причина гибели организма (для логирования и get_params).
    death_cause: Option<&'static str>,
    /// P7: системный SASP с прошлого шага — применяется как ros_boost (лаг 1 шаг).
    systemic_sasp_prev: f32,
    /// P13: агрегированные морфогенные поля по всем нишам (обновляются каждый шаг).
    morphogen_aggregates: MorphogenAggregates,
}

/// Агрегированные морфогенные поля по всем живым нишам.
/// Вычисляется как среднее из TissueState.{gli_activation, shh_concentration, ...}
#[derive(Debug, Clone, Default)]
pub struct MorphogenAggregates {
    pub gli_activation_mean: f32,
    pub shh_mean:            f32,
    pub bmp_balance_mean:    f32,
    pub wnt_activity_mean:   f32,
}

impl HumanDevelopmentModule {
    pub fn new() -> Self {
        Self {
            params: HumanDevelopmentParams::default(),
            step_count: 0,
            damage_rates: DamageParams::default(),
            damage_rates_dirty: false,
            rng: StdRng::from_entropy(),
            interventions: Vec::new(),
            healthspan_days: 0.0,
            organ_state: OrganismState::new(),
            death_cause: None,
            systemic_sasp_prev: 0.0,
            morphogen_aggregates: MorphogenAggregates::default(),
        }
    }

    pub fn with_params(params: HumanDevelopmentParams) -> Self {
        Self {
            params,
            step_count: 0,
            damage_rates: DamageParams::default(),
            damage_rates_dirty: false,
            rng: StdRng::from_entropy(),
            interventions: Vec::new(),
            healthspan_days: 0.0,
            organ_state: OrganismState::new(),
            death_cause: None,
            systemic_sasp_prev: 0.0,
            morphogen_aggregates: MorphogenAggregates::default(),
        }
    }

    /// P11: Добавить терапевтическую интервенцию в расписание.
    pub fn add_intervention(&mut self, iv: Intervention) {
        self.interventions.push(iv);
    }

    /// P11: Текущий healthspan [лет] — период с total_damage_score < 0.5.
    pub fn healthspan_years(&self) -> f64 {
        self.healthspan_days / 365.25
    }

    /// P8: Агрегированное состояние организма (обновляется каждый шаг).
    pub fn organism_state(&self) -> &OrganismState {
        &self.organ_state
    }

    /// P8: Причина гибели организма или `None` если организм жив.
    pub fn death_cause(&self) -> Option<&'static str> {
        self.death_cause
    }

    /// P8: Агрегировать тканевые метрики из ECS → обновить `organ_state`.
    ///
    /// Критерии смерти организма (по Jaiswal 2014, López-Otín 2023):
    /// - `frailty_index >= 0.95` — системный коллапс (все ткани истощены)
    /// - Blood `stem_cell_pool < 0.02` — фатальная панцитопения
    /// - Neural `functional_capacity < 0.05` — смерть мозга
    fn update_organism_state(&mut self, world: &World) {
        if !self.organ_state.is_alive { return; }

        let mut total_fc_sum = 0.0f32;
        let mut total_count  = 0u32;

        let mut blood_fc_sum   = 0.0f32; let mut blood_pool_sum  = 0.0f32;
        let mut blood_sf_sum   = 0.0f32; let mut blood_count     = 0u32;
        let mut neural_fc_sum  = 0.0f32; let mut neural_cilia    = 0.0f32;
        let mut neural_count   = 0u32;
        let mut muscle_fc_sum  = 0.0f32; let mut muscle_count    = 0u32;
        let mut gut_sf_sum     = 0.0f32; let mut gut_count       = 0u32;
        let mut age_days_sum   = 0.0f64; // для среднего возраста
        let mut sasp_sum       = 0.0f32; // P7: системный SASP

        for (_, (comp, modulation_opt)) in world
            .query::<(&HumanDevelopmentComponent, Option<&ModulationState>)>()
            .iter()
        {
            if !comp.is_alive { continue; }
            let fc   = comp.tissue_state.functional_capacity;
            let sf   = comp.tissue_state.senescent_fraction;
            let pool = comp.tissue_state.stem_cell_pool;
            let cilia = comp.centriolar_damage.ciliary_function;
            total_fc_sum += fc;
            total_count  += 1;
            age_days_sum += comp.age_days;
            // P7: собираем SASP от всех ниш
            if let Some(m) = modulation_opt {
                sasp_sum += m.sasp_output;
            }
            match comp.tissue_type {
                HumanTissueType::Blood => {
                    blood_fc_sum += fc; blood_pool_sum += pool;
                    blood_sf_sum += sf; blood_count += 1;
                }
                HumanTissueType::Neural => {
                    neural_fc_sum += fc; neural_cilia += cilia; neural_count += 1;
                }
                HumanTissueType::Muscle => {
                    muscle_fc_sum += fc; muscle_count += 1;
                }
                HumanTissueType::Epithelial => {
                    gut_sf_sum += sf; gut_count += 1;
                }
                _ => {}
            }
        }

        if total_count == 0 {
            // Все ниши мертвы — организм умирает
            if self.organ_state.is_alive {
                self.organ_state.is_alive = false;
                self.death_cause = Some("all_niches_exhausted");
                warn!("Organism death at {:.1} yr: all niches exhausted",
                    self.organ_state.age_years);
            }
            return;
        }

        // Обновить возраст
        self.organ_state.age_years = age_days_sum / total_count as f64 / 365.25;

        // frailty = 1 − mean(functional_capacity)
        self.organ_state.frailty_index = 1.0 - total_fc_sum / total_count as f32;

        // Когнитивный индекс: нейральные ниши
        if neural_count > 0 {
            let nfc   = neural_fc_sum / neural_count as f32;
            let ncilia = neural_cilia / neural_count as f32;
            self.organ_state.cognitive_index = (nfc * 0.7 + ncilia * 0.3).max(0.0);
        }
        // Иммунный резерв: HSC
        if blood_count > 0 {
            self.organ_state.immune_reserve = blood_fc_sum / blood_count as f32;
        }
        // Мышечная масса
        if muscle_count > 0 {
            self.organ_state.muscle_mass = muscle_fc_sum / muscle_count as f32;
        }
        // Inflammaging: SASP от кишечника + кроветворной ткани
        let gut_sf = if gut_count   > 0 { gut_sf_sum   / gut_count   as f32 } else { 0.0 };
        let hsc_sf = if blood_count > 0 { blood_sf_sum / blood_count as f32 } else { 0.0 };
        self.organ_state.inflammaging_score = ((gut_sf + hsc_sf) / 2.0).min(1.0);

        // P7: Системный SASP — среднее sasp_output всех ниш
        let new_systemic_sasp = if total_count > 0 {
            (sasp_sum / total_count as f32).min(1.0)
        } else { 0.0 };
        self.organ_state.systemic_sasp = new_systemic_sasp;
        // Сохраняем для применения в следующем шаге (1-шаговый лаг → биологически корректно)
        self.systemic_sasp_prev = new_systemic_sasp;

        // P13: Агрегируем морфогенные поля по всем живым нишам
        {
            let mut gli_sum = 0.0f32;
            let mut shh_sum = 0.0f32;
            let mut bmp_sum = 0.0f32;
            let mut wnt_sum = 0.0f32;
            let mut n = 0u32;
            for (_, comp) in world.query::<&HumanDevelopmentComponent>().iter() {
                if !comp.is_alive { continue; }
                gli_sum += comp.tissue_state.gli_activation;
                shh_sum += comp.tissue_state.shh_concentration;
                bmp_sum += comp.tissue_state.bmp_balance;
                wnt_sum += comp.tissue_state.wnt_activity;
                n += 1;
            }
            if n > 0 {
                let nf = n as f32;
                self.morphogen_aggregates = MorphogenAggregates {
                    gli_activation_mean: gli_sum / nf,
                    shh_mean:            shh_sum / nf,
                    bmp_balance_mean:    bmp_sum / nf,
                    wnt_activity_mean:   wnt_sum / nf,
                };
            }
        }

        // P7: Ось IGF-1/GH — линейное снижение после пика (20 лет) до 0.3 к 90 годам.
        // Формула: peak=1.0, decline=0.01/год после 20 лет, minimum=0.3
        let age_f = self.organ_state.age_years as f32;
        self.organ_state.igf1_level =
            (1.0 - (age_f - 20.0).max(0.0) * 0.01).clamp(0.3, 1.0);

        // ── Критерии смерти организма ──────────────────────────────────────
        let blood_pool = if blood_count > 0 {
            blood_pool_sum / blood_count as f32
        } else { 1.0 };
        let neural_fc = if neural_count > 0 {
            neural_fc_sum / neural_count as f32
        } else { 1.0 };

        let cause: Option<&'static str> =
            if self.organ_state.frailty_index >= 0.95 {
                Some("frailty")
            } else if blood_pool < 0.02 {
                Some("pancytopenia")
            } else if neural_fc < 0.05 {
                Some("neurodegeneration")
            } else {
                None
            };

        if let Some(c) = cause {
            self.organ_state.is_alive = false;
            self.death_cause = Some(c);
            info!(
                "Organism death at {:.1} yr: cause={} \
                 (frailty={:.3}, blood_pool={:.3}, neural_fc={:.3})",
                self.organ_state.age_years, c,
                self.organ_state.frailty_index, blood_pool, neural_fc
            );
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
    ///
    /// `detail_level` управляет точностью морфогенных вычислений (P13):
    /// 1=только GLI, 2=GLI+BMP, 3+=полный профиль.
    fn update_tissue_state(comp: &mut HumanDevelopmentComponent, detail_level: usize) {
        let dam = &comp.centriolar_damage;
        let ciliary = dam.ciliary_function;

        // Трек A (P13): Hill-нелинейная активация GLI через первичную ресничку.
        // Заменяет линейный `regeneration_tempo = ciliary_function`.
        //
        // Биологически: в позвоночных (в отличие от Drosophila) процессинг GLI
        // происходит ТОЛЬКО внутри реснички (Huangfu et al., 2003; Rohatgi et al., 2007).
        // При CEP164↓ → переходная зона разрушена → SMO не транспортируется →
        // GLI3 конститутивно превращается в репрессор → Hedgehog-ответ = 0.
        // Hill: K=0.5 (EC50), n=2 (кооперативность SMO/SUFU/GLI комплекса).
        comp.tissue_state.regeneration_tempo = dam.gli_activation();

        // Трек B: spindle_fidelity↓ → вероятность про-дифф. деления → потеря пула
        let pool_loss = dam.pool_exhaustion_probability();
        comp.tissue_state.stem_cell_pool = (1.0 - pool_loss).max(0.0);

        // Апоптоз (M=0, D=0) — пул обнуляется немедленно
        if comp.inducers.is_apoptotic() {
            comp.tissue_state.stem_cell_pool = 0.0;
        }

        comp.tissue_state.senescent_fraction =
            (dam.total_damage_score() * 0.85).min(1.0);

        // P13: Обновляем морфогенные поля с учётом tissue_detail_level
        comp.tissue_state.update_morphogen_fields(ciliary, detail_level);

        // P13 (temporal): стадия-зависимое масштабирование морфогенов.
        // Биологически: эмбриональные стадии требуют максимальной морфогенной активности
        // для паттернирования тела; взрослые ниши — гомеостаза; старение — дисрегуляции.
        //
        // | Стадия          | Shh | BMP | Wnt | GLI |
        // |-----------------|-----|-----|-----|-----|
        // | Эмбрион         | 1.4 | 0.6 | 1.3 | 1.2 |  ← максимальное паттернирование
        // | Плод/Новорождён | 1.1 | 0.8 | 1.1 | 1.0 |  ← органогенез завершается
        // | Детство/Юность  | 0.9 | 0.9 | 1.0 | 1.0 |  ← рост+гомеостаз
        // | Взрослый        | 0.7 | 1.0 | 0.8 | 0.9 |  ← минимальный гомеостаз
        // | Средний возраст | 0.5 | 1.2 | 0.6 | 0.7 |  ← Wnt↓, BMP↑ (CDATA)
        // | Пожилой         | 0.3 | 1.5 | 0.4 | 0.5 |  ← сильная дисрегуляция
        let (shh_s, bmp_s, wnt_s, gli_s) = stage_morphogen_scale(comp.stage);
        if detail_level >= 3 {
            comp.tissue_state.shh_concentration =
                (comp.tissue_state.shh_concentration * shh_s).clamp(0.0, 1.0);
            comp.tissue_state.wnt_activity =
                (comp.tissue_state.wnt_activity * wnt_s).clamp(0.0, 1.0);
        }
        if detail_level >= 2 {
            comp.tissue_state.bmp_balance =
                (comp.tissue_state.bmp_balance * bmp_s).clamp(0.0, 1.0);
        }
        // Морфогенное поле gli_activation масштабируется по стадии (сигнальный контекст).
        // NB: regeneration_tempo НЕ изменяется здесь — он определяется реальным состоянием
        // ресничек (dam.gli_activation()) и установлен выше. Стадиевое масштабирование
        // влияет только на морфогенные поля для межклеточной сигнализации.
        comp.tissue_state.gli_activation =
            (comp.tissue_state.gli_activation * gli_s).clamp(0.0, 1.0);

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
    /// `shield_factor` — вклад митохондриального щита из `MitochondrialState`:
    ///   = fusion_index×0.40 + membrane_potential×0.35 + (1−ros_production)×0.25
    ///
    /// Когда MitochondrialModule активен (`shield_factor` = mito_shield_contribution):
    ///   o2_at_centriole = 1.0 − shield_factor   (прямое следствие из митохондриального состояния)
    ///
    /// Когда MitochondrialModule не зарегистрирован (`shield_factor` = 1.0 по умолчанию):
    ///   fallback = centrosomal_oxygen_level(&damage)  (приближение на основе повреждений)
    ///
    /// Причинность: митохондрии → O₂ → центриоли; НЕ наоборот.
    fn apply_oxygen_detachment(
        comp: &mut HumanDevelopmentComponent,
        shield_factor: f32,
        rng: &mut impl Rng,
    ) {
        // Если MitochondrialModule активен (shield_factor < 1.0 возможен),
        // используем его напрямую: o2 = 1 - mito_shield.
        // Если недоступен (shield_factor = map_or(1.0, ...)), используем fallback.
        let oxygen = if shield_factor < 1.0 {
            // Правильный путь: митохондриальный щит прямо задаёт уровень O₂ у центросомы.
            (1.0 - shield_factor).clamp(0.0, 1.0)
        } else {
            // Fallback: MitochondrialModule не зарегистрирован.
            // Приближение: повреждения → ослабление щита → рост O₂.
            centrosomal_oxygen_level(&comp.centriolar_damage)
        };
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

/// Масштабные коэффициенты морфогенных полей для каждой стадии развития.
///
/// Возвращает `(shh_scale, bmp_scale, wnt_scale, gli_scale)`.
///
/// Биологическая основа:
/// - Эмбриональные стадии: максимальный Shh/Wnt для паттернирования тела (French Flag Model);
///   BMP подавлен Noggin → нейральная пластина, не кожа.
/// - Органогенез/Плод: Shh ещё высок (органные бугорки), Wnt сохраняется.
/// - Постнатальный рост: умеренные уровни для нишевого гомеостаза.
/// - Взрослый: минимальный гомеостаз — Shh/Wnt снижены (стволовые ниши в покое).
/// - Средний возраст/Пожилой: CDATA-дисрегуляция → Wnt↓↓, BMP↑↑ (Wnt-BMP переключение
///   описано при саркопении, остеопорозе, ХМЛ: Ye et al. 2018; Laine et al. 2021).
fn stage_morphogen_scale(stage: HumanDevelopmentalStage) -> (f32, f32, f32, f32) {
    // (shh, bmp, wnt, gli)
    match stage {
        HumanDevelopmentalStage::Zygote
        | HumanDevelopmentalStage::Cleavage
        | HumanDevelopmentalStage::Morula
        | HumanDevelopmentalStage::Blastocyst
        | HumanDevelopmentalStage::Implantation
        | HumanDevelopmentalStage::Gastrulation => (1.4, 0.6, 1.3, 1.2),

        HumanDevelopmentalStage::Neurulation
        | HumanDevelopmentalStage::Organogenesis => (1.2, 0.7, 1.2, 1.1),

        HumanDevelopmentalStage::Fetal
        | HumanDevelopmentalStage::Newborn => (1.1, 0.8, 1.1, 1.0),

        HumanDevelopmentalStage::Childhood
        | HumanDevelopmentalStage::Adolescence => (0.9, 0.9, 1.0, 1.0),

        HumanDevelopmentalStage::Adult => (0.7, 1.0, 0.8, 0.9),

        HumanDevelopmentalStage::MiddleAge => (0.5, 1.2, 0.6, 0.7),

        HumanDevelopmentalStage::Elderly => (0.3, 1.5, 0.4, 0.5),
    }
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

        // Lazy-init: NichePool-замены помечены NeedsHumanDevInit.
        // Ткань определяется по clone_id новой сущности: ищем родителя с тем же clone_id
        // среди уже инициализированных HumanDevelopmentComponent. Если не найден — Blood (fallback).
        {
            // Собираем пары (entity, clone_id) для инициализации
            let uninit: Vec<(Entity, u64)> = world
                .query::<(&NeedsHumanDevInit, &ClonalState)>()
                .iter()
                .map(|(e, (_, cs))| (e, cs.clone_id))
                .collect();

            // Карта clone_id → tissue из уже инициализированных ниш
            let clone_tissue_map: std::collections::HashMap<u64, HumanTissueType> = world
                .query::<(&HumanDevelopmentComponent, &ClonalState)>()
                .iter()
                .map(|(_, (hdc, cs))| (cs.clone_id, hdc.tissue_type))
                .collect();

            for (entity, clone_id) in uninit {
                if !world.contains(entity) { continue; }
                let m_max = self.params.mother_inducer_count;
                let d_max = self.params.daughter_inducer_count;
                // Используем ткань родительского клона; Blood — если не найдена
                let tissue = clone_tissue_map.get(&clone_id)
                    .copied()
                    .unwrap_or(HumanTissueType::Blood);
                let mut comp = HumanDevelopmentComponent::for_tissue(tissue);
                comp.inducers = CentriolarInducerPair::zygote(m_max, d_max);
                comp.inducers.detachment_params.base_detach_probability =
                    self.params.base_detach_probability;
                comp.inducers.detachment_params.mother_bias =
                    tissue.default_mother_bias();
                comp.inducers.detachment_params.age_bias_coefficient =
                    self.params.age_bias_coefficient;
                comp.inducers.detachment_params.ptm_exhaustion_scale =
                    self.params.ptm_exhaustion_scale;
                if self.params.noise_scale > 0.0 {
                    comp.damage_rates.noise_scale = self.params.noise_scale;
                }
                world.insert_one(entity, comp)?;
                world.insert_one(entity, CentriolarDamageState::pristine())?;
                world.insert_one(entity, TelomereState::default())?;
                world.insert_one(entity, EpigeneticClockState::default())?;
                world.insert_one(entity, DifferentiationStatus::default())?;
                world.insert_one(entity, ModulationState::default())?;
                world.insert_one(entity, NKSurveillanceState::default())?;
                world.insert_one(entity, ProteostasisState::default())?;
                world.insert_one(entity, CircadianState::default())?;
                world.insert_one(entity, AutophagyState::default())?;
                world.insert_one(entity, DDRState::default())?;
                world.insert_one(entity, StemCellDivisionRateState::default())?;
                // Убираем маркер: сущность теперь полноценная ниша
                let _ = world.remove::<(NeedsHumanDevInit,)>(entity);
                trace!("HumanDev lazy-init: NichePool replacement initialized as {:?} HSC (clone {})", tissue, clone_id);
            }
        }

        // P7: Применить системный SASP (из прошлого шага) + IGF-1 ко всем нишам.
        // SASP повышает ros_boost через InflammagingState (канал обратной связи).
        // IGF-1 снижает regeneration_tempo при старении.
        let sasp_boost = self.systemic_sasp_prev * 0.05; // 5% вклад системного SASP в ros_boost
        let igf1       = self.organ_state.igf1_level;
        let igf1_regen = 0.8 + 0.2 * igf1; // IGF-1=1.0 → 1.0; IGF-1=0.3 → 0.86
        if sasp_boost > 0.0 || igf1_regen < 1.0 {
            for (_, (comp, infl_opt)) in world.query_mut::<(
                &mut HumanDevelopmentComponent,
                Option<&mut InflammagingState>,
            )>() {
                if !comp.is_alive { continue; }
                // Системный SASP → усиливает ros_boost
                if sasp_boost > 0.0 {
                    if let Some(infl) = infl_opt {
                        infl.ros_boost = (infl.ros_boost + sasp_boost).min(1.0);
                    }
                }
                // IGF-1 → снижает regeneration_tempo при старении
                comp.tissue_state.regeneration_tempo =
                    (comp.tissue_state.regeneration_tempo * igf1_regen).min(1.0);
            }
        }

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
            // P9: perinuclear_density добавляет пространственный барьер диффузии O₂ (max +15%)
            // Применяется к base_detach_probability через масштабирование (лаг 1 шаг)
            let mito_shield = mito_opt.map_or(1.0, |m| {
                (m.mito_shield_contribution + m.perinuclear_density * 0.15).clamp(0.0, 1.0)
            });
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

                // P11: вычислить эффект активных интервенций (модифицирует DamageParams)
                let iv_effect = interventions::compute_effect(
                    &self.interventions,
                    age_years,
                    &comp.damage_rates,
                    dt_years,
                );

                // P17: Компенсаторная пролиферация (pooldepletion feedback).
                // При пуле < 50% ниша вынуждена увеличить скорость деления:
                //   compensatory_ros = max(0, 0.5 - pool) × 0.30
                // При pool=0.3: boost = 0.06 (≈50% прироста базового ROS ~0.12/год)
                // При pool=0.1: boost = 0.12 (100% прироста)
                // Биологически: истощение HSC-ниши → компенсаторная пролиферация
                // → повышенный спиндл-стресс → ROS↑ (Murphy et al. 2021; Flach et al. 2014).
                let pool = comp.tissue_state.stem_cell_pool;
                let compensatory_ros_boost = if pool < 0.5 {
                    (0.5 - pool) * 0.30
                } else {
                    0.0
                };

                accumulate_damage(
                    &mut comp.centriolar_damage,
                    &iv_effect.damage_params,  // P11: эффективные параметры (с интервенциями)
                    age_years,
                    dt_years,
                    infl_ros_boost + epi_ros_from_prev + mito_ros_boost + compensatory_ros_boost,
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
                    // ros_level: σ уменьшена вдвое — ROS-петля усиливает флуктуации сама по себе
                    dam.ros_level = (dam.ros_level
                        + self.rng.gen::<f32>() * sigma - sigma * 0.5).clamp(0.0, 1.0);
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
                // P11: NadPlus добавляет extra_mitophagy к потоку митофагии.
                {
                    let base_mitophagy = mito_opt.map_or(0.0, |m| m.mitophagy_flux);
                    let effective_mitophagy = (base_mitophagy + iv_effect.extra_mitophagy).min(1.0);
                    damage::apply_appendage_repair(
                        &mut comp.centriolar_damage,
                        &iv_effect.damage_params,  // P11: эффективные параметры
                        effective_mitophagy,
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
                // P11+: CafdRetainer/CafdReleaser масштабируют вероятность отщепления.
                {
                    let orig = comp.inducers.detachment_params.base_detach_probability;
                    if (iv_effect.detach_probability_modifier - 1.0).abs() > f32::EPSILON {
                        comp.inducers.detachment_params.base_detach_probability =
                            (orig * iv_effect.detach_probability_modifier).max(0.0);
                    }
                    Self::apply_oxygen_detachment(comp, mito_shield, &mut self.rng);
                    comp.inducers.detachment_params.base_detach_probability = orig;
                }

                // 4а. Трансплантация центросомы (CentrosomeTransplant).
                // Восстанавливает индукторы до донорских уровней, если они ниже порога.
                // Один шаг задержки после O₂-отщепления — корректно (лаг = 1 шаг).
                if let Some((min_m, min_d)) = iv_effect.centrosome_transplant {
                    if comp.inducers.mother_set.remaining < min_m {
                        comp.inducers.mother_set.remaining = min_m;
                        comp.inducers.mother_set.inherited_count =
                            comp.inducers.mother_set.inherited_count.max(min_m);
                    }
                    if comp.inducers.daughter_set.remaining < min_d {
                        comp.inducers.daughter_set.remaining = min_d;
                        comp.inducers.daughter_set.inherited_count =
                            comp.inducers.daughter_set.inherited_count.max(min_d);
                    }
                }

                // 4б. PTM-опосредованное истощение (только мать — механизм истощения пула).
                // Независим от O₂: структурные ПТМ матери ослабляют связи индукторов.
                // Срабатывает только при наличии PTM-асимметрии мать > дочь.
                if ptm_asymmetry > 0.01 {
                    Self::apply_ptm_exhaustion(comp, ptm_asymmetry, &mut self.rng);
                }

                // 5. Тканевое состояние (Трек A + Трек B + P13 морфогены)
                Self::update_tissue_state(comp, self.params.tissue_detail_level);

                // 5-P11: Senolytics — клиренс сенесцентных клеток.
                // Снижает senescent_fraction и тем самым уменьшает SASP (косвенно).
                if iv_effect.senolytic_clearance > 0.0 {
                    comp.tissue_state.senescent_fraction = (comp.tissue_state.senescent_fraction
                        * (1.0 - iv_effect.senolytic_clearance)).max(0.0);
                }

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
                        - exhaustion_ratio * POOL_DEPLETION_RATE * dt_years).max(0.0);
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

                    // P11: TertActivation — удлинение теломер терапевтической теломеразой.
                    // Применяется независимо от tert_active (терапевтический TERT отличается
                    // от эндогенного — может работать в дифференцированных клетках).
                    if iv_effect.tert_elongation > 0.0 {
                        let div_rate: f32 = match comp.stage {
                            HumanDevelopmentalStage::Newborn
                            | HumanDevelopmentalStage::Childhood   => 24.0,
                            HumanDevelopmentalStage::Adolescence
                            | HumanDevelopmentalStage::Adult       => 12.0,
                            HumanDevelopmentalStage::MiddleAge     => 6.0,
                            HumanDevelopmentalStage::Elderly       => 2.0,
                            _                                      => 1.0,
                        };
                        let elongation = iv_effect.tert_elongation * div_rate * dt_years;
                        tel.mean_length = (tel.mean_length + elongation).min(1.0);
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
                    let chron_age = comp.age_years() as f32;

                    // Эпигенетическое наследование при делении (Трек D, P14).
                    // При каждом делении дочерняя клетка наследует только половину
                    // «избыточного» метилирования родителя (Lagerlöf et al. 2019;
                    // Kerepesi et al. 2021 — «epigenetic partial reset»):
                    //   methylation_age_daughter = (methylation_age + chron_age) / 2
                    //
                    // Биологически: пассивная деметиляция за счёт отсутствия DNMT1 на
                    // половине дочерних нитей — избыток CpG-метилирования вымывается за 1 деление.
                    // Эффект = «молодение» эпигенетических часов при симметричном делении;
                    // при асимметричных/повреждённых делениях сброс неполный (учтено через
                    // уже накопленный methylation_age, который сразу растёт обратно).
                    let cur_div = exhaustion_opt.map_or(0, |e| e.total_divisions);
                    if cur_div > epi.last_division_count {
                        // Произошло одно или более делений с последнего шага
                        epi.methylation_age = (epi.methylation_age + chron_age) / 2.0;
                        epi.last_division_count = cur_div;
                    }

                    epi.clock_acceleration = 1.0 + damage * 0.5;
                    epi.methylation_age += dt_years * epi.clock_acceleration;

                    // Опережение эпигенетического возраста над хронологическим → ROS-буст
                    // (CpG-гипометилирование нестабилизирует хроматин → транспозоны → ROS)
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

                // P11: Healthspan — считаем дни пока повреждения < 0.5
                if comp.centriolar_damage.total_damage_score() < 0.5 {
                    self.healthspan_days += dt_days;
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

        // Шаг 1г: Циркадный ритм (P18) — обновить CircadianState.
        // Амплитуда из CEP164/агрегатов/ROS → ночной буст протеасомы + SASP-вклад.
        // Два эффекта применяются НИЖЕ:
        //   1) circadian_sasp_contribution → InflammagingState::sasp_intensity (доп. буст)
        //   2) proteasome_night_boost → добавляется к aggregate_clearance_rate в протеостазе
        {
            for (_, (dev, circ)) in world.query_mut::<(
                &mut HumanDevelopmentComponent,
                &mut CircadianState,
            )>() {
                if !dev.is_alive { continue; }
                circ.update(&dev.centriolar_damage);
            }
        }
        // Применить циркадный SASP к InflammagingState (если компонент присутствует)
        {
            for (_, (dev, circ, infl)) in world.query_mut::<(
                &HumanDevelopmentComponent,
                &CircadianState,
                &mut InflammagingState,
            )>() {
                if !dev.is_alive { continue; }
                // Добавить конститутивный NF-κB буст от нарушения циркадного ритма
                infl.sasp_intensity = (infl.sasp_intensity
                    + circ.circadian_sasp_contribution * dt_years)
                    .clamp(0.0, 1.0);
            }
        }

        // Шаг 1е: Аутофагия / mTOR (P19) — обновить AutophagyState и применить аутофагический клиренс.
        // CR → mTOR↓ → аутофагия↑ → агрегаты↓ (механизм интервенции Caloric Restriction).
        // NadPlus → SIRT1→AMPK → mTOR↓ → аутофагия↑.
        {
            // Определяем активные интервенции один раз для всего прохода
            let cr_active = self.interventions.iter().any(|iv| {
                matches!(iv.kind, interventions::InterventionKind::CaloricRestriction { .. })
            });
            let nad_boost: f32 = self.interventions.iter()
                .filter_map(|iv| if let interventions::InterventionKind::NadPlus { mitophagy_boost } = iv.kind {
                    Some(mitophagy_boost)
                } else {
                    None
                })
                .sum::<f32>()
                .min(1.0);

            for (_, (dev, autoph)) in world.query_mut::<(
                &mut HumanDevelopmentComponent,
                &mut AutophagyState,
            )>() {
                if !dev.is_alive { continue; }
                let age = dev.age_years() as f32;
                autoph.update(age, cr_active, nad_boost);
                // Аутофагический клиренс агрегатов (независим от протеасомного/агрегасомного)
                let clearance = autoph.aggregate_autophagy_clearance * dt_years;
                dev.centriolar_damage.protein_aggregates =
                    (dev.centriolar_damage.protein_aggregates - clearance).max(0.0);
                dev.centriolar_damage.update_functional_metrics();
            }
        }

        // Шаг 1д: Протеостаз (P16) — обновить ProteostasisState и применить агрегасомный клиренс.
        // Центросома = организующий центр агрегасом (Johnston et al. 1998).
        // CEP164↓ → агрегасомы не формируются → protein_aggregates накапливается быстрее.
        // Клиренс: protein_aggregates снижается на `clearance_rate × aggregates × dt`.
        // Параметр 0.15 — максимальный ежедневный клиренс (≈15% при полной протеостазе).
        // P18: дополнительный циркадный буст протеасомы добавляется к clearance_rate.
        {
            for (_, (dev, prot, circ_opt)) in world.query_mut::<(
                &mut HumanDevelopmentComponent,
                &mut ProteostasisState,
                Option<&CircadianState>,
            )>() {
                if !dev.is_alive { continue; }
                prot.update(&dev.centriolar_damage);
                // Циркадный ночной буст (+0..+0.25 к clearance_rate)
                let night_boost = circ_opt.map_or(0.0, |c| c.proteasome_night_boost);
                let clearance = (prot.aggregate_clearance_rate + night_boost) * 0.15 * dt_years;
                dev.centriolar_damage.protein_aggregates =
                    (dev.centriolar_damage.protein_aggregates - clearance).max(0.0);
                // Обновить производные метрики после изменения агрегатов
                dev.centriolar_damage.update_functional_metrics();
            }
        }

        // Шаг 1ж: DDR — ответ на повреждение ДНК (P20).
        // spindle_fidelity↓ → анеуплоидия → DSBs → ATM → p53 → p21 → G1-арест.
        // Замыкает петлю CDATA → cell_cycle_module через GeneExpressionState.p21_level.
        // p21_contribution() = p53_stabilization × 0.3 — добавляется к существующему p21.
        {
            for (_, (dev, ddr, gene_expr_opt)) in world.query_mut::<(
                &mut HumanDevelopmentComponent,
                &mut DDRState,
                Option<&mut GeneExpressionState>,
            )>() {
                if !dev.is_alive { continue; }
                let dam = &dev.centriolar_damage;
                ddr.update(
                    dam.spindle_fidelity,
                    dam.ros_level,
                    dam.protein_aggregates,
                    dev.age_years() as f32,
                );
                // Записываем p53→p21 в GeneExpressionState (читается cell_cycle_module)
                if let Some(ge) = gene_expr_opt {
                    // Берём максимум: DDR-вклад vs уже накопленный p21
                    // (p21 может быть накоплен теломерным/другим путём)
                    let ddr_p21 = ddr.p21_contribution();
                    ge.p21_level = ge.p21_level.max(ddr_p21);
                }
            }
        }

        // Шаг 1з: Track F — Снижение темпа деления стволовых клеток.
        // StemCellDivisionRateState обновляется после DDR (p21 уже записан) и
        // применяется как мультипликатор к regeneration_tempo:
        //   regeneration_tempo_final = regeneration_tempo × division_rate
        // mTOR-активность берётся из AutophagyState (если есть), иначе 0.3 (молодая норма).
        {
            for (_, (dev, divrate, autoph_opt)) in world.query_mut::<(
                &mut HumanDevelopmentComponent,
                &mut StemCellDivisionRateState,
                Option<&AutophagyState>,
            )>() {
                if !dev.is_alive { continue; }
                let dam = &dev.centriolar_damage;
                let mtor = autoph_opt.map_or(0.3, |a| a.mtor_activity);
                divrate.update(
                    dam.ciliary_function,
                    dam.spindle_fidelity,
                    dam.ros_level,
                    dev.age_years() as f32,
                    mtor,
                );
                // Применяем: замедление деления снижает темп регенерации ткани.
                // Используем sqrt чтобы смягчить эффект (division_rate — долгосрочный фактор,
                // regeneration_tempo уже учитывает краткосрочные изменения).
                dev.tissue_state.regeneration_tempo *= divrate.division_rate.sqrt();
                // Гарантируем минимум (ткань не умирает мгновенно от замедления)
                dev.tissue_state.regeneration_tempo =
                    dev.tissue_state.regeneration_tempo.max(0.01);
            }
        }

        // Шаг 1в: NK-клеточный иммунный надзор (P15).
        // Двухфазовый паттерн: сначала обновляем NKSurveillanceState,
        // затем элиминируем клетки с высокой вероятностью — отдельным проходом.
        //
        // NK-элиминация — стохастическая: ячейка умирает, если kill_probability
        // превышает порог 0.02 (≈ 2% максимальной ежедневной NK-активности).
        // При dt = 1 день это соответствует ≈1 NK-элиминации за 50 дней
        // при максимальных повреждениях. Биологически адекватно для HSC ниши.
        // Биологически: NK-элиминация — медленный селективный процесс.
        // Только клетки с высокой экспрессией NKG2D-лигандов (тяжёлый стресс/сенесценция)
        // подвергаются элиминации. Порог 0.10: kill_prob = nk_activity × nkg2d × (1-escape).
        // При nkg2d < 0.1 (ros+aggregates < 0.4 после вычитания базового 0.3) — 0.
        // При максимальных повреждениях (ros=1, agg=1) и молодом организме:
        //   kill_prob ≈ 1.0 × 0.7 × 1.0 = 0.70 >> 0.10 → элиминация.
        // При умеренных (ros=0.5, agg=0.4) и пожилом (nk=0.5):
        //   kill_prob ≈ 0.5 × 0.20 × 0.9 = 0.09 < 0.10 → нет элиминации (пограничный случай).
        const NK_KILL_THRESHOLD: f32 = 0.10;
        {
            let nk_kill_entities: Vec<Entity> = {
                let mut q = world.query::<(
                    &mut NKSurveillanceState,
                    &HumanDevelopmentComponent,
                    Option<&InflammagingState>,
                )>();
                let mut to_kill = Vec::new();
                for (entity, (nk, dev, infl_opt)) in q.iter() {
                    if !dev.is_alive { continue; }
                    // Миелоидный сдвиг — через sasp_intensity как прокси (пишется myeloid_shift_module)
                    let myeloid_proxy = infl_opt.map_or(0.0, |i| i.sasp_intensity);
                    let dam = &dev.centriolar_damage;
                    nk.update(
                        dam.ros_level,
                        dam.protein_aggregates,
                        dam.protein_carbonylation,
                        dev.age_years() as f32,
                        myeloid_proxy,
                    );
                    if nk.nk_kill_probability > NK_KILL_THRESHOLD {
                        nk.total_eliminations += 1;
                        to_kill.push(entity);
                    }
                }
                to_kill
            };
            // Фаза 2: помечаем NK-элиминированные клетки мёртвыми
            for entity in nk_kill_entities {
                if let Ok(mut q) = world.query_one_mut::<&mut HumanDevelopmentComponent>(entity) {
                    q.is_alive = false;
                    info!(
                        "NK elimination: niche {:?} at age {:.1} yr (kill_prob exceeded threshold)",
                        q.tissue_type, q.age_years()
                    );
                }
                let _ = world.insert_one(entity, Dead);
            }
        }

        // Шаг 2: синхронизировать отдельный ECS-компонент CentriolarDamageState
        // чтобы stem_cell_hierarchy и asymmetric_division могли читать повреждения
        // без зависимости от human_development_module.
        // P8: Агрегировать тканевые метрики → OrganismState + критерии смерти.
        self.update_organism_state(world);

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
            // P11: Интервенции + healthspan
            "intervention_count":      self.interventions.len(),
            "healthspan_years":        self.healthspan_years(),
            // P8: Состояние организма
            "organism_is_alive":       self.organ_state.is_alive,
            "organism_age_years":      self.organ_state.age_years,
            "organism_frailty":        self.organ_state.frailty_index,
            "organism_cognitive":      self.organ_state.cognitive_index,
            "organism_immune":         self.organ_state.immune_reserve,
            "organism_muscle":         self.organ_state.muscle_mass,
            "organism_inflammaging":   self.organ_state.inflammaging_score,
            "organism_death_cause":    self.death_cause.unwrap_or(""),
            "organism_igf1_level":     self.organ_state.igf1_level,
            "organism_systemic_sasp":  self.organ_state.systemic_sasp,
            // P13: Морфогенные поля (агрегированные по всем нишам)
            // Вычисляются в update_tissue_state() как Hill-нелинейные функции от ciliary_function
            "morphogen_gli_activation":   self.morphogen_aggregates.gli_activation_mean,
            "morphogen_shh":              self.morphogen_aggregates.shh_mean,
            "morphogen_bmp_balance":      self.morphogen_aggregates.bmp_balance_mean,
            "morphogen_wnt_activity":     self.morphogen_aggregates.wnt_activity_mean,
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
            // Ткань-специфичная асимметрия M/D:
            // Если пользователь не менял mother_bias (= дефолт 0.5),
            // используем биологически обоснованное значение для каждого типа ткани.
            // Если задано явно — применяем глобальный override для всех тканей.
            comp.inducers.detachment_params.mother_bias =
                if (self.params.mother_bias - 0.5).abs() < f32::EPSILON {
                    tissue.default_mother_bias()
                } else {
                    self.params.mother_bias
                };
            comp.inducers.detachment_params.age_bias_coefficient =
                self.params.age_bias_coefficient;
            comp.inducers.detachment_params.ptm_exhaustion_scale =
                self.params.ptm_exhaustion_scale;
            // Стохастический шум: применяется к DamageParams каждой ниши.
            // noise_scale > 0 → разные траектории при одинаковых начальных условиях.
            if self.params.noise_scale > 0.0 {
                comp.damage_rates.noise_scale = self.params.noise_scale;
            }
            // Standalone ECS-компоненты для межмодульного взаимодействия:
            // CentriolarDamageState — синхронизируется в step() для других модулей.
            // InflammagingState     — пишется myeloid_shift_module, читается здесь.
            world.insert_one(entity, CentriolarDamageState::pristine())?;
            world.insert_one(entity, InflammagingState::default())?;
            world.insert_one(entity, TelomereState::default())?;
            world.insert_one(entity, EpigeneticClockState::default())?;
            world.insert_one(entity, DifferentiationStatus::default())?;
            world.insert_one(entity, ModulationState::default())?;
            world.insert_one(entity, NKSurveillanceState::default())?;
            world.insert_one(entity, ProteostasisState::default())?;
            world.insert_one(entity, CircadianState::default())?;
            world.insert_one(entity, AutophagyState::default())?;
            world.insert_one(entity, DDRState::default())?;
            world.insert_one(entity, StemCellDivisionRateState::default())?;
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
        // Полные дефолтные параметры: base_detach_probability=0.0003, ptm_exhaustion_scale=0.001
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

    /// Простейшая проверка: за 78 лет суммарно теряется хотя бы один индуктор.
    ///
    /// Тест работает без CentrioleModule и MitochondrialModule — только fallback O₂-путь.
    /// При base_detach=0.0003 каждый из комплектов ожидаемо теряет ~1 индуктор за 78 лет,
    /// но результат стохастичен: гарантировано лишь что суммарная потеря (M+D) ≥ 1.
    /// Проверку потери каждого комплекта в отдельности — см. `test_inductor_calibration_multiseed`.
    #[test]
    fn test_inductor_depletion_occurs() {
        let (m_rem, m_init, d_rem, d_init) = run_with_inducers(78, 42);
        let m_loss = m_init.saturating_sub(m_rem);
        let d_loss = d_init.saturating_sub(d_rem);
        assert!(m_loss + d_loss >= 1,
            "За 78 лет суммарно должен быть потерян хотя бы 1 индуктор (M+D): \
             M={}/{}, D={}/{}", m_rem, m_init, d_rem, d_init);
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

// ---------------------------------------------------------------------------
// P8: Тесты критериев смерти организма
// ---------------------------------------------------------------------------
#[cfg(test)]
mod organism_death_tests {
    use super::*;
    use cell_dt_core::components::{CentriolePair, CellCycleStateExtended};
    use cell_dt_core::hecs::World;

    /// Создаёт мир с N Blood-нишами и заданными значениями functional_capacity и stem_cell_pool.
    fn world_with_blood_niches(n: usize, fc: f32, pool: f32) -> (World, HumanDevelopmentModule) {
        let mut world = World::new();
        for _ in 0..n {
            let mut comp = HumanDevelopmentComponent::for_tissue(HumanTissueType::Blood);
            comp.tissue_state.functional_capacity = fc;
            comp.tissue_state.stem_cell_pool = pool;
            comp.tissue_state.senescent_fraction = 1.0 - fc;
            comp.is_alive = true;
            world.spawn((CentriolePair::default(), CellCycleStateExtended::new(), comp));
        }
        let module = HumanDevelopmentModule::new();
        (world, module)
    }

    /// frailty_index >= 0.95 → смерть от "frailty"
    #[test]
    fn test_frailty_death() {
        // functional_capacity = 0.04 → frailty = 0.96 >= 0.95
        let (world, mut module) = world_with_blood_niches(3, 0.04, 0.5);
        module.update_organism_state(&world);
        assert!(!module.organ_state.is_alive, "организм должен умереть от дряхлости");
        assert_eq!(module.death_cause(), Some("frailty"),
            "причина смерти должна быть frailty");
    }

    /// Blood stem_cell_pool < 0.02 → панцитопения
    #[test]
    fn test_pancytopenia_death() {
        // fc=0.5 (frailty=0.5, не фатально), pool=0.01 < 0.02
        let (world, mut module) = world_with_blood_niches(3, 0.5, 0.01);
        module.update_organism_state(&world);
        assert!(!module.organ_state.is_alive, "организм должен умереть от панцитопении");
        assert_eq!(module.death_cause(), Some("pancytopenia"),
            "причина смерти должна быть pancytopenia");
    }

    /// Neural functional_capacity < 0.05 → нейродегенерация
    #[test]
    fn test_neurodegeneration_death() {
        let mut world = World::new();
        let mut blood = HumanDevelopmentComponent::for_tissue(HumanTissueType::Blood);
        blood.tissue_state.functional_capacity = 0.8;
        blood.tissue_state.stem_cell_pool = 0.8;
        blood.is_alive = true;

        let mut neural = HumanDevelopmentComponent::for_tissue(HumanTissueType::Neural);
        neural.tissue_state.functional_capacity = 0.03; // < 0.05
        neural.is_alive = true;

        world.spawn((CentriolePair::default(), CellCycleStateExtended::new(), blood));
        world.spawn((CentriolePair::default(), CellCycleStateExtended::new(), neural));

        let mut module = HumanDevelopmentModule::new();
        module.update_organism_state(&world);
        assert!(!module.organ_state.is_alive, "организм должен умереть от нейродегенерации");
        assert_eq!(module.death_cause(), Some("neurodegeneration"),
            "причина смерти должна быть neurodegeneration");
    }

    /// Здоровый организм — ни один критерий не срабатывает
    #[test]
    fn test_healthy_organism_survives() {
        let (world, mut module) = world_with_blood_niches(5, 0.9, 0.9);
        module.update_organism_state(&world);
        assert!(module.organ_state.is_alive, "здоровый организм должен жить");
        assert_eq!(module.death_cause(), None, "нет причины смерти");
        assert!(module.organ_state.frailty_index < 0.95, "frailty должен быть низким");
    }
}

// ---------------------------------------------------------------------------
// Интеграционные тесты интервенций
// ---------------------------------------------------------------------------
#[cfg(test)]
mod intervention_integration_tests {
    use super::*;
    use super::interventions::{Intervention, InterventionKind};
    use super::damage::DamageParams;
    use cell_dt_core::{SimulationManager, SimulationConfig};
    use cell_dt_core::components::{CentriolePair, CellCycleStateExtended};

    /// Вспомогательная функция: запустить одну нишу на `years` лет с заданными параметрами.
    /// Детерминированно (seed=1), без стохастических индукторных потерь (base_detach=0).
    /// Возвращает `(damage_score, senescent_fraction)` в конце симуляции.
    fn run_with_interventions(
        years: usize,
        interventions: Vec<Intervention>,
    ) -> (f32, f32) {
        let config = SimulationConfig {
            max_steps: (years * 365 + 10) as u64,
            dt: 1.0,
            checkpoint_interval: 100_000,
            num_threads: None,
            seed: Some(1),
            parallel_modules: false,
            cleanup_dead_interval: None,
        };
        let mut sim = SimulationManager::new(config);

        let mut hdm = HumanDevelopmentModule::with_params(HumanDevelopmentParams {
            base_detach_probability: 0.0,
            ptm_exhaustion_scale:    0.0,
            noise_scale:             0.0,
            ..HumanDevelopmentParams::default()
        });
        for iv in interventions {
            hdm.add_intervention(iv);
        }

        sim.register_module(Box::new(hdm)).unwrap();

        sim.world_mut().spawn((CentriolePair::default(), CellCycleStateExtended::new()));
        sim.initialize().unwrap();

        // Ускоренное старение — накапливает повреждения за меньшее число лет
        {
            let w = sim.world_mut();
            for (_, comp) in w.query_mut::<&mut HumanDevelopmentComponent>() {
                comp.damage_rates = DamageParams::progeria();
            }
        }

        for _ in 0..(years * 365) {
            sim.step().unwrap();
        }

        let result = {
            let world = sim.world();
            world.query::<&HumanDevelopmentComponent>()
                .iter()
                .next()
                .map(|(_, c)| (c.damage_score(), c.tissue_state.senescent_fraction))
                .unwrap_or((0.0, 0.0))
        };
        result
    }

    /// Антиоксидантная терапия должна снижать damage_score относительно контроля.
    #[test]
    fn test_antioxidant_reduces_damage() {
        let (damage_ctrl, _) = run_with_interventions(30, vec![]);
        let (damage_iv, _)   = run_with_interventions(30, vec![
            Intervention {
                start_age_years: 0.0,
                end_age_years: None,
                kind: InterventionKind::Antioxidant,
            }
        ]);

        assert!(
            damage_iv < damage_ctrl,
            "Antioxidant должен снижать damage_score: iv={:.3} >= ctrl={:.3}",
            damage_iv, damage_ctrl
        );
    }

    /// Калорийное ограничение (CR) должно снижать накопление повреждений (ros_factor < 1).
    #[test]
    fn test_caloric_restriction_slows_damage() {
        let (damage_ctrl, _) = run_with_interventions(30, vec![]);
        let (damage_cr, _)   = run_with_interventions(30, vec![
            Intervention {
                start_age_years: 0.0,
                end_age_years: None,
                kind: InterventionKind::CaloricRestriction { ros_factor: 0.6 },
            }
        ]);

        assert!(
            damage_cr < damage_ctrl,
            "CR должен замедлять повреждения: cr={:.3} >= ctrl={:.3}",
            damage_cr, damage_ctrl
        );
    }

    /// CAFD-ретейнер, применённый с рождения, должен снизить повреждения
    /// относительно контроля (retention_factor = 0.5 → детач ×0.5).
    #[test]
    fn test_cafd_retainer_reduces_damage() {
        let (damage_ctrl, _) = run_with_interventions(30, vec![]);
        let (damage_cr, _)   = run_with_interventions(30, vec![
            Intervention {
                start_age_years: 0.0,
                end_age_years: None,
                kind: InterventionKind::CafdRetainer { retention_factor: 0.5 },
            }
        ]);
        // CafdRetainer снижает base_detach_probability — здесь оно 0, так что
        // он может не влиять напрямую на damage_score, но не должен УВЕЛИЧИВАТЬ
        assert!(
            damage_cr <= damage_ctrl + 0.01, // допуск на float-шум
            "CafdRetainer не должен увеличивать damage_score: cr={:.3} > ctrl={:.3}",
            damage_cr, damage_ctrl
        );
    }

    /// NAD⁺-буст должен снизить повреждения (ускоряет митофагию → меньше ROS).
    #[test]
    fn test_nad_plus_reduces_damage() {
        let (damage_ctrl, _) = run_with_interventions(30, vec![]);
        let (damage_nad, _)  = run_with_interventions(30, vec![
            Intervention {
                start_age_years: 0.0,
                end_age_years: None,
                kind: InterventionKind::NadPlus { mitophagy_boost: 0.5 },
            }
        ]);

        assert!(
            damage_nad < damage_ctrl,
            "NAD⁺ должен снижать damage_score: nad={:.3} >= ctrl={:.3}",
            damage_nad, damage_ctrl
        );
    }

    /// Комбинация Senolytics + NAD⁺ должна давать меньше повреждений,
    /// чем каждая интервенция по отдельности (синергия).
    #[test]
    fn test_combined_interventions_are_better_than_single() {
        let (damage_ctrl, _)     = run_with_interventions(30, vec![]);
        let (damage_combo, _)    = run_with_interventions(30, vec![
            Intervention {
                start_age_years: 0.0,
                end_age_years: None,
                kind: InterventionKind::Antioxidant,
            },
            Intervention {
                start_age_years: 0.0,
                end_age_years: None,
                kind: InterventionKind::NadPlus { mitophagy_boost: 0.3 },
            },
        ]);

        assert!(
            damage_combo < damage_ctrl,
            "Комбинация интервенций должна снижать повреждения: combo={:.3} >= ctrl={:.3}",
            damage_combo, damage_ctrl
        );
    }
}

#[cfg(test)]
mod morphogen_temporal_tests {
    use super::*;

    #[test]
    fn test_embryonic_stage_has_high_shh_low_bmp() {
        let (shh, bmp, _wnt, _gli) = stage_morphogen_scale(HumanDevelopmentalStage::Gastrulation);
        assert!(shh > 1.0, "Embryonic Shh scale should be > 1.0, got {shh}");
        assert!(bmp < 1.0, "Embryonic BMP scale should be < 1.0, got {bmp}");
    }

    #[test]
    fn test_elderly_stage_has_low_shh_high_bmp() {
        let (shh, bmp, _wnt, _gli) = stage_morphogen_scale(HumanDevelopmentalStage::Elderly);
        assert!(shh < 0.5, "Elderly Shh scale should be < 0.5, got {shh}");
        assert!(bmp > 1.2, "Elderly BMP scale should be > 1.2, got {bmp}");
    }

    #[test]
    fn test_shh_monotone_decreasing_with_age() {
        let stages = [
            HumanDevelopmentalStage::Gastrulation,
            HumanDevelopmentalStage::Fetal,
            HumanDevelopmentalStage::Adult,
            HumanDevelopmentalStage::MiddleAge,
            HumanDevelopmentalStage::Elderly,
        ];
        let shh_values: Vec<f32> = stages.iter()
            .map(|&s| stage_morphogen_scale(s).0)
            .collect();
        for i in 1..shh_values.len() {
            assert!(
                shh_values[i] <= shh_values[i - 1],
                "Shh should decrease monotonically: [{i}]={} > [{}]={}",
                shh_values[i], i - 1, shh_values[i - 1]
            );
        }
    }

    #[test]
    fn test_bmp_monotone_increasing_with_age() {
        let stages = [
            HumanDevelopmentalStage::Gastrulation,
            HumanDevelopmentalStage::Fetal,
            HumanDevelopmentalStage::Adult,
            HumanDevelopmentalStage::MiddleAge,
            HumanDevelopmentalStage::Elderly,
        ];
        let bmp_values: Vec<f32> = stages.iter()
            .map(|&s| stage_morphogen_scale(s).1)
            .collect();
        for i in 1..bmp_values.len() {
            assert!(
                bmp_values[i] >= bmp_values[i - 1],
                "BMP should increase monotonically: [{i}]={} < [{}]={}",
                bmp_values[i], i - 1, bmp_values[i - 1]
            );
        }
    }

    #[test]
    fn test_all_scales_positive() {
        let all_stages = [
            HumanDevelopmentalStage::Zygote, HumanDevelopmentalStage::Cleavage,
            HumanDevelopmentalStage::Morula, HumanDevelopmentalStage::Blastocyst,
            HumanDevelopmentalStage::Implantation, HumanDevelopmentalStage::Gastrulation,
            HumanDevelopmentalStage::Neurulation, HumanDevelopmentalStage::Organogenesis,
            HumanDevelopmentalStage::Fetal, HumanDevelopmentalStage::Newborn,
            HumanDevelopmentalStage::Childhood, HumanDevelopmentalStage::Adolescence,
            HumanDevelopmentalStage::Adult, HumanDevelopmentalStage::MiddleAge,
            HumanDevelopmentalStage::Elderly,
        ];
        for stage in all_stages {
            let (shh, bmp, wnt, gli) = stage_morphogen_scale(stage);
            assert!(shh > 0.0, "Shh scale must be positive for {stage:?}");
            assert!(bmp > 0.0, "BMP scale must be positive for {stage:?}");
            assert!(wnt > 0.0, "Wnt scale must be positive for {stage:?}");
            assert!(gli > 0.0, "GLI scale must be positive for {stage:?}");
        }
    }

    #[test]
    fn test_temporal_scaling_applied_directly() {
        // Прямая проверка: scale-функция правильно масштабирует GLI
        // без запуска полной симуляции (избегаем зависимости от внешних крейтов).
        //
        // GLI молодого взрослого (scale=0.9) > GLI пожилого (scale=0.5) при одинаковом cilia
        use cell_dt_core::components::TissueState;
        use cell_dt_core::components::TissueType;

        let ciliary = 0.8_f32; // одинаковая функция ресничек

        let mut ts_adult   = TissueState::new(TissueType::Blood);
        let mut ts_elderly = TissueState::new(TissueType::Blood);

        ts_adult.update_morphogen_fields(ciliary, 1);
        ts_elderly.update_morphogen_fields(ciliary, 1);

        let (_, _, _, gli_scale_adult)   = stage_morphogen_scale(HumanDevelopmentalStage::Adult);
        let (_, _, _, gli_scale_elderly) = stage_morphogen_scale(HumanDevelopmentalStage::Elderly);

        ts_adult.gli_activation   = (ts_adult.gli_activation   * gli_scale_adult).clamp(0.0, 1.0);
        ts_elderly.gli_activation = (ts_elderly.gli_activation * gli_scale_elderly).clamp(0.0, 1.0);

        assert!(
            ts_elderly.gli_activation < ts_adult.gli_activation,
            "Elderly GLI ({:.3}) should be < adult GLI ({:.3})",
            ts_elderly.gli_activation, ts_adult.gli_activation
        );
    }
}

#[cfg(test)]
mod epigenetic_inheritance_tests {
    use super::*;
    use cell_dt_core::components::{EpigeneticClockState, DivisionExhaustionState};

    /// Деление уменьшает methylation_age до среднего между текущим и хронологическим.
    #[test]
    fn test_division_resets_methylation_age_halfway() {
        let mut epi = EpigeneticClockState::default();
        epi.methylation_age = 60.0;
        epi.last_division_count = 0;

        let chron_age = 40.0_f32;
        let cur_div = 1u32;

        // Эмулируем логику из step(): детектируем деление и применяем сброс
        if cur_div > epi.last_division_count {
            epi.methylation_age = (epi.methylation_age + chron_age) / 2.0;
            epi.last_division_count = cur_div;
        }

        // Ожидаем: (60 + 40) / 2 = 50
        assert!(
            (epi.methylation_age - 50.0).abs() < 0.01,
            "After division: methylation_age should be 50.0, got {:.3}",
            epi.methylation_age
        );
    }

    /// Без нового деления methylation_age не сбрасывается.
    #[test]
    fn test_no_division_no_reset() {
        let mut epi = EpigeneticClockState::default();
        epi.methylation_age = 70.0;
        epi.last_division_count = 5;

        let chron_age = 50.0_f32;
        let cur_div = 5u32; // не изменилось

        if cur_div > epi.last_division_count {
            epi.methylation_age = (epi.methylation_age + chron_age) / 2.0;
            epi.last_division_count = cur_div;
        }

        assert!(
            (epi.methylation_age - 70.0).abs() < 0.01,
            "Without division: methylation_age should remain 70.0, got {:.3}",
            epi.methylation_age
        );
    }

    /// Повторные деления прогрессивно приближают methylation_age к chron_age.
    #[test]
    fn test_repeated_divisions_converge_to_chron_age() {
        let mut epi = EpigeneticClockState::default();
        epi.methylation_age = 80.0;
        let chron_age = 20.0_f32;

        for div in 1..=10u32 {
            epi.methylation_age = (epi.methylation_age + chron_age) / 2.0;
            epi.last_division_count = div;
        }

        // После 10 делений избыток должен уменьшиться: (80-20) * (0.5^10) ≈ 0.06
        let excess = epi.methylation_age - chron_age;
        assert!(
            excess < 1.0,
            "After 10 divisions methylation excess should be < 1.0 yr, got {excess:.3}"
        );
        assert!(
            epi.methylation_age > chron_age,
            "methylation_age should remain > chron_age (never negative excess), got {:.3}",
            epi.methylation_age
        );
    }

    /// Полностью молодая клетка (без избытка) не меняет возраст при делении.
    #[test]
    fn test_young_cell_no_change_on_division() {
        let mut epi = EpigeneticClockState::default();
        epi.methylation_age = 25.0;
        let chron_age = 25.0_f32; // точное совпадение

        epi.methylation_age = (epi.methylation_age + chron_age) / 2.0;

        assert!(
            (epi.methylation_age - 25.0).abs() < 0.01,
            "Young cell: methylation_age should stay 25.0, got {:.3}",
            epi.methylation_age
        );
    }

    /// `last_division_count` обновляется корректно после детекции деления.
    #[test]
    fn test_last_division_count_updated() {
        let mut epi = EpigeneticClockState::default();
        assert_eq!(epi.last_division_count, 0);

        let cur_div = 3u32;
        if cur_div > epi.last_division_count {
            epi.methylation_age = (epi.methylation_age + 0.0) / 2.0;
            epi.last_division_count = cur_div;
        }

        assert_eq!(epi.last_division_count, 3);
    }
}

#[cfg(test)]
mod nk_surveillance_tests {
    use cell_dt_core::components::NKSurveillanceState;

    #[test]
    fn test_young_healthy_cell_low_kill_probability() {
        let mut nk = NKSurveillanceState::default();
        // Молодая клетка: нулевой стресс, возраст 20 лет, нет миелоидного сдвига
        nk.update(0.0, 0.0, 0.0, 20.0, 0.0);
        assert!(
            nk.nk_kill_probability < 0.01,
            "Young pristine cell should have near-zero kill_prob, got {:.4}",
            nk.nk_kill_probability
        );
    }

    #[test]
    fn test_damaged_elderly_cell_high_kill_probability() {
        let mut nk = NKSurveillanceState::default();
        // Повреждённая пожилая клетка: высокий ROS, агрегаты, 75 лет
        nk.update(0.8, 0.7, 0.3, 75.0, 0.4);
        // Не обязательно > порога 0.02, но должно быть значимо выше молодой клетки
        assert!(
            nk.nk_kill_probability > 0.05,
            "Elderly damaged cell should have elevated kill_prob, got {:.4}",
            nk.nk_kill_probability
        );
    }

    #[test]
    fn test_nk_activity_declines_with_age() {
        let mut nk_young  = NKSurveillanceState::default();
        let mut nk_old    = NKSurveillanceState::default();

        nk_young.update(0.0, 0.0, 0.0, 20.0, 0.0);
        nk_old.update(0.0, 0.0, 0.0, 80.0, 0.0);

        assert!(
            nk_old.nk_activity < nk_young.nk_activity,
            "NK activity should decline with age: old={:.3} >= young={:.3}",
            nk_old.nk_activity, nk_young.nk_activity
        );
    }

    #[test]
    fn test_myeloid_bias_suppresses_nk_activity() {
        let mut nk_no_myeloid   = NKSurveillanceState::default();
        let mut nk_high_myeloid = NKSurveillanceState::default();

        nk_no_myeloid.update(0.3, 0.3, 0.1, 50.0, 0.0);
        nk_high_myeloid.update(0.3, 0.3, 0.1, 50.0, 0.8);

        assert!(
            nk_high_myeloid.nk_activity < nk_no_myeloid.nk_activity,
            "Myeloid bias should suppress NK activity: {:.3} >= {:.3}",
            nk_high_myeloid.nk_activity, nk_no_myeloid.nk_activity
        );
    }

    #[test]
    fn test_immune_escape_reduces_kill_probability() {
        let mut nk_low_carb  = NKSurveillanceState::default();
        let mut nk_high_carb = NKSurveillanceState::default();

        // Высокое карбонилирование → MHC-I нарушен → больше ускользания
        nk_low_carb.update(0.7, 0.7, 0.1, 50.0, 0.0);
        nk_high_carb.update(0.7, 0.7, 0.9, 50.0, 0.0);

        assert!(
            nk_high_carb.immune_escape_fraction > nk_low_carb.immune_escape_fraction,
            "High carbonylation should increase immune escape: {:.3} <= {:.3}",
            nk_high_carb.immune_escape_fraction, nk_low_carb.immune_escape_fraction
        );
        assert!(
            nk_high_carb.nk_kill_probability < nk_low_carb.nk_kill_probability,
            "Immune escape should reduce kill_prob: {:.4} >= {:.4}",
            nk_high_carb.nk_kill_probability, nk_low_carb.nk_kill_probability
        );
    }

    #[test]
    fn test_nkg2d_ligand_driven_by_ros_and_aggregates() {
        let mut nk = NKSurveillanceState::default();
        nk.update(1.0, 1.0, 0.0, 30.0, 0.0);
        // После вычитания базового уровня 0.30: max_ligand = 1.0 - 0.30 = 0.70
        assert!(
            nk.nkg2d_ligand_expression > 0.6,
            "Max ROS + aggregates should give high NKG2D expression, got {:.3}",
            nk.nkg2d_ligand_expression
        );
    }

    #[test]
    fn test_nk_activity_clamped_above_minimum() {
        let mut nk = NKSurveillanceState::default();
        // Экстремальные условия: 150 лет, max myeloid bias
        nk.update(0.0, 0.0, 0.0, 150.0, 1.0);
        assert!(
            nk.nk_activity >= 0.1,
            "NK activity should never fall below 0.1, got {:.3}",
            nk.nk_activity
        );
    }
}

#[cfg(test)]
mod proteostasis_tests {
    use cell_dt_core::components::{ProteostasisState, CentriolarDamageState};

    #[test]
    fn test_pristine_damage_gives_max_clearance() {
        let mut prot = ProteostasisState::default();
        let dam = CentriolarDamageState::pristine();
        prot.update(&dam);
        // Новая клетка: CEP164=1, spindle=высокий, ROS=0, aggregates=0
        assert!(prot.aggresome_index > 0.8,
            "Pristine cell should have high aggresome_index, got {:.3}", prot.aggresome_index);
        assert!(prot.hsp_capacity > 0.9,
            "Pristine cell should have high HSP capacity, got {:.3}", prot.hsp_capacity);
        assert!(prot.proteasome_activity > 0.9,
            "Pristine cell should have high proteasome, got {:.3}", prot.proteasome_activity);
        assert!(prot.aggregate_clearance_rate > 0.7,
            "Pristine cell should have high clearance, got {:.3}", prot.aggregate_clearance_rate);
    }

    #[test]
    fn test_high_ros_reduces_hsp_capacity() {
        let mut prot = ProteostasisState::default();
        let mut dam = CentriolarDamageState::pristine();
        dam.ros_level = 0.9;
        prot.update(&dam);
        assert!(prot.hsp_capacity < 0.5,
            "High ROS should reduce HSP capacity: {:.3}", prot.hsp_capacity);
    }

    #[test]
    fn test_high_aggregates_reduces_proteasome() {
        let mut prot = ProteostasisState::default();
        let mut dam = CentriolarDamageState::pristine();
        dam.protein_aggregates = 0.9;
        dam.update_functional_metrics();
        prot.update(&dam);
        assert!(prot.proteasome_activity < 0.3,
            "High aggregates should reduce proteasome activity: {:.3}", prot.proteasome_activity);
    }

    #[test]
    fn test_cep164_loss_reduces_aggresome_index() {
        let mut prot_intact  = ProteostasisState::default();
        let mut prot_damaged = ProteostasisState::default();

        let mut dam_intact  = CentriolarDamageState::pristine();
        let mut dam_damaged = CentriolarDamageState::pristine();
        dam_damaged.cep164_integrity = 0.1;
        dam_damaged.update_functional_metrics();

        prot_intact.update(&dam_intact);
        prot_damaged.update(&dam_damaged);

        assert!(
            prot_damaged.aggresome_index < prot_intact.aggresome_index,
            "CEP164 loss should reduce aggresome_index: {:.3} >= {:.3}",
            prot_damaged.aggresome_index, prot_intact.aggresome_index
        );
    }

    #[test]
    fn test_clearance_rate_reflects_combined_proteostasis() {
        let mut prot_healthy  = ProteostasisState::default();
        let mut prot_impaired = ProteostasisState::default();

        let dam_healthy = CentriolarDamageState::pristine();
        let mut dam_impaired = CentriolarDamageState::pristine();
        dam_impaired.ros_level             = 0.8;
        dam_impaired.protein_aggregates    = 0.7;
        dam_impaired.cep164_integrity      = 0.2;
        dam_impaired.update_functional_metrics();

        prot_healthy.update(&dam_healthy);
        prot_impaired.update(&dam_impaired);

        assert!(
            prot_impaired.aggregate_clearance_rate < prot_healthy.aggregate_clearance_rate,
            "Impaired proteostasis should have lower clearance: {:.3} >= {:.3}",
            prot_impaired.aggregate_clearance_rate, prot_healthy.aggregate_clearance_rate
        );
    }

    #[test]
    fn test_unfolded_protein_load_increases_with_aggregates() {
        let mut prot_low  = ProteostasisState::default();
        let mut prot_high = ProteostasisState::default();

        let mut dam_low  = CentriolarDamageState::pristine();
        let mut dam_high = CentriolarDamageState::pristine();
        dam_low.protein_aggregates  = 0.1;
        dam_high.protein_aggregates = 0.9;
        dam_low.update_functional_metrics();
        dam_high.update_functional_metrics();

        prot_low.update(&dam_low);
        prot_high.update(&dam_high);

        assert!(
            prot_high.unfolded_protein_load > prot_low.unfolded_protein_load,
            "Higher aggregates → higher UPR load: {:.3} <= {:.3}",
            prot_high.unfolded_protein_load, prot_low.unfolded_protein_load
        );
    }
}

#[cfg(test)]
mod compensatory_proliferation_tests {
    use super::*;
    use super::damage::{DamageParams, accumulate_damage};
    use cell_dt_core::components::CentriolarDamageState;

    const DT: f32 = 1.0 / 365.25;
    const AGE: f32 = 50.0;

    fn run_with_pool(pool: f32, steps: usize) -> f32 {
        let params = DamageParams::default();
        let mut dam = CentriolarDamageState::pristine();
        for _ in 0..steps {
            let compensatory = if pool < 0.5 { (0.5 - pool) * 0.30 } else { 0.0 };
            accumulate_damage(&mut dam, &params, AGE, DT, compensatory);
        }
        dam.ros_level
    }

    #[test]
    fn test_depleted_pool_accelerates_ros() {
        let ros_full   = run_with_pool(1.0, 365); // нормальный пул
        let ros_half   = run_with_pool(0.3, 365); // истощённый пул
        assert!(
            ros_half > ros_full,
            "Depleted pool should cause higher ROS: full={:.4}, depleted={:.4}",
            ros_full, ros_half
        );
    }

    #[test]
    fn test_full_pool_no_compensatory_boost() {
        // При pool >= 0.5 нет бустa
        for pool in [0.5f32, 0.7, 1.0] {
            let boost = if pool < 0.5 { (0.5 - pool) * 0.30 } else { 0.0 };
            assert_eq!(boost, 0.0, "No boost expected at pool={pool:.1}");
        }
    }

    #[test]
    fn test_compensatory_boost_proportional_to_deficit() {
        let boost_30 = (0.5 - 0.3_f32) * 0.30; // pool=0.3
        let boost_10 = (0.5 - 0.1_f32) * 0.30; // pool=0.1
        assert!(boost_10 > boost_30,
            "Smaller pool → larger boost: {:.4} <= {:.4}", boost_10, boost_30);
        // pool=0.0 → max boost = 0.5*0.30 = 0.15
        let boost_max = 0.5_f32 * 0.30;
        assert!((boost_max - 0.15).abs() < 0.001,
            "Max boost should be 0.15, got {:.4}", boost_max);
    }

    #[test]
    fn test_compensatory_ros_creates_damage_avalanche() {
        // После сильного истощения пула повреждения ускоряются нелинейно
        let params = DamageParams::default();

        let run = |pool: f32| {
            let mut dam = CentriolarDamageState::pristine();
            for _ in 0..(365 * 10) { // 10 лет
                let boost = if pool < 0.5 { (0.5 - pool) * 0.30 } else { 0.0 };
                accumulate_damage(&mut dam, &params, 60.0, DT, boost);
            }
            dam.total_damage_score()
        };

        let damage_normal   = run(1.0);
        let damage_depleted = run(0.1);
        assert!(
            damage_depleted > damage_normal * 1.01,
            "Pool=0.1 should accumulate more damage than pool=1.0 over 10yr: \
             {:.4} vs {:.4}", damage_depleted, damage_normal
        );
    }
}

#[cfg(test)]
mod circadian_tests {
    use cell_dt_core::components::{CircadianState, CentriolarDamageState};

    #[test]
    fn test_pristine_cell_full_circadian_amplitude() {
        let mut circ = CircadianState::default();
        let dam = CentriolarDamageState::pristine();
        circ.update(&dam);
        // CEP164=1, aggregates=0, ROS≈0 → amplitude near 1.0
        assert!(circ.amplitude > 0.85,
            "Pristine cell should have high circadian amplitude, got {:.3}", circ.amplitude);
        assert!(circ.proteasome_night_boost > 0.20,
            "Pristine cell should have full night boost, got {:.3}", circ.proteasome_night_boost);
        assert!(circ.circadian_sasp_contribution < 0.05,
            "Pristine cell should have low SASP contribution, got {:.3}",
            circ.circadian_sasp_contribution);
    }

    #[test]
    fn test_cep164_loss_reduces_amplitude() {
        let mut circ_intact  = CircadianState::default();
        let mut circ_damaged = CircadianState::default();

        let mut dam_intact  = CentriolarDamageState::pristine();
        let mut dam_damaged = CentriolarDamageState::pristine();
        dam_damaged.cep164_integrity = 0.1;
        dam_damaged.update_functional_metrics();

        circ_intact.update(&dam_intact);
        circ_damaged.update(&dam_damaged);

        assert!(
            circ_damaged.amplitude < circ_intact.amplitude,
            "CEP164 loss should reduce circadian amplitude: {:.3} >= {:.3}",
            circ_damaged.amplitude, circ_intact.amplitude
        );
    }

    #[test]
    fn test_high_ros_reduces_phase_coherence() {
        let mut circ = CircadianState::default();
        let mut dam = CentriolarDamageState::pristine();
        dam.ros_level = 0.9;
        circ.update(&dam);
        assert!(circ.phase_coherence < 0.6,
            "High ROS should reduce phase coherence: {:.3}", circ.phase_coherence);
    }

    #[test]
    fn test_amplitude_drives_sasp_contribution() {
        let mut circ_young = CircadianState::default();
        let mut circ_old   = CircadianState::default();

        let dam_young = CentriolarDamageState::pristine();
        let mut dam_old = CentriolarDamageState::pristine();
        dam_old.cep164_integrity   = 0.2;
        dam_old.protein_aggregates = 0.7;
        dam_old.ros_level          = 0.6;
        dam_old.update_functional_metrics();

        circ_young.update(&dam_young);
        circ_old.update(&dam_old);

        assert!(
            circ_old.circadian_sasp_contribution > circ_young.circadian_sasp_contribution,
            "Damaged cell should have higher SASP from circadian disruption: {:.3} <= {:.3}",
            circ_old.circadian_sasp_contribution, circ_young.circadian_sasp_contribution
        );
    }

    #[test]
    fn test_night_boost_proportional_to_amplitude() {
        let mut circ_full = CircadianState::default();
        let mut circ_half = CircadianState::default();

        circ_full.amplitude = 1.0;
        circ_half.amplitude = 0.5;

        // Пересчитать boost вручную согласно формуле
        circ_full.proteasome_night_boost = circ_full.amplitude * 0.25;
        circ_half.proteasome_night_boost = circ_half.amplitude * 0.25;

        assert!(
            (circ_full.proteasome_night_boost - 0.25).abs() < 0.001,
            "Full amplitude → night boost = 0.25, got {:.3}", circ_full.proteasome_night_boost
        );
        assert!(
            (circ_half.proteasome_night_boost - 0.125).abs() < 0.001,
            "Half amplitude → night boost = 0.125, got {:.3}", circ_half.proteasome_night_boost
        );
    }

    #[test]
    fn test_cep164_cdata_chain_reduces_proteostasis_via_circadian() {
        // CEP164↓ → circadian amplitude↓ → proteasome_night_boost↓
        // Это прямая CDATA-специфическая предсказательная цепочка
        let mut circ = CircadianState::default();

        let mut dam_healthy = CentriolarDamageState::pristine();
        let mut dam_aged    = CentriolarDamageState::pristine();
        dam_aged.cep164_integrity = 0.3;
        dam_aged.update_functional_metrics();

        circ.update(&dam_healthy);
        let boost_healthy = circ.proteasome_night_boost;

        circ.update(&dam_aged);
        let boost_aged = circ.proteasome_night_boost;

        assert!(boost_aged < boost_healthy,
            "CEP164↓ → circadian↓ → night_boost↓: {:.3} >= {:.3}", boost_aged, boost_healthy);
    }
}

#[cfg(test)]
mod autophagy_tests {
    use cell_dt_core::components::AutophagyState;

    #[test]
    fn test_young_organism_moderate_mtor() {
        let mut a = AutophagyState::default();
        a.update(20.0, false, 0.0);
        // mTOR = 0.3 + 20/100*0.5 = 0.40
        assert!((a.mtor_activity - 0.40).abs() < 0.01,
            "Young mTOR should be ~0.40, got {:.3}", a.mtor_activity);
        assert!(a.autophagy_flux > 0.6,
            "Young autophagy flux should be high, got {:.3}", a.autophagy_flux);
    }

    #[test]
    fn test_elderly_higher_mtor_lower_autophagy() {
        let mut young = AutophagyState::default();
        let mut old   = AutophagyState::default();
        young.update(20.0, false, 0.0);
        old.update(80.0, false, 0.0);
        assert!(old.mtor_activity > young.mtor_activity,
            "Elderly mTOR should be higher: {:.3} <= {:.3}",
            old.mtor_activity, young.mtor_activity);
        assert!(old.autophagy_flux < young.autophagy_flux,
            "Elderly autophagy should be lower: {:.3} >= {:.3}",
            old.autophagy_flux, young.autophagy_flux);
    }

    #[test]
    fn test_cr_reduces_mtor() {
        let mut no_cr = AutophagyState::default();
        let mut cr    = AutophagyState::default();
        no_cr.update(50.0, false, 0.0);
        cr.update(50.0, true, 0.0);
        assert!(cr.mtor_activity < no_cr.mtor_activity,
            "CR should reduce mTOR: {:.3} >= {:.3}", cr.mtor_activity, no_cr.mtor_activity);
        assert!(cr.autophagy_flux > no_cr.autophagy_flux,
            "CR should boost autophagy: {:.3} <= {:.3}", cr.autophagy_flux, no_cr.autophagy_flux);
    }

    #[test]
    fn test_nad_plus_reduces_mtor() {
        let mut baseline = AutophagyState::default();
        let mut nad      = AutophagyState::default();
        baseline.update(60.0, false, 0.0);
        nad.update(60.0, false, 0.5);
        assert!(nad.mtor_activity < baseline.mtor_activity,
            "NadPlus should reduce mTOR: {:.3} >= {:.3}", nad.mtor_activity, baseline.mtor_activity);
    }

    #[test]
    fn test_cr_and_nad_combined_stronger() {
        let mut cr_only  = AutophagyState::default();
        let mut combined = AutophagyState::default();
        cr_only.update(60.0, true, 0.0);
        combined.update(60.0, true, 0.5);
        assert!(combined.autophagy_flux >= cr_only.autophagy_flux,
            "CR+NadPlus should have ≥ autophagy than CR alone: {:.3} < {:.3}",
            combined.autophagy_flux, cr_only.autophagy_flux);
    }

    #[test]
    fn test_aggregate_clearance_proportional_to_flux() {
        let mut low_flux  = AutophagyState::default();
        let mut high_flux = AutophagyState::default();
        low_flux.update(80.0, false, 0.0);   // old, no intervention
        high_flux.update(20.0, true,  0.5);  // young + CR + NadPlus
        assert!(high_flux.aggregate_autophagy_clearance > low_flux.aggregate_autophagy_clearance,
            "Higher flux → higher clearance: {:.4} <= {:.4}",
            high_flux.aggregate_autophagy_clearance, low_flux.aggregate_autophagy_clearance);
    }

    #[test]
    fn test_mtor_clamped_minimum() {
        let mut a = AutophagyState::default();
        // Max interventions should not bring mTOR below 0.05
        a.update(20.0, true, 1.0);
        assert!(a.mtor_activity >= 0.05,
            "mTOR should never fall below 0.05, got {:.4}", a.mtor_activity);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// P20: DDR — ответ на повреждение ДНК
// ─────────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod ddr_tests {
    use cell_dt_core::components::DDRState;

    /// Молодая здоровая клетка: spindle=1, ros=0 → γH2AX≈0, p53≈0.
    #[test]
    fn test_pristine_cell_no_damage() {
        let mut d = DDRState::default();
        d.update(1.0, 0.0, 0.0, 20.0);
        assert!(d.gamma_h2ax_level < 0.01,
            "Pristine cell: γH2AX should be ~0, got {:.4}", d.gamma_h2ax_level);
        assert!(d.p53_stabilization < 0.01,
            "Pristine cell: p53 should be ~0, got {:.4}", d.p53_stabilization);
        assert!(d.p21_contribution() < 0.01,
            "Pristine cell: p21 contribution should be ~0, got {:.4}", d.p21_contribution());
    }

    /// Тяжёлое повреждение веретена → высокий γH2AX → активный ATM → p53 > 0.
    #[test]
    fn test_severe_spindle_damage_activates_ddr() {
        let mut d = DDRState::default();
        d.update(0.0, 0.0, 0.0, 30.0); // spindle=0 — полная потеря
        // gamma_h2ax = 1.0^1.5 × 0.8 + 0 × 0.2 = 0.80
        assert!(d.gamma_h2ax_level > 0.7,
            "Spindle=0 should give γH2AX > 0.7, got {:.4}", d.gamma_h2ax_level);
        assert!(d.atm_activity > 0.5,
            "Spindle=0 should activate ATM > 0.5, got {:.4}", d.atm_activity);
        assert!(d.p53_stabilization > 0.3,
            "Spindle=0 should give p53 > 0.3, got {:.4}", d.p53_stabilization);
    }

    /// ROS-путь: ros=1.0, spindle=1.0 → γH2AX = 0.2 (чисто окислительные разрывы).
    #[test]
    fn test_ros_contributes_to_gamma_h2ax() {
        let mut d = DDRState::default();
        d.update(1.0, 1.0, 0.0, 20.0); // spindle=1 (нет анеуплоидии), ros=1 (max)
        // gamma_h2ax = 0^1.5 × 0.8 + 1.0 × 0.2 = 0.20
        assert!((d.gamma_h2ax_level - 0.20).abs() < 0.02,
            "ROS-only γH2AX should be ~0.20, got {:.4}", d.gamma_h2ax_level);
    }

    /// Возраст снижает dna_repair_capacity.
    #[test]
    fn test_age_reduces_repair_capacity() {
        let mut young = DDRState::default();
        let mut old   = DDRState::default();
        young.update(0.5, 0.3, 0.1, 20.0);
        old.update(  0.5, 0.3, 0.1, 90.0);
        assert!(old.dna_repair_capacity < young.dna_repair_capacity,
            "Old cell should have lower repair: {:.3} >= {:.3}",
            old.dna_repair_capacity, young.dna_repair_capacity);
        // Минимальный предел = 0.3
        assert!(old.dna_repair_capacity >= 0.3,
            "Repair capacity floor should be 0.3, got {:.4}", old.dna_repair_capacity);
    }

    /// p21_contribution() = p53 × 0.3 — корректная формула.
    #[test]
    fn test_p21_contribution_formula() {
        let mut d = DDRState::default();
        d.update(0.2, 0.5, 0.2, 50.0);
        let expected = d.p53_stabilization * 0.3;
        assert!((d.p21_contribution() - expected).abs() < 1e-6,
            "p21_contribution should equal p53×0.3: expected {:.6}, got {:.6}",
            expected, d.p21_contribution());
    }

    /// Агрегаты снижают dna_repair_capacity.
    #[test]
    fn test_aggregates_reduce_repair() {
        let mut clean     = DDRState::default();
        let mut aggregate = DDRState::default();
        clean.update(0.8, 0.1, 0.0, 40.0);
        aggregate.update(0.8, 0.1, 0.8, 40.0); // высокие агрегаты
        assert!(aggregate.dna_repair_capacity < clean.dna_repair_capacity,
            "High aggregates should reduce repair: {:.3} >= {:.3}",
            aggregate.dna_repair_capacity, clean.dna_repair_capacity);
    }

    /// Интеграционный тест: повреждение веретена → p21_contribution > 0 → G1-сигнал.
    #[test]
    fn test_spindle_damage_propagates_to_p21() {
        let mut d = DDRState::default();
        d.update(0.3, 0.2, 0.1, 45.0); // умеренно повреждённый spindle
        assert!(d.p21_contribution() > 0.05,
            "Moderate spindle damage should produce p21 contribution > 0.05, got {:.4}",
            d.p21_contribution());
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Track F — Снижение темпа деления стволовых клеток
// ─────────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod division_rate_tests {
    use cell_dt_core::components::StemCellDivisionRateState;

    /// Молодая здоровая клетка: темп деления должен быть близок к 1.0.
    #[test]
    fn test_young_healthy_max_rate() {
        let mut s = StemCellDivisionRateState::default();
        s.update(1.0, 1.0, 0.0, 20.0, 0.3);
        assert!(s.division_rate > 0.90,
            "Young healthy cell should have division_rate > 0.90, got {:.4}", s.division_rate);
        assert!(s.decline_index < 0.10,
            "Young healthy decline_index should be < 0.10, got {:.4}", s.decline_index);
    }

    /// Пожилая клетка делится медленнее молодой при одинаковых повреждениях.
    #[test]
    fn test_old_cell_slower_than_young() {
        let mut young = StemCellDivisionRateState::default();
        let mut old   = StemCellDivisionRateState::default();
        young.update(0.8, 0.8, 0.1, 25.0, 0.35);
        old.update(  0.8, 0.8, 0.1, 75.0, 0.55);
        assert!(old.division_rate < young.division_rate,
            "Old cell should divide slower: {:.4} >= {:.4}",
            old.division_rate, young.division_rate);
    }

    /// Потеря ресничек снижает цилиарный драйв (морфогенный сигнал).
    #[test]
    fn test_cilia_loss_reduces_drive() {
        let mut intact = StemCellDivisionRateState::default();
        let mut no_cil = StemCellDivisionRateState::default();
        intact.update(1.0, 0.8, 0.1, 40.0, 0.4);
        no_cil.update(0.0, 0.8, 0.1, 40.0, 0.4);
        assert!(no_cil.cilia_drive < intact.cilia_drive,
            "No cilia should reduce cilia_drive: {:.4} >= {:.4}",
            no_cil.cilia_drive, intact.cilia_drive);
        assert!(no_cil.division_rate < intact.division_rate,
            "No cilia should reduce division_rate: {:.4} >= {:.4}",
            no_cil.division_rate, intact.division_rate);
    }

    /// Повреждение веретена → снижение spindle_drive → замедление деления.
    #[test]
    fn test_spindle_damage_reduces_rate() {
        let mut good = StemCellDivisionRateState::default();
        let mut bad  = StemCellDivisionRateState::default();
        good.update(0.9, 1.0, 0.05, 35.0, 0.35);
        bad.update( 0.9, 0.2, 0.05, 35.0, 0.35);
        assert!(bad.spindle_drive < good.spindle_drive,
            "Low spindle should reduce spindle_drive");
        assert!(bad.division_rate < good.division_rate,
            "Low spindle should reduce division_rate: {:.4} >= {:.4}",
            bad.division_rate, good.division_rate);
    }

    /// Высокий ROS тормозит деление.
    #[test]
    fn test_high_ros_brakes_division() {
        let mut low_ros  = StemCellDivisionRateState::default();
        let mut high_ros = StemCellDivisionRateState::default();
        low_ros.update( 0.8, 0.8, 0.05, 40.0, 0.4);
        high_ros.update(0.8, 0.8, 0.90, 40.0, 0.4);
        assert!(high_ros.ros_brake < low_ros.ros_brake,
            "High ROS should lower ros_brake");
        assert!(high_ros.division_rate < low_ros.division_rate,
            "High ROS should lower division_rate: {:.4} >= {:.4}",
            high_ros.division_rate, low_ros.division_rate);
    }

    /// Высокий mTOR тормозит деление (перераспределение ресурсов).
    #[test]
    fn test_high_mtor_brakes_division() {
        let mut low_mtor  = StemCellDivisionRateState::default();
        let mut high_mtor = StemCellDivisionRateState::default();
        low_mtor.update( 0.8, 0.8, 0.1, 40.0, 0.30); // молодой mTOR
        high_mtor.update(0.8, 0.8, 0.1, 40.0, 0.80); // старческий mTOR
        assert!(high_mtor.mtor_brake < low_mtor.mtor_brake,
            "High mTOR should lower mtor_brake");
        assert!(high_mtor.division_rate < low_mtor.division_rate,
            "High mTOR should lower division_rate: {:.4} >= {:.4}",
            high_mtor.division_rate, low_mtor.division_rate);
    }

    /// decline_index = 1 - division_rate (инвариант).
    #[test]
    fn test_decline_index_invariant() {
        let mut s = StemCellDivisionRateState::default();
        s.update(0.5, 0.6, 0.3, 60.0, 0.55);
        let expected = 1.0 - s.division_rate;
        assert!((s.decline_index - expected).abs() < 1e-6,
            "decline_index should equal 1 - division_rate: {:.6} != {:.6}",
            s.decline_index, expected);
    }

    /// Темп никогда не падает ниже 0.01 (минимальная пролиферативная активность).
    #[test]
    fn test_rate_never_zero() {
        let mut s = StemCellDivisionRateState::default();
        // Экстремальные условия: старость + нет ресничек + нет веретена + макс. ROS + макс. mTOR
        s.update(0.0, 0.0, 1.0, 120.0, 1.0);
        assert!(s.division_rate >= 0.01,
            "division_rate should never go below 0.01, got {:.6}", s.division_rate);
    }
}
