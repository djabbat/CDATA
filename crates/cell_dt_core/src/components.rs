use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::Hash;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum Phase {
    G1,
    S,
    G2,
    M,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Default for Position {
    fn default() -> Self {
        Self { x: 0.0, y: 0.0, z: 0.0 }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GeneExpression {
    pub profile: HashMap<String, f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellCycleState {
    pub phase: Phase,
    pub progress: f32,
}

impl Default for CellCycleState {
    fn default() -> Self {
        Self { phase: Phase::G1, progress: 0.0 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PTMProfile {
    pub acetylation_level: f32,
    pub oxidation_level: f32,
    pub methylation_level: f32,
    pub phosphorylation_level: f32,
}

impl Default for PTMProfile {
    fn default() -> Self {
        Self {
            acetylation_level: 0.0,
            oxidation_level: 0.0,
            methylation_level: 0.0,
            phosphorylation_level: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CAFD {
    pub name: String,
    pub activity: f32,
    pub concentration: f32,
}

impl CAFD {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            activity: 0.0,
            concentration: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Centriole {
    pub maturity: f32,
    pub ptm_signature: PTMProfile,
    pub associated_cafds: Vec<CAFD>,
}

impl Centriole {
    pub fn new(maturity: f32) -> Self {
        Self {
            maturity,
            ptm_signature: PTMProfile::default(),
            associated_cafds: Vec::new(),
        }
    }
    
    pub fn new_daughter() -> Self {
        Self::new(0.0)
    }
    
    pub fn new_mature() -> Self {
        Self::new(1.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CentriolePair {
    pub mother: Centriole,
    pub daughter: Centriole,
    pub cilium_present: bool,
    pub mtoc_activity: f32,
}

impl Default for CentriolePair {
    fn default() -> Self {
        Self {
            mother: Centriole::new_mature(),
            daughter: Centriole::new_daughter(),
            cilium_present: false,
            mtoc_activity: 0.5,
        }
    }
}

// Типы для расширенного клеточного цикла
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CyclinType {
    CyclinD,
    CyclinE,
    CyclinA,
    CyclinB,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CdkType {
    Cdk4,
    Cdk6,
    Cdk2,
    Cdk1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Checkpoint {
    G1SRestriction,
    G2MCheckpoint,
    SpindleAssembly,
    DNARepair,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CyclinCdkComplex {
    pub cyclin_type: CyclinType,
    pub cdk_type: CdkType,
    pub activity: f32,
    pub concentration: f32,
    pub phosphorylation_level: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrowthFactors {
    pub growth_signal: f32,
    pub nutrient_level: f32,
    pub stress_level: f32,
    pub dna_damage: f32,
    pub oxidative_stress: f32,
}

impl Default for GrowthFactors {
    fn default() -> Self {
        Self {
            growth_signal: 0.8,
            nutrient_level: 1.0,
            stress_level: 0.0,
            dna_damage: 0.0,
            oxidative_stress: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointState {
    pub checkpoint: Checkpoint,
    pub satisfied: bool,
    pub time_in_checkpoint: f32,
    pub arrest_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellCycleStateExtended {
    pub phase: Phase,
    pub progress: f32,
    pub cyclin_cdk_complexes: Vec<CyclinCdkComplex>,
    pub checkpoints: Vec<CheckpointState>,
    pub current_checkpoint: Option<Checkpoint>,
    pub growth_factors: GrowthFactors,
    pub cycle_count: u32,
    pub time_in_current_phase: f32,
    pub total_time: f32,
    pub centriole_influence: f32,
}

impl CellCycleStateExtended {
    /// Создать клетку в фазе G1 (начало цикла).
    ///
    /// **Обязательный компонент при спавне сущностей.**
    /// Большинство модулей обнаруживают управляемые ими сущности именно по наличию
    /// `CellCycleStateExtended`. При спавне новой сущности всегда включайте этот компонент
    /// первым, затем остальные:
    ///
    /// ```rust,ignore
    /// world.spawn((
    ///     CellCycleStateExtended::new(),   // ← обязателен
    ///     CentriolarDamageState::pristine(),
    ///     // ... остальные компоненты
    /// ));
    /// ```
    pub fn new() -> Self {
        Self {
            phase: Phase::G1,
            progress: 0.0,
            cyclin_cdk_complexes: Vec::new(),
            checkpoints: Vec::new(),
            current_checkpoint: None,
            growth_factors: GrowthFactors::default(),
            cycle_count: 0,
            time_in_current_phase: 0.0,
            total_time: 0.0,
            centriole_influence: 0.0,
        }
    }
}

impl Default for CellCycleStateExtended {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================
// CDATA — Centriolar Damage Accumulation Theory of Aging
// Компоненты для моделирования полного жизненного цикла
// ============================================================

/// Стадии развития организма (от зиготы до смерти)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DevelopmentalStage {
    /// Оплодотворённая яйцеклетка — нет центриолей, тотипотентность
    Zygote,
    /// 2–16 клеток, де-ново формирование центриолей
    Cleavage,
    /// Бластоциста — ВКМ (плюрипотентные) vs трофобласт
    Blastocyst,
    /// Гаструляция — три зародышевых листка
    Gastrulation,
    /// Нейруляция и органогенез
    Organogenesis,
    /// Плодный период
    Fetal,
    /// Постнатальный рост и развитие (0–18 лет)
    Postnatal,
    /// Взрослый организм (18–40 лет) — гомеостаз тканей
    Adult,
    /// Средний возраст (40–65 лет) — начало накопления повреждений
    MiddleAge,
    /// Пожилой (65+ лет) — выраженное старение
    Senescent,
    /// Смерть организма
    Death,
}

impl DevelopmentalStage {
    /// Возраст (в годах) начала стадии
    pub fn age_start_years(&self) -> f64 {
        match self {
            DevelopmentalStage::Zygote        => 0.0,
            DevelopmentalStage::Cleavage      => 0.0,
            DevelopmentalStage::Blastocyst    => 0.0,
            DevelopmentalStage::Gastrulation  => 0.0,
            DevelopmentalStage::Organogenesis => 0.0,
            DevelopmentalStage::Fetal         => 0.0,
            DevelopmentalStage::Postnatal     => 0.0,
            DevelopmentalStage::Adult         => 18.0,
            DevelopmentalStage::MiddleAge     => 40.0,
            DevelopmentalStage::Senescent     => 65.0,
            DevelopmentalStage::Death         => 80.0,
        }
    }

    /// Следующая стадия развития
    pub fn next(&self) -> Option<DevelopmentalStage> {
        match self {
            DevelopmentalStage::Zygote        => Some(DevelopmentalStage::Cleavage),
            DevelopmentalStage::Cleavage      => Some(DevelopmentalStage::Blastocyst),
            DevelopmentalStage::Blastocyst    => Some(DevelopmentalStage::Gastrulation),
            DevelopmentalStage::Gastrulation  => Some(DevelopmentalStage::Organogenesis),
            DevelopmentalStage::Organogenesis => Some(DevelopmentalStage::Fetal),
            DevelopmentalStage::Fetal         => Some(DevelopmentalStage::Postnatal),
            DevelopmentalStage::Postnatal     => Some(DevelopmentalStage::Adult),
            DevelopmentalStage::Adult         => Some(DevelopmentalStage::MiddleAge),
            DevelopmentalStage::MiddleAge     => Some(DevelopmentalStage::Senescent),
            DevelopmentalStage::Senescent     => Some(DevelopmentalStage::Death),
            DevelopmentalStage::Death         => None,
        }
    }
}

/// Комплект индукторов дифференцировки на одной центриоли (CDATA)
///
/// Материнская и дочерняя центриоли имеют **разные** комплекты (M и D).
/// Индукторы отщепляются необратимо при проникновении O₂ к центриолям.
/// Новая центриоль синтезируется с числом индукторов, равным ТЕКУЩЕМУ
/// остатку родительской (не исходному максимуму зиготы).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CentrioleInducerSet {
    /// Текущий остаток индукторов
    pub remaining: u32,
    /// Количество, унаследованное при синтезе (не абсолютный максимум зиготы)
    pub inherited_count: u32,
}

impl CentrioleInducerSet {
    pub fn full(count: u32) -> Self {
        Self { remaining: count, inherited_count: count }
    }

    pub fn empty() -> Self {
        Self { remaining: 0, inherited_count: 0 }
    }

    /// Дочерний комплект: наследует ТЕКУЩИЙ остаток, а не inherited_count.
    pub fn inherit_from(&self) -> Self {
        Self { remaining: self.remaining, inherited_count: self.remaining }
    }

    /// Полный ли комплект относительно наследованного количества?
    pub fn is_full(&self) -> bool {
        self.inherited_count > 0 && self.remaining == self.inherited_count
    }

    pub fn has_any(&self) -> bool { self.remaining > 0 }

    /// Необратимо отщепить один индуктор. Возвращает true если был доступен.
    pub fn detach_one(&mut self) -> bool {
        if self.remaining > 0 { self.remaining -= 1; true } else { false }
    }
}

/// Уровень потентности — определяется по состоянию обоих индукторных комплектов.
///
/// Переход происходит через отщепление индукторов при O₂-воздействии на центриоли.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PotencyLevel {
    /// M=полный И D=полный — оба комплекта нетронуты
    Totipotent,
    /// M≥1 И D≥1 — оба непусты, но не оба полные
    Pluripotent,
    /// Одна центриоль пуста, другая содержит ≥2 индуктора
    Oligopotent,
    /// Ровно 1 индуктор на одной центриоли, другая пуста
    Unipotent,
    /// M=0 И D=0 — запущен путь запрограммированного апоптоза
    Apoptosis,
}

/// Параметры отщепления индукторов при O₂-воздействии (для панели управления)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct InducerDetachmentParams {
    /// Базовая вероятность отщепления на шаг при oxygen_level=1.0
    pub base_detach_probability: f32,
    /// Доля вероятности, приходящаяся на материнскую центриоль [0..1]
    /// 0.5 = равновероятно (по умолчанию); >0.5 = материнская теряет чаще
    pub mother_bias: f32,
    /// Коэффициент влияния возраста (лет) на mother_bias.
    /// По умолчанию 0.0 — возраст не является причиной потери индукторов.
    pub age_bias_coefficient: f32,
    /// Масштаб PTM-опосредованного истощения материнского комплекта.
    ///
    /// Второй, независимый от O₂ путь: структурные ПТМ матери ослабляют
    /// связи индукторов. Вероятность = ptm_asymmetry × ptm_exhaustion_scale.
    /// 0.0 → механизм выключен.
    pub ptm_exhaustion_scale: f32,
}

impl Default for InducerDetachmentParams {
    fn default() -> Self {
        Self {
            base_detach_probability: 0.0003,
            mother_bias: 0.5,           // одинаковая вероятность для M и D
            age_bias_coefficient: 0.0,  // возраст не влияет по умолчанию
            ptm_exhaustion_scale: 0.001, // PTM-асимметрия → истощение матери
        }
    }
}

impl InducerDetachmentParams {
    pub fn effective_mother_bias(&self, age_years: f32) -> f32 {
        (self.mother_bias + age_years * self.age_bias_coefficient).min(0.95)
    }
    pub fn mother_prob(&self, oxygen_level: f32, age_years: f32) -> f32 {
        oxygen_level * self.base_detach_probability * self.effective_mother_bias(age_years)
    }
    pub fn daughter_prob(&self, oxygen_level: f32, age_years: f32) -> f32 {
        oxygen_level * self.base_detach_probability * (1.0 - self.effective_mother_bias(age_years))
    }
}

/// Пара индукторных комплектов (материнская + дочерняя центриоль).
///
/// Заменяет устаревший `CentriolarInducers`. Асимметрия дифференцировки
/// возникает из разного остатка комплектов M и D при O₂-отщеплении:
/// материнская центриоль накапливает больше ПТМ → теряет индукторы чаще.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CentriolarInducerPair {
    /// Комплект M: индукторы на материнской центриоли
    pub mother_set: CentrioleInducerSet,
    /// Комплект D: индукторы на дочерней центриоли (другой тип молекул)
    pub daughter_set: CentrioleInducerSet,
    /// Суммарное число делений данной клеточной линии
    pub division_count: u32,
    /// Параметры отщепления (настраиваемые через панель управления)
    pub detachment_params: InducerDetachmentParams,
}

impl CentriolarInducerPair {
    /// Зигота: полные комплекты на обеих центриолях.
    pub fn zygote(mother_max: u32, daughter_max: u32) -> Self {
        Self {
            mother_set: CentrioleInducerSet::full(mother_max),
            daughter_set: CentrioleInducerSet::full(daughter_max),
            division_count: 0,
            detachment_params: InducerDetachmentParams::default(),
        }
    }

    /// Определить уровень потентности по текущему состоянию обоих комплектов.
    pub fn potency_level(&self) -> PotencyLevel {
        let m = self.mother_set.remaining;
        let d = self.daughter_set.remaining;
        match (m, d) {
            (0, 0) => PotencyLevel::Apoptosis,
            (1, 0) | (0, 1) => PotencyLevel::Unipotent,
            (_, 0) | (0, _) => PotencyLevel::Oligopotent,
            _ if self.mother_set.is_full() && self.daughter_set.is_full() => PotencyLevel::Totipotent,
            _ => PotencyLevel::Pluripotent,
        }
    }

    pub fn is_apoptotic(&self) -> bool {
        self.potency_level() == PotencyLevel::Apoptosis
    }

    /// Создать пары для двух дочерних клеток при делении.
    /// Новая дочерняя центриоль синтезируется с ТЕКУЩИМ остатком родительской.
    pub fn divide(&mut self) -> (CentriolarInducerPair, CentriolarInducerPair) {
        self.division_count += 1;
        let cell_a = CentriolarInducerPair {
            mother_set:  self.mother_set.clone(),
            daughter_set: self.mother_set.inherit_from(),
            division_count: 0,
            detachment_params: self.detachment_params,
        };
        let cell_b = CentriolarInducerPair {
            mother_set:  self.daughter_set.clone(),
            daughter_set: self.daughter_set.inherit_from(),
            division_count: 0,
            detachment_params: self.detachment_params,
        };
        (cell_a, cell_b)
    }
}

impl Default for CentriolarInducerPair {
    fn default() -> Self { Self::zygote(10, 8) }
}

/// Состояние повреждений центриоли (CDATA)
///
/// Повреждения накапливаются необратимо в материнской центриоли
/// стволовых клеток на протяжении всей жизни организма.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CentriolarDamageState {
    // --- Молекулярные повреждения ---
    /// Уровень карбонилирования белков (SAS-6, CEP135) — окислительный стресс
    pub protein_carbonylation: f32,
    /// Гиперацетилирование альфа-тубулина (снижение HDAC6/SIRT2) [0..1]
    pub tubulin_hyperacetylation: f32,
    /// Агрегаты белков (CPAP, CEP290) — блокируют аппарат дупликации [0..1]
    pub protein_aggregates: f32,
    /// Нарушение фосфорилирования (PLK4, NEK2, PP1) [0..1]
    pub phosphorylation_dysregulation: f32,

    // --- Потеря дистальных придатков ---
    /// Целостность CEP164 (главный: инициация ресничек) [0..1]
    pub cep164_integrity: f32,
    /// Целостность CEP89 [0..1]
    pub cep89_integrity: f32,
    /// Целостность Ninein (субдистальные придатки, якорение) [0..1]
    pub ninein_integrity: f32,
    /// Целостность CEP170 [0..1]
    pub cep170_integrity: f32,

    // --- Производные функциональные метрики ---
    /// Функциональность первичной реснички [0..1] — зависит от придатков
    pub ciliary_function: f32,
    /// Точность ориентации веретена деления [0..1] — определяет АКД
    pub spindle_fidelity: f32,
    /// Кумулятивный уровень ROS в нише (петля обратной связи)
    pub ros_level: f32,

    /// Общее число делений клетки (счётчик Хейфлика)
    pub total_divisions: u32,
    /// Клетка вошла в сенесценцию?
    pub is_senescent: bool,
    /// Порог total_damage_score для входа в сенесценцию.
    /// Синхронизируется из `DamageParams::senescence_threshold` через `accumulate_damage()`.
    /// По умолчанию: 0.75 (соответствует ~78 годам при нормальном старении).
    pub senescence_threshold: f32,
}

impl CentriolarDamageState {
    /// Новорождённая центриоль (де-ново или в зиготе) — без повреждений
    pub fn pristine() -> Self {
        Self {
            protein_carbonylation: 0.0,
            tubulin_hyperacetylation: 0.0,
            protein_aggregates: 0.0,
            phosphorylation_dysregulation: 0.0,
            cep164_integrity: 1.0,
            cep89_integrity: 1.0,
            ninein_integrity: 1.0,
            cep170_integrity: 1.0,
            ciliary_function: 1.0,
            spindle_fidelity: 1.0,
            ros_level: 0.05,
            total_divisions: 0,
            is_senescent: false,
            senescence_threshold: 0.75,
        }
    }

    /// Обновить производные метрики из молекулярных повреждений
    pub fn update_functional_metrics(&mut self) {
        // Функция реснички — зависит от целостности дистальных придатков
        let appendage_mean = (self.cep164_integrity
            + self.cep89_integrity
            + self.ninein_integrity
            + self.cep170_integrity) / 4.0;
        self.ciliary_function = appendage_mean * (1.0 - self.protein_aggregates * 0.5);

        // Точность веретена — деградирует от карбонилирования и агрегатов
        let structural_damage = (self.protein_carbonylation + self.protein_aggregates) / 2.0;
        self.spindle_fidelity = (1.0 - structural_damage).max(0.0)
            * (1.0 - self.phosphorylation_dysregulation * 0.3);

        // Сенесценция — когда суммарный ущерб превышает настраиваемый порог.
        // Порог синхронизируется из DamageParams::senescence_threshold через accumulate_damage().
        let total_damage = self.total_damage_score();
        if total_damage > self.senescence_threshold {
            self.is_senescent = true;
        }
    }

    /// Суммарный балл повреждений [0..1]
    pub fn total_damage_score(&self) -> f32 {
        let mol_damage = (self.protein_carbonylation
            + self.tubulin_hyperacetylation
            + self.protein_aggregates
            + self.phosphorylation_dysregulation) / 4.0;
        let appendage_loss = 1.0 - (self.cep164_integrity
            + self.cep89_integrity
            + self.ninein_integrity
            + self.cep170_integrity) / 4.0;
        (mol_damage + appendage_loss) / 2.0
    }

    /// Вероятность симметричного деления (оба потомка дифференцируются
    /// ИЛИ оба самообновляются) — растёт по мере снижения spindle_fidelity
    pub fn symmetric_division_probability(&self) -> f32 {
        (1.0 - self.spindle_fidelity).powf(1.5)
    }

    /// Вероятность истощения пула (оба потомка дифференцируются)
    pub fn pool_exhaustion_probability(&self) -> f32 {
        self.symmetric_division_probability() * 0.6
    }

    /// Hill-функция активации GLI через первичную ресничку (Трек A, нелинейный).
    ///
    /// Моделирует реснично-зависимую обработку GLI-транскрипционного фактора
    /// в позвоночных: SMO → кончик реснички → GLI-активатор.
    ///
    /// Формула: `GLI = cilia^n / (K^n + cilia^n)`
    /// - K = 0.5 (EC50, нормализованный): при ciliary_function=0.5 ответ = 50%
    /// - n = 2.0 (коэффициент Хилла): кооперативность → сигмоидный порог
    ///
    /// Биологическое обоснование: переход от линейного к нелинейному (~возраст 50 лет)
    /// соответствует клиническим данным о резком снижении Hedgehog-зависимого
    /// самообновления HSC и нейральных прогениторов (Rohatgi et al., 2007).
    pub fn gli_activation(&self) -> f32 {
        const K: f32 = 0.5; // EC50 нормализованный
        const N: f32 = 2.0; // коэффициент Хилла (кооперативность SMO-GLI)
        let c = self.ciliary_function;
        c.powf(N) / (K.powf(N) + c.powf(N))
    }
}

impl Default for CentriolarDamageState {
    fn default() -> Self {
        Self::pristine()
    }
}

/// Тип ткани для специфики стволовых ниш.
///
/// Объединяет биологические ниши (`Neural`, `Muscle`, …) и
/// тканеспецифичные типы клеток человека (`Liver`, `Kidney`, …).
/// Ранее дублировался как `HumanTissueType` в `human_development_module`
/// — теперь единый тип; `HumanTissueType` является псевдонимом.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TissueType {
    // ── Биологические ниши ────────────────────────────────────────────────
    /// Нейральные стволовые клетки (СВЗ, зубчатая извилина)
    Neural,
    /// Кровь / гемопоэтические стволовые клетки (красный костный мозг)
    Blood,
    /// Эпителиальные ниши (кишечные крипты, кожный базальный слой и т.п.)
    Epithelial,
    /// Мышечные клетки-сателлиты
    Muscle,
    /// Кожный эпителий (базальный слой)
    Skin,
    /// Половые клетки
    Germline,
    // ── Специфичные для человека ─────────────────────────────────────────
    /// Соединительная ткань
    Connective,
    /// Костная ткань
    Bone,
    /// Хрящевая ткань
    Cartilage,
    /// Жировая ткань
    Adipose,
    /// Печень (гепатоциты / звёздчатые клетки)
    Liver,
    /// Почки (тубулярный эпителий)
    Kidney,
    /// Сердечная мышца (кардиомиоциты)
    Heart,
    /// Лёгочный эпителий (альвеолярный тип II)
    Lung,
}

/// Правило центриолярной асимметрии при делении стволовой клетки.
///
/// Определяет, какая из центриолей (материнская или дочерняя) наследуется
/// дочерней клеткой, остающейся в стволовом состоянии.
///
/// Источники: «Centrioles as determinants» (2026-01-27),
///            «Strategic Timekeepers» (2026-01-15).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CentrioleAsymmetryRule {
    /// Материнская центриоль → стволовая дочь (HSC, нейральная радиальная глия,
    /// зародышевые клетки млекопитающих, гепатоциты).
    ///
    /// Следствие для CDATA: материнский комплект (M-set) несёт «историю» ниши
    /// и теряет индукторы быстрее (больше ПТМ, слабее связи) → `mother_bias > 0.5`.
    MotherToStem,
    /// Дочерняя центриоль → стволовая дочь (нейробласты Drosophila).
    ///
    /// Редкое исключение. Для млекопитающих не характерно.
    /// Следствие для CDATA: D-set = стволовой, поэтому D теряет медленнее → `mother_bias < 0.5`.
    DaughterToStem,
    /// Симметричное деление: обе центриоли равнозначны.
    ///
    /// Характерно для эпителия, мышечных сателлитов, кожи.
    /// `mother_bias = 0.5`.
    Symmetric,
}

impl TissueType {
    /// Правило асимметрии центриолей для данного типа ткани.
    pub fn asymmetry_rule(self) -> CentrioleAsymmetryRule {
        use CentrioleAsymmetryRule::*;
        match self {
            // MotherToStem: материнская → долгосрочная стволовая ниша
            TissueType::Blood     => MotherToStem, // LT-HSC наследует мать. (Hinge et al. 2020)
            TissueType::Neural    => MotherToStem, // радиальная глия → нейрогенная нишa
            TissueType::Germline  => MotherToStem, // зародышевые стволовые клетки
            TissueType::Liver     => MotherToStem, // гепатоциты зоны 1 (перипортальные)
            // Symmetric: симметричное деление, оба пула равнозначны
            TissueType::Epithelial   => Symmetric,
            TissueType::Muscle       => Symmetric,
            TissueType::Skin         => Symmetric,
            TissueType::Connective   => Symmetric,
            TissueType::Bone         => Symmetric,
            TissueType::Cartilage    => Symmetric,
            TissueType::Adipose      => Symmetric,
            TissueType::Kidney       => Symmetric,
            TissueType::Heart        => Symmetric,
            TissueType::Lung         => Symmetric,
        }
    }

    /// Ткань-специфичный `mother_bias` для `InducerDetachmentParams`.
    ///
    /// Значение определяется правилом асимметрии:
    /// - `MotherToStem` → 0.65 (мать старше, больше ПТМ, связи слабее → теряет чаще)
    /// - `DaughterToStem` → 0.35 (дочь = стволовая, мать теряет меньше)
    /// - `Symmetric` → 0.50
    pub fn default_mother_bias(self) -> f32 {
        match self.asymmetry_rule() {
            CentrioleAsymmetryRule::MotherToStem   => 0.65,
            CentrioleAsymmetryRule::DaughterToStem => 0.35,
            CentrioleAsymmetryRule::Symmetric      => 0.50,
        }
    }
}

/// Состояние ткани — агрегированные метрики регенеративного потенциала
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TissueState {
    pub tissue_type: TissueType,
    /// Размер пула стволовых клеток (0..1, относительно молодого организма)
    pub stem_cell_pool: f32,
    /// Темп регенерации (0..1, относительно молодого организма).
    /// Вычисляется через Hill-функцию GLI (нелинейно от ciliary_function).
    pub regeneration_tempo: f32,
    /// Доля сенесцентных клеток [0..1]
    pub senescent_fraction: f32,
    /// Средний возраст материнской центриоли в нише (в делениях)
    pub mean_centriole_age: f32,
    /// Функциональная ёмкость ткани [0..1]
    pub functional_capacity: f32,

    // ── Морфогенные поля (P13) ────────────────────────────────────────────
    /// Локальная концентрация Sonic Hedgehog [0..1] в нише.
    /// Продуцируется поддерживающими клетками, потребляется через PTCH1 на реснички.
    /// Снижается с возрастом: cilia loss → нарушение рецепции → градиент размывается.
    pub shh_concentration: f32,
    /// Баланс BMP-активность / Noggin-ингибирование [0..1].
    /// 0.0 = полный Noggin-эффект (стволовость); 1.0 = максимальная BMP (дифференцировка).
    /// С возрастом: cilia loss → Noggin↓ → BMP_balance↑ → дифференцировка↑.
    pub bmp_balance: f32,
    /// Активность Wnt-пути в нише [0..1].
    /// Зависит от stem_cell_pool (больше клеток → больше Wnt-лигандов) и ресничек.
    pub wnt_activity: f32,
    /// Активация GLI (Hedgehog-ответ через ресничку) [0..1].
    /// Hill-нелинейная функция от ciliary_function, вычисляется в update_tissue_state.
    pub gli_activation: f32,
}

impl TissueState {
    pub fn new(tissue_type: TissueType) -> Self {
        Self {
            tissue_type,
            stem_cell_pool: 1.0,
            regeneration_tempo: 1.0,
            senescent_fraction: 0.0,
            mean_centriole_age: 0.0,
            functional_capacity: 1.0,
            // Морфогенные поля — инициализируются "молодым" паттерном
            shh_concentration: 0.8,  // высокий Shh в молодой нише
            bmp_balance: 0.3,        // умеренный BMP (Noggin преобладает)
            wnt_activity: 0.7,       // активный Wnt → самообновление
            gli_activation: 0.8,     // полный GLI-ответ при cilia=1.0
        }
    }

    /// Обновить функциональную ёмкость из текущих метрик
    pub fn update_functional_capacity(&mut self) {
        self.functional_capacity = self.stem_cell_pool
            * self.regeneration_tempo
            * (1.0 - self.senescent_fraction * 0.8);
    }

    /// Обновить морфогенные поля на основе текущего состояния ниши.
    ///
    /// Вызывается из `HumanDevelopmentModule::update_tissue_state()`.
    ///
    /// # Аргументы
    /// * `ciliary_function` — из `CentriolarDamageState`
    /// * `detail_level` — `tissue_detail_level` из `HumanDevelopmentParams`:
    ///   - 1: только GLI-активация (минимум, быстро)
    ///   - 2: GLI + BMP_balance
    ///   - 3+: полный морфогенный профиль (GLI + BMP + Wnt + Shh)
    pub fn update_morphogen_fields(&mut self, ciliary_function: f32, detail_level: usize) {
        // Уровень 1+: GLI-активация (Hedgehog) через Hill-нелинейность реснички.
        // Это основной механизм Трека A — всегда вычисляется.
        const K: f32 = 0.5;
        const N: f32 = 2.0;
        let c = ciliary_function;
        self.gli_activation = c.powf(N) / (K.powf(N) + c.powf(N));

        if detail_level < 2 { return; }

        // Уровень 2+: BMP/Noggin баланс.
        // С потерей ресничек падает Noggin → BMP-активность растёт → дифференцировка↑.
        // Биологически: Noggin-экспрессия в нише зависит от Hedgehog через ресничку.
        let noggin_activity = ciliary_function * 0.7;
        self.bmp_balance = ((1.0 - noggin_activity) * 0.8
            + self.senescent_fraction * 0.2)
            .clamp(0.0, 1.0);

        if detail_level < 3 { return; }

        // Уровень 3+: полный морфогенный профиль (Shh + Wnt).
        // Shh: продукция ∝ пулу стволовых клеток + вкладу Wnt
        self.shh_concentration = (self.stem_cell_pool * 0.8
            + self.wnt_activity * 0.2)
            .clamp(0.0, 1.0);

        // Wnt: синергия с GLI (Wnt-Hedgehog crosstalk в нише HSC/NSC)
        self.wnt_activity = (self.stem_cell_pool * 0.6
            + self.gli_activation * 0.4)
            .clamp(0.0, 1.0);
    }
}

/// Глобальное состояние организма (уровень организм/особь)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganismState {
    /// Возраст в годах
    pub age_years: f64,
    /// Текущая стадия развития
    pub developmental_stage: DevelopmentalStage,
    /// Накопленный уровень системного воспаления (inflammaging) [0..1]
    pub inflammaging_score: f32,
    /// Интегральный индекс дряхлости [0..1]
    pub frailty_index: f32,
    /// Когнитивный индекс [0..1]
    pub cognitive_index: f32,
    /// Иммунный резерв [0..1]
    pub immune_reserve: f32,
    /// Мышечная масса (саркопения) [0..1]
    pub muscle_mass: f32,
    /// Жив ли организм
    pub is_alive: bool,
    /// Уровень ИФР-1/ГР (ось IGF-1/GH) [0..1].
    /// Пик в ~20 лет, линейное снижение до 0.3 к 90 годам.
    /// Влияет на `regeneration_tempo` всех тканей.
    pub igf1_level: f32,
    /// Уровень системного SASP [0..1] — среднее sasp_output всех ниш.
    /// Паракринный сигнал: ускоряет повреждения соседних тканей.
    pub systemic_sasp: f32,
}

impl OrganismState {
    pub fn new() -> Self {
        Self {
            age_years: 0.0,
            developmental_stage: DevelopmentalStage::Zygote,
            inflammaging_score: 0.0,
            frailty_index: 0.0,
            cognitive_index: 1.0,
            immune_reserve: 1.0,
            muscle_mass: 1.0,
            is_alive: true,
            igf1_level: 1.0,
            systemic_sasp: 0.0,
        }
    }
}

impl Default for OrganismState {
    fn default() -> Self {
        Self::new()
    }
}

/// Маркер мёртвой сущности.
///
/// Вставляется модулями (например, `human_development_module`) при гибели клетки.
/// `SimulationManager::cleanup_dead_entities()` периодически удаляет все сущности
/// с этим компонентом из ECS-мира.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Dead;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phase_enum() {
        let phases = vec![Phase::G1, Phase::S, Phase::G2, Phase::M];
        assert_eq!(phases.len(), 4);
    }

    #[test]
    fn test_centriole_creation() {
        let mother = Centriole::new_mature();
        let daughter = Centriole::new_daughter();
        
        assert_eq!(mother.maturity, 1.0);
        assert_eq!(daughter.maturity, 0.0);
        assert_eq!(mother.associated_cafds.len(), 0);
    }

    #[test]
    fn test_centriole_pair_default() {
        let pair = CentriolePair::default();
        assert_eq!(pair.mother.maturity, 1.0);
        assert_eq!(pair.daughter.maturity, 0.0);
        assert_eq!(pair.mtoc_activity, 0.5);
        assert!(!pair.cilium_present);
    }

    #[test]
    fn test_cafd_creation() {
        let cafd = CAFD::new("YAP");
        assert_eq!(cafd.name, "YAP");
        assert_eq!(cafd.activity, 0.0);
        assert_eq!(cafd.concentration, 0.0);
    }

    #[test]
    fn test_ptm_profile_default() {
        let ptm = PTMProfile::default();
        assert_eq!(ptm.acetylation_level, 0.0);
        assert_eq!(ptm.oxidation_level, 0.0);
    }
}

// ---------------------------------------------------------------------------
// Inflammaging — канал обратной связи myeloid_shift_module → human_development_module
// ---------------------------------------------------------------------------

/// Состояние воспалительного старения (inflammaging).
///
/// Пишется из `myeloid_shift_module` каждый шаг.
/// Читается из `human_development_module` для коррекции скорости повреждений.
/// При отсутствии `myeloid_shift_module` компонент остаётся нулевым — поведение как раньше.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InflammagingState {
    /// Дополнительный множитель скорости ROS-повреждения [0..0.5].
    /// Применяется как: `effective_ros_rate = base_ros_rate × (1 + ros_boost)`
    pub ros_boost: f32,
    /// Снижение темпа регенерации ниши [0..0.5].
    /// Применяется как: `regeneration_tempo *= (1 - niche_impairment)`
    pub niche_impairment: f32,
    /// Интенсивность SASP (Senescence-Associated Secretory Phenotype) [0..1].
    pub sasp_intensity: f32,
}

/// Shared ECS-компонент статистики делений.
///
/// Пишется из `asymmetric_division_module` каждый шаг.
/// Читается из `human_development_module` для коррекции `stem_cell_pool`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DivisionExhaustionState {
    /// Число делений типа Differentiation (оба потомка дифференцируются — истощение пула).
    pub exhaustion_count: u32,
    /// Число асимметричных делений (нормальных).
    pub asymmetric_count: u32,
    /// Суммарное число завершённых делений.
    pub total_divisions: u32,
}

impl DivisionExhaustionState {
    /// Доля делений-истощений [0..1].
    /// 0 — только асимметричные; 1 — только дифференцировка.
    pub fn exhaustion_ratio(&self) -> f32 {
        let total = self.exhaustion_count + self.asymmetric_count;
        if total == 0 { 0.0 } else { self.exhaustion_count as f32 / total as f32 }
    }
}

/// Shared ECS-компонент для ключевых уровней экспрессии генов.
///
/// Пишется из `transcriptome_module` каждый шаг.
/// Читается из `cell_cycle_module` для p21/p16-арестов и модуляции G1.
/// При отсутствии `transcriptome_module` компонент остаётся дефолтным — поведение как раньше.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneExpressionState {
    /// CDKN1A (p21) — ингибитор Cdk1/2/4/6 [0..1].
    /// p21 > 0.7 → временный G1/S арест (ДНК-повреждение, стресс).
    pub p21_level: f32,
    /// CDKN2A (p16/INK4a) — ингибитор Cdk4/6 [0..1].
    /// p16 > 0.8 → постоянный арест (сенесценция).
    pub p16_level: f32,
    /// CCND1 (Cyclin D1) — промотор G1→S перехода [0..1].
    /// Высокий уровень укорачивает G1.
    pub cyclin_d_level: f32,
    /// CCNE1/2 (Cyclin E) — промотор G1→S (поздняя G1) [0..1].
    /// Синергичен с Cyclin D: ускоряет переход G1→S при высоких значениях.
    /// Записывается из `transcriptome_module` (при наличии), иначе дефолтный.
    pub cyclin_e_level: f32,
    /// MYC — общий транскрипционный активатор пролиферации [0..1].
    pub myc_level: f32,
}

impl Default for GeneExpressionState {
    fn default() -> Self {
        Self {
            p21_level:      0.0,
            p16_level:      0.0,
            cyclin_d_level: 0.5, // умеренный базальный уровень
            cyclin_e_level: 0.4, // умеренный базальный уровень
            myc_level:      0.3,
        }
    }
}

// ---------------------------------------------------------------------------
// Трек C: Теломеры
// ---------------------------------------------------------------------------

/// Состояние теломер стволовой клетки (Трек C CDATA).
///
/// Теломеры укорачиваются при каждом делении (лимит Хейфлика).
/// В рамках CDATA ускорение укорачивания обусловлено:
/// - `spindle_fidelity ↓` → хромосомная нестабильность → двойные разрывы у теломер
/// - `ros_level ↑` → окислительное повреждение теломерной ДНК
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelomereState {
    /// Средняя длина теломер в единицах T/S ratio [0..1]. Зигота = 1.0.
    pub mean_length: f32,
    /// Укорачивание за одно деление (≈50 п.н. → ~0.002 в T/S единицах).
    pub shortening_per_division: f32,
    /// true когда mean_length < 0.3 (Хейфликовский предел → сенесценция).
    pub is_critically_short: bool,
}

impl Default for TelomereState {
    fn default() -> Self {
        Self {
            mean_length: 1.0,
            shortening_per_division: 0.002,
            is_critically_short: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Трек D: Эпигенетические часы
// ---------------------------------------------------------------------------

/// Эпигенетические часы (Трек D CDATA) — биологический возраст по CpG-метилированию.
///
/// `methylation_age` догоняет хронологический возраст в молодости,
/// обгоняет его при высоком суммарном повреждении центриоли.
/// Ускорение часов отражает кумулятивный молекулярный стресс.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpigeneticClockState {
    /// Биологический возраст по эпигенетическим часам (лет).
    pub methylation_age: f32,
    /// Коэффициент ускорения часов (1.0 = норма; >1.0 = ускорены).
    /// `clock_acceleration = 1.0 + total_damage_score × 0.5`
    pub clock_acceleration: f32,
    /// Вклад эпигенетического ускорения в ROS следующего шага [0..0.05].
    /// Аналог `InflammagingState::ros_boost`, но от эпигенетических часов.
    /// Читается в начале step() и передаётся в `accumulate_damage()` вместе с infl_ros_boost.
    pub epi_ros_contribution: f32,
    /// Число делений на момент последнего эпигенетического сброса.
    /// Используется для детекции новых делений (сравнивается с `DivisionExhaustionState::total_divisions`).
    /// При делении дочерняя клетка наследует только половину «лишнего» метилирования:
    /// `methylation_age = (methylation_age + chron_age) / 2`.
    pub last_division_count: u32,
}

impl Default for EpigeneticClockState {
    fn default() -> Self {
        Self {
            methylation_age: 0.0,
            clock_acceleration: 1.0,
            epi_ros_contribution: 0.0,
            last_division_count: 0,
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Циркадный ритм (P18) — нарушение через потерю ресничек (CEP164↓)
// ──────────────────────────────────────────────────────────────────────────────

/// Состояние циркадного ритма стволовой ниши.
///
/// # Механизм связи с CDATA
///
/// Первичные реснички у стволовых клеток необходимы для трансдукции
/// ночных сигналов SCN (супрахиазматического ядра) через SHH-рецептор PTCH1
/// и цАМФ-каскад. CEP164 — ключевой белок перехода зоны — обеспечивает
/// барьер, необходимый для правильной сборки и функции реснички.
///
/// С потерей CEP164:
///  - нарушается PTCH1-канализация → Shh/Wnt сигналы ослабевают
///  - периферическая циркадная синхронизация нарушается → амплитуда↓
///  - BMAL1/CLOCK-зависимая активация протеасомы в ночной фазе снижается
///  - NF-κB получает конститутивную активацию → SASP↑
///
/// Биологические ссылки:
///  - Baggs & Green (2003): Clock proteins at centrosome
///  - Yeh et al. (2013): Primary cilia in circadian input
///  - Lipton et al. (2015): Circadian proteasome regulation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircadianState {
    /// Циркадная амплитуда [0..1].
    /// 1.0 = здоровый ритм; снижается при потере CEP164 и накоплении агрегатов.
    /// `amplitude = (cep164 × 0.6 + (1 − aggregates) × 0.4) × (1 − ros × 0.2)`
    pub amplitude: f32,

    /// Когерентность клеточных часов [0..1] — синхронность BMAL1/CLOCK цикла.
    /// Нарушается при высоком ROS (окисление CLOCK-белков).
    pub phase_coherence: f32,

    /// Циркадный буст протеасомы в ночной фазе [0..1].
    /// Модулирует агрегасомный клиренс: `proteasome_night_boost = amplitude × 0.25`.
    /// В норме UPS активность на 25% выше в ночной фазе (Lipton et al. 2015).
    pub proteasome_night_boost: f32,

    /// Вклад циркадного нарушения в SASP [0..0.3].
    /// `= (1 − amplitude) × 0.3`
    /// Биологически: нарушение циркадных часов активирует NF-κB конститутивно.
    pub circadian_sasp_contribution: f32,
}

impl Default for CircadianState {
    fn default() -> Self {
        Self {
            amplitude:                  1.0,
            phase_coherence:            1.0,
            proteasome_night_boost:     0.25,
            circadian_sasp_contribution: 0.0,
        }
    }
}

impl CircadianState {
    /// Обновить циркадное состояние на основе текущих повреждений центриоли.
    pub fn update(&mut self, dam: &CentriolarDamageState) {
        // Амплитуда: определяется целостностью CEP164 (ресничка) и уровнем агрегатов
        // (агрегаты нарушают BMAL1-транслокацию), модулируется ROS (окисление Clock)
        self.amplitude = (
            dam.cep164_integrity * 0.6
            + (1.0 - dam.protein_aggregates) * 0.4
        ) * (1.0 - dam.ros_level * 0.2);
        self.amplitude = self.amplitude.clamp(0.0, 1.0);

        // Когерентность: нарушается ROS → окисление CLOCK/BMAL1
        self.phase_coherence = (1.0 - dam.ros_level * 0.5)
            .clamp(0.1, 1.0);

        // Ночной буст протеасомы: пропорционален амплитуде
        self.proteasome_night_boost = self.amplitude * 0.25;

        // Циркадный вклад в SASP (конститутивная NF-κB при десинхронизации)
        self.circadian_sasp_contribution = (1.0 - self.amplitude) * 0.3;
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Аутофагия / mTOR (P19)
// ──────────────────────────────────────────────────────────────────────────────

/// Состояние пути аутофагии и mTOR-регуляции.
///
/// mTOR — главный ингибитор аутофагии. С возрастом mTOR-активность растёт
/// (nutrient sensing desensitization), аутофагический поток падает → агрегаты↑.
///
/// Связь с CDATA:
/// - Аутофагия чистит агрегаты независимо от агрегасомного пути (дополнительный клиренс)
/// - CR (Caloric Restriction) → mTOR↓ → аутофагия↑ → CDATA↓ (механизм интервенции P11)
/// - NadPlus → SIRT1↑ → AMPK↑ → mTOR↓ → аутофагия↑
/// - Mitophagy (MitochondrialModule) — частный случай: аутофагия специфически митохондрий
///
/// Биологические ссылки:
/// - Rubinsztein et al. (2011): Autophagy and ageing — Nature Rev Mol Cell Biol
/// - Harrison et al. (2009): Rapamycin-extended lifespan — Nature
/// - Madeo et al. (2015): Spermidine/AMPK/autophagy — Science
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutophagyState {
    /// Активность mTOR комплекса 1 [0..1].
    /// 0.0 = полное подавление (CR/рапамицин); 1.0 = максимальная активность.
    /// `mtor_activity_base = 0.3 + age/100 × 0.5`; модулируется питанием/интервенциями.
    pub mtor_activity: f32,

    /// Аутофагический поток [0..1].
    /// Обратно пропорционален mTOR: `autophagy_flux = 1 − mtor_activity × 0.8`
    pub autophagy_flux: f32,

    /// Вклад аутофагии в клиренс агрегатов [0..1].
    /// = `autophagy_flux × 0.10` — максимально 10% агрегатов/год при полной аутофагии.
    pub aggregate_autophagy_clearance: f32,

    /// Связь с митофагией: аутофагия усиливает базовую митофагию [0..0.3].
    /// = `autophagy_flux × 0.3`; суммируется с `MitochondrialState::mitophagy_flux`.
    pub mitophagy_coupling: f32,
}

impl Default for AutophagyState {
    fn default() -> Self {
        Self {
            mtor_activity:               0.3,   // молодой организм: умеренный mTOR
            autophagy_flux:              0.76,  // = 1 − 0.3×0.8
            aggregate_autophagy_clearance: 0.076,
            mitophagy_coupling:          0.228,
        }
    }
}

impl AutophagyState {
    /// Обновить аутофагическое состояние.
    ///
    /// # Аргументы
    /// * `age_years`         — хронологический возраст (лет)
    /// * `cr_active`         — флаг Caloric Restriction интервенции
    /// * `nad_plus_active`   — флаг NadPlus интервенции
    pub fn update(&mut self, age_years: f32, cr_active: bool, nad_plus_active: f32) {
        // Базовая mTOR: растёт с возрастом (десенсибилизация нутриент-сенсинга)
        let age_mtor = (0.3 + age_years / 100.0 * 0.5).clamp(0.3, 0.8);

        // Интервенции снижают mTOR:
        // CR: ~30% снижение mTOR (AMPK↑)
        // NadPlus: ~20% снижение (SIRT1→AMPK)
        let cr_reduction    = if cr_active { 0.30 } else { 0.0 };
        let nadp_reduction  = nad_plus_active * 0.20;

        self.mtor_activity = (age_mtor - cr_reduction - nadp_reduction).clamp(0.05, 1.0);

        // Аутофагический поток: обратно пропорционален mTOR
        self.autophagy_flux = (1.0 - self.mtor_activity * 0.8).clamp(0.0, 1.0);

        // Клиренс агрегатов через аутофагию (независим от протеасомного пути)
        self.aggregate_autophagy_clearance = self.autophagy_flux * 0.10;

        // Буст митофагии (суммируется с базовым потоком MitochondrialModule)
        self.mitophagy_coupling = self.autophagy_flux * 0.30;
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Протеостаз (P16) — белковый гомеостаз и клиренс агрегатов
// ──────────────────────────────────────────────────────────────────────────────

/// Состояние системы протеостаза (белкового гомеостаза).
///
/// Центросома = организующий центр агрегасом (Johnston et al. 1998).
/// Повреждение CEP164/spindle → нарушение агрегасом → агрегаты накапливаются быстрее.
/// HSP70/90 оксидируются ROS → потеря шапероновой ёмкости.
/// Протеасома перегружается агрегатами → активность падает.
///
/// Этот компонент модифицирует скорость накопления `protein_aggregates` в `CentriolarDamageState`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProteostasisState {
    /// Ёмкость протеасомы UPS [0..1].
    /// Снижается при перегрузке белковыми агрегатами: `= max(0, 1 - aggregates × 0.8)`.
    pub proteasome_activity: f32,

    /// Шапероновая ёмкость (HSP70/90/110) [0..1].
    /// Окисляется ROS: `= max(0.1, 1 - ros × 0.6)`.
    pub hsp_capacity: f32,

    /// Индекс формирования агрегасом [0..1].
    /// Зависит от целостности центросомы: `= cep164 × 0.6 + spindle_fidelity × 0.4`.
    /// Высокий индекс = центросома активно маршрутизирует мисфолдированные белки к лизосомам.
    pub aggresome_index: f32,

    /// Нагрузка несложенных белков (UPR-стресс) [0..1].
    /// `= aggregates × (1 - proteasome_activity × 0.7 - hsp_capacity × 0.3)`
    pub unfolded_protein_load: f32,

    /// Эффективный клиренс агрегатов [0..1] — суммарный защитный эффект.
    /// Применяется как `aggregate_clearance_rate = aggresome_index × hsp_capacity`.
    pub aggregate_clearance_rate: f32,
}

impl Default for ProteostasisState {
    fn default() -> Self {
        Self {
            proteasome_activity:     1.0,
            hsp_capacity:            1.0,
            aggresome_index:         1.0,
            unfolded_protein_load:   0.0,
            aggregate_clearance_rate: 1.0,
        }
    }
}

impl ProteostasisState {
    /// Обновить состояние протеостаза на основе текущих повреждений.
    ///
    /// Вызывается в `HumanDevelopmentModule::step()` ПОСЛЕ `accumulate_damage()`.
    pub fn update(&mut self, dam: &CentriolarDamageState) {
        // Протеасома: перегружается при высоких агрегатах
        self.proteasome_activity = (1.0 - dam.protein_aggregates * 0.8).max(0.1);

        // Шапероны: оксидируются ROS
        self.hsp_capacity = (1.0 - dam.ros_level * 0.6).max(0.1);

        // Агрегасомный индекс: зависит от CEP164 (переходная зона) и spindle_fidelity (MTOC)
        self.aggresome_index = dam.cep164_integrity * 0.6
            + dam.spindle_fidelity * 0.4;

        // UPR-нагрузка: агрегаты, которые не удаётся разобрать
        let clearance_capacity = self.proteasome_activity * 0.7 + self.hsp_capacity * 0.3;
        self.unfolded_protein_load =
            (dam.protein_aggregates * (1.0 - clearance_capacity)).clamp(0.0, 1.0);

        // Суммарный клиренс: агрегасомы + шапероны работают совместно
        self.aggregate_clearance_rate =
            (self.aggresome_index * self.hsp_capacity).clamp(0.0, 1.0);
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// NK-клеточный иммунный надзор (P15)
// ──────────────────────────────────────────────────────────────────────────────

/// Состояние NK-клеточного иммунного надзора стволовой ниши.
///
/// NK-клетки распознают повреждённые стволовые клетки через NKG2D-лиганды
/// (MICA/MICB/ULBP1-3), экспрессия которых индуцируется при клеточном стрессе
/// (ATM/ATR → NF-κB → лиганды), и элиминируют их.
///
/// С возрастом активность NK-клеток снижается (иммуносенесценция): меньше
/// функциональных NK-клеток → дефектные HSC выживают → CDATA ускоряется.
///
/// Обратная связь с myeloid_shift: миелоидный сдвиг подавляет NK-функцию
/// через ИЛ-13/ИЛ-4 (Th2-поляризация) и TGF-β (Трeg-экспансия).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NKSurveillanceState {
    /// Поверхностная экспрессия NKG2D-лигандов [0..1].
    /// Растёт пропорционально ROS и белковым агрегатам (стресс-ответ).
    pub nkg2d_ligand_expression: f32,

    /// Активность NK-клеток в нише [0..1].
    /// 1.0 — молодой здоровый организм; снижается с возрастом и при миелоидном сдвиге.
    pub nk_activity: f32,

    /// Вероятность NK-элиминации клетки за один шаг [0..1].
    /// `kill_prob = nk_activity × nkg2d_expression × (1 − immune_escape_fraction)`
    pub nk_kill_probability: f32,

    /// Фракция клеток с иммунным ускользанием [0..1].
    /// Возникает из-за снижения MHC-I (иммунное ускользание при высоких повреждениях).
    /// При protein_carbonylation > 0.6: MHC-I нарушается → частичное ускользание.
    pub immune_escape_fraction: f32,

    /// Общее число NK-элиминаций, пережитых нишей (для мониторинга).
    pub total_eliminations: u32,
}

impl Default for NKSurveillanceState {
    fn default() -> Self {
        Self {
            nkg2d_ligand_expression: 0.0,
            nk_activity:             1.0,
            nk_kill_probability:     0.0,
            immune_escape_fraction:  0.0,
            total_eliminations:      0,
        }
    }
}

impl NKSurveillanceState {
    /// Обновить NK-состояние на основе текущих повреждений и возраста.
    ///
    /// # Аргументы
    /// * `ros`           — `CentriolarDamageState::ros_level`
    /// * `aggregates`    — `CentriolarDamageState::protein_aggregates`
    /// * `carbonylation` — `CentriolarDamageState::protein_carbonylation`
    /// * `age_years`     — хронологический возраст (лет)
    /// * `myeloid_bias`  — из `MyeloidShiftComponent::myeloid_bias` (опционально, 0 если нет)
    pub fn update(
        &mut self,
        ros: f32,
        aggregates: f32,
        carbonylation: f32,
        age_years: f32,
        myeloid_bias: f32,
    ) {
        // NKG2D-лиганды индуцируются при стрессе (ROS + агрегаты).
        // ВАЖНО: нормальные клетки не экспрессируют NKG2D-лиганды — только клетки
        // под серьёзным стрессом (>30% от максимума). Поэтому вычитаем базовый уровень 0.30.
        // Биологически: MICA/MICB/ULBP1-3 индуцируются только при активации ATM/ATR/NF-κB,
        // что требует значимого повреждения ДНК или белкового стресса (Raulet et al. 2013).
        let raw_ligand = ros * 0.6 + aggregates * 0.4;
        self.nkg2d_ligand_expression = (raw_ligand - 0.30).max(0.0).clamp(0.0, 1.0);

        // NK-активность: 1.0 в молодости, линейно снижается после 40 лет до 0.3 к 90 годам.
        // Дополнительно подавляется миелоидным сдвигом (TGF-β/ИЛ-13).
        let age_decline = if age_years > 40.0 {
            ((age_years - 40.0) / 50.0 * 0.7).clamp(0.0, 0.7)
        } else {
            0.0
        };
        let myeloid_suppression = myeloid_bias * 0.25;
        self.nk_activity = (1.0 - age_decline - myeloid_suppression).clamp(0.1, 1.0);

        // Иммунное ускользание: при высоком карбонилировании нарушается MHC-I
        self.immune_escape_fraction = (carbonylation * 0.5).clamp(0.0, 0.5);

        // Итоговая вероятность элиминации за один шаг.
        // Ненулевая только если лиганды выше базового уровня.
        self.nk_kill_probability = (
            self.nk_activity
            * self.nkg2d_ligand_expression
            * (1.0 - self.immune_escape_fraction)
        ).clamp(0.0, 1.0);
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Ответ на повреждение ДНК (P20) — DDR / ATM / p53
// ──────────────────────────────────────────────────────────────────────────────

/// Состояние пути ответа на повреждение ДНК (DDR).
///
/// Spindle fidelity↓ → хромосомная нестабильность → анеуплоидия → DSB (двунитевые разрывы) →
/// ATM-киназа → p53-стабилизация → p21-транскрипция → G1-арест.
///
/// Этот компонент закрывает петлю CDATA → клеточный цикл:
/// - `p53_stabilization × 0.3` пишется в `GeneExpressionState.p21_level`
/// - `cell_cycle_module` читает `p21_level` и применяет G1SRestriction
///
/// Связь с другими компонентами:
/// - ROS (от `CentriolarDamageState`) вносит прямой вклад в γH2AX (окислительные разрывы)
/// - Агрегаты снижают `dna_repair_capacity` (протеасомная нагрузка мешает NHEJ/HR)
/// - Возраст снижает `dna_repair_capacity` (уменьшение MRN-комплекса, Ku70/Ku80)
///
/// Биологические ссылки:
/// - Jackson & Bartek (2009): DNA-damage response in human biology — Nature
/// - Rodier et al. (2009): Persistent DNA damage signalling — Nature Cell Biol
/// - Bakkenist & Kastan (2003): DNA damage activates ATM — Nature
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DDRState {
    /// Уровень γH2AX (маркер двунитевых разрывов ДНК) [0..1].
    /// `= (1 − spindle_fidelity)^1.5 × 0.8 + ros_level × 0.2`
    /// Spindle↓ → анеуплоидия → DSB; ROS→ окислительные разрывы (8-OHdG).
    pub gamma_h2ax_level: f32,

    /// Активность ATM-киназы [0..1].
    /// `= gamma_h2ax_level × dna_repair_capacity`
    /// ATM — главный сенсор DSB; активность ограничена ёмкостью репарации.
    pub atm_activity: f32,

    /// Стабилизация p53 [0..1].
    /// `= atm_activity × 0.7`
    /// ATM фосфорилирует MDM2, что стабилизирует p53 (снижает деградацию).
    /// Высокий p53 → транскрипция p21 → G1-арест (антипролиферативный барьер).
    pub p53_stabilization: f32,

    /// Ёмкость репарации ДНК (NHEJ + HR) [0..1].
    /// `= max(0.3, 1 − age_years/150 − aggregates × 0.2)`
    /// Снижается с возрастом (уменьшение Ku70/Ku80, Rad51) и при перегрузке протеасомы.
    /// Пол: 0.3 — минимальный уровень (репарация никогда полностью не отключается).
    pub dna_repair_capacity: f32,
}

impl Default for DDRState {
    fn default() -> Self {
        Self {
            gamma_h2ax_level:    0.0,
            atm_activity:        0.0,
            p53_stabilization:   0.0,
            dna_repair_capacity: 1.0,
        }
    }
}

impl DDRState {
    /// Обновить DDR-состояние на основе текущих повреждений и возраста.
    ///
    /// # Аргументы
    /// * `spindle_fidelity` — `CentriolarDamageState::spindle_fidelity` [0..1]
    /// * `ros_level`        — `CentriolarDamageState::ros_level` [0..1]
    /// * `aggregates`       — `CentriolarDamageState::protein_aggregates` [0..1]
    /// * `age_years`        — хронологический возраст (лет)
    pub fn update(
        &mut self,
        spindle_fidelity: f32,
        ros_level: f32,
        aggregates: f32,
        age_years: f32,
    ) {
        // ёмкость репарации: снижается с возрастом и перегрузкой агрегатами
        self.dna_repair_capacity =
            (1.0 - age_years / 150.0 - aggregates * 0.2).max(0.3);

        // γH2AX: spindle↓ → хромосомная нестабильность → DSB; ROS → окислительные разрывы
        let spindle_damage = (1.0 - spindle_fidelity).powf(1.5);
        self.gamma_h2ax_level = (spindle_damage * 0.8 + ros_level * 0.2).clamp(0.0, 1.0);

        // ATM: активируется DSB, ограничен ёмкостью репарации
        self.atm_activity = (self.gamma_h2ax_level * self.dna_repair_capacity).clamp(0.0, 1.0);

        // p53: стабилизируется ATM (фосфорилирование MDM2 → деградация MDM2↓)
        self.p53_stabilization = (self.atm_activity * 0.7).clamp(0.0, 1.0);
    }

    /// Вычислить вклад p53 в p21 для передачи в `GeneExpressionState`.
    ///
    /// Возвращает значение, которое нужно **добавить** к `GeneExpressionState.p21_level`:
    /// `p53_stabilization × 0.3` (максимальный вклад DDR в p21 = 0.3).
    #[inline]
    pub fn p21_contribution(&self) -> f32 {
        self.p53_stabilization * 0.3
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Трек F — Снижение темпа деления стволовых клеток со временем
// ──────────────────────────────────────────────────────────────────────────────

/// Состояние темпа деления стволовых клеток (Track F).
///
/// С возрастом стволовые клетки делятся всё реже — даже если ниша ещё не истощена
/// (Track B). Это самостоятельный механизм старения: замедление обновления тканей
/// при сохранном пуле. Ткань деградирует медленнее, чем при Track B, но неизбежно.
///
/// ## Молекулярные причины снижения темпа (Tkemaladze, 2024)
/// 1. **CEP164↓ → цилии↓ → Wnt/Shh↓ → Cyclin D1↓** — морфогенная стимуляция G1→S
///    снижается при потере первичных ресничек.
/// 2. **Spindle damage → p21↑ → G1-арест удлиняется** — клетки проводят больше времени
///    в G1, сокращая частоту делений в единицу времени.
/// 3. **ROS-индуцированное старение** — высокий ROS стабилизирует p16/p21, создавая
///    квазисенесцентные состояния без полного арреста.
/// 4. **mTOR-зависимое замедление** — с возрастом mTOR-активность растёт, подавляя
///    автофагию и перераспределяя ресурсы от пролиферации к репарации.
/// 5. **Накопление PTM на центриолях** — затруднение разборки PCM (перицентросомного
///    материала) перед делением → удлинение G2/M.
///
/// ## Связь с другими треками
/// - Усиливает Track B: медленное + уменьшающееся пул = двойной удар.
/// - Взаимодействует с Track A: потеря морфогенного сигнала снижает пролиферативный стимул.
/// - Связан с P20 (DDR): ATM → p53 → p21 → удлинение G1 → снижение division_rate.
/// - Взаимодействует с P19 (mTOR): высокий mTOR подавляет пролиферацию стволовых клеток.
///
/// ## Биологические ссылки
/// - Tkemaladze (2024): The rate of stem cell division decreases with age
/// - Janzen et al. (2006): Stem cell ageing is regulated by p21 — Aging Cell
/// - Beerman & Rossi (2015): Epigenetic regulation of HSC aging — Exp Cell Res
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StemCellDivisionRateState {
    /// Нормализованный темп деления [0..1].
    /// 1.0 = молодая клетка; снижается со временем по совокупности факторов.
    /// Применяется как мультипликатор к `TissueState::regeneration_tempo`.
    pub division_rate: f32,

    /// Вклад цилиарного/морфогенного сигнала в темп деления [0..1].
    /// `cilia_drive = 0.25 + ciliary_function × 0.75`
    /// При ciliary_function=0: минимальный сигнал 0.25 (паракринные пути).
    pub cilia_drive: f32,

    /// Вклад качества веретена (через длительность G1) [0..1].
    /// `spindle_drive = 0.3 + spindle_fidelity × 0.7`
    /// При spindle=0: фракция клеток в длительном G1-аресте максимальна.
    pub spindle_drive: f32,

    /// Возрастной коэффициент торможения [0..1].
    /// `age_factor = max(0.15, 1.0 - (age - 20).max(0) / 100)`
    /// Линейное снижение с 20 лет; достигает 0.15 примерно к 105 годам.
    pub age_factor: f32,

    /// Вклад ROS-индуцированного квазисенесцентного торможения [0..1].
    /// `ros_brake = 1.0 - ros_level × 0.4`  (ROS подавляет пролиферацию)
    pub ros_brake: f32,

    /// Вклад mTOR-зависимого торможения [0..1].
    /// `mtor_brake = 1.0 - (mtor_activity - 0.3) × 0.35`
    /// Базовый mTOR=0.3 → нет торможения; высокий mTOR → снижение темпа.
    pub mtor_brake: f32,

    /// Интегральный индекс снижения темпа деления по сравнению с молодой нормой [0..1].
    /// `decline_index = 1.0 - division_rate`
    /// 0 = нет снижения; 1 = деления полностью прекратились.
    pub decline_index: f32,
}

impl Default for StemCellDivisionRateState {
    fn default() -> Self {
        Self {
            division_rate:  1.0,
            cilia_drive:    1.0,
            spindle_drive:  1.0,
            age_factor:     1.0,
            ros_brake:      1.0,
            mtor_brake:     1.0,
            decline_index:  0.0,
        }
    }
}

impl StemCellDivisionRateState {
    /// Обновить темп деления на основе текущего состояния повреждений.
    ///
    /// # Аргументы
    /// * `ciliary_function` — `CentriolarDamageState::ciliary_function` [0..1]
    /// * `spindle_fidelity` — `CentriolarDamageState::spindle_fidelity` [0..1]
    /// * `ros_level`        — `CentriolarDamageState::ros_level` [0..1]
    /// * `age_years`        — хронологический возраст (лет)
    /// * `mtor_activity`    — из `AutophagyState::mtor_activity` [0..1]; 0.3 если недоступен
    pub fn update(
        &mut self,
        ciliary_function: f32,
        spindle_fidelity: f32,
        ros_level:        f32,
        age_years:        f32,
        mtor_activity:    f32,
    ) {
        // 1. Цилиарный/морфогенный вклад: Wnt/Shh стимулируют G1→S через Cyclin D1
        self.cilia_drive = (0.25 + ciliary_function * 0.75).clamp(0.0, 1.0);

        // 2. Качество веретена: повреждение → более длительный G1, арест (p21↑)
        self.spindle_drive = (0.30 + spindle_fidelity * 0.70).clamp(0.0, 1.0);

        // 3. Возрастной фактор: линейное снижение с 20 лет
        self.age_factor = (1.0 - (age_years - 20.0).max(0.0) / 100.0)
            .clamp(0.15, 1.0);

        // 4. ROS-тормоз: высокий ROS → p21/p16 стабилизация → квазисенесценция
        self.ros_brake = (1.0 - ros_level * 0.40).clamp(0.40, 1.0);

        // 5. mTOR-тормоз: повышенный mTOR перераспределяет ресурсы от пролиферации
        // Базовый mTOR=0.3 → нет торможения; mTOR=0.8 → -17.5% к темпу
        self.mtor_brake = (1.0 - (mtor_activity - 0.3).max(0.0) * 0.35).clamp(0.6, 1.0);

        // Интегральный темп: произведение всех компонентов
        self.division_rate = (self.cilia_drive
            * self.spindle_drive
            * self.age_factor
            * self.ros_brake
            * self.mtor_brake)
            .clamp(0.01, 1.0);

        self.decline_index = 1.0 - self.division_rate;
    }
}

impl CellCycleStateExtended {
    /// Получить активность конкретного комплекса
    pub fn get_complex_activity(&self, cyclin_type: CyclinType, cdk_type: CdkType) -> f32 {
        for complex in &self.cyclin_cdk_complexes {
            if complex.cyclin_type == cyclin_type && complex.cdk_type == cdk_type {
                return complex.activity;
            }
        }
        0.0
    }
    
    /// Учет влияния центриоли (заглушка)
    pub fn apply_centriole_influence(&mut self, _centriole: &CentriolePair) {
        // Будет реализовано позже
    }
    
    /// Обновление циклинов (заглушка)
    pub fn update_cyclins(&mut self, _dt: f32) {
        // Будет реализовано позже
    }
}

// ---------------------------------------------------------------------------
// Необратимая дифференцировка и обратимая модуляция (CDATA)
// ---------------------------------------------------------------------------

/// Необратимый уровень дифференцировки клетки.
///
/// Определяется отщеплением индукторов дифференцировки от центриолей.
/// Каждый переход запускается **внутренним** фактором — потерей индуктора —
/// а не внешними сигналами. После фиксации уровень не может регрессировать.
///
/// Соответствует [`PotencyLevel`] молекулярному состоянию:
/// `Totipotent → Pluripotent → Multipotent → Committed → Terminal`
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum DifferentiationTier {
    /// Зигота: все индукторы интактны, оба комплекта полны.
    Totipotent,
    /// Плюрипотент: оба комплекта имеют оставшиеся индукторы, но уже потеряли часть.
    Pluripotent,
    /// Мультипотент (олигопотент): один комплект исчерпан.
    Multipotent,
    /// Коммитированная: последние индукторы почти исчерпаны (унипотент).
    Committed,
    /// Терминально дифференцированная: оба комплекта пусты, деления невозможны.
    Terminal,
}

impl DifferentiationTier {
    /// Производить `DifferentiationTier` из текущего потентностного состояния.
    pub fn from_potency(potency: PotencyLevel) -> Self {
        match potency {
            PotencyLevel::Totipotent  => DifferentiationTier::Totipotent,
            PotencyLevel::Pluripotent => DifferentiationTier::Pluripotent,
            PotencyLevel::Oligopotent => DifferentiationTier::Multipotent,
            PotencyLevel::Unipotent   => DifferentiationTier::Committed,
            PotencyLevel::Apoptosis   => DifferentiationTier::Terminal,
        }
    }
}

/// ECS-компонент необратимого статуса дифференцировки (CDATA).
///
/// Устанавливается однажды при первом отщеплении индуктора и может продвигаться
/// **только вперёд** по лестнице дифференцировки.
/// Отражает биологическую концепцию CDATA: при каждом отщеплении индуктора
/// он внедряется в ядерную ДНК → включаются генные сети нового статуса,
/// выключаются предыдущие. Этот процесс необратим.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DifferentiationStatus {
    /// Текущий необратимый уровень (только вперёд).
    pub tier: DifferentiationTier,
    /// История переходов: `(новый_уровень, возраст_в_годах)`.
    pub tier_history: Vec<(DifferentiationTier, f64)>,
    /// Количество необратимых переходов (событий коммитирования).
    pub commitment_events: u32,
    /// Активны ли индукторы дифференцировки (создаются de novo при n-м делении).
    /// `false` до достижения стадии de novo — клетка не может коммитироваться.
    pub inductors_active: bool,
    /// Произошла ли элиминация центриолей в прелептотенной стадии мейоза.
    /// `true` — элиминация зарегистрирована (для текущего поколения).
    pub meiotic_reset_done: bool,
}

impl DifferentiationStatus {
    pub fn new(initial_potency: PotencyLevel) -> Self {
        Self {
            tier: DifferentiationTier::from_potency(initial_potency),
            tier_history: Vec::new(),
            commitment_events: 0,
            inductors_active: false,
            meiotic_reset_done: false,
        }
    }

    /// Продвинуть tier вперёд, если `new_potency` даёт более высокий уровень дифференцировки.
    /// Возвращает `true` если произошёл переход (commitment event).
    /// Никогда не позволяет регрессировать.
    pub fn try_advance(&mut self, new_potency: PotencyLevel, age_years: f64) -> bool {
        let new_tier = DifferentiationTier::from_potency(new_potency);
        if new_tier > self.tier {
            self.tier_history.push((new_tier, age_years));
            self.tier = new_tier;
            self.commitment_events += 1;
            true
        } else {
            false
        }
    }

    /// Сброс статуса дифференцировки при элиминации центриолей в прелептотенной стадии мейоза.
    /// Индукторы элиминируются → следующее поколение начнёт с Totipotent.
    /// История сохраняется для аудита; счётчик коммитирований сбрасывается.
    /// `meiotic_reset_done` сбрасывается в `false` — следующее поколение может снова пройти мейоз.
    pub fn reset_for_meiosis(&mut self) {
        self.tier = DifferentiationTier::Totipotent;
        self.commitment_events = 0;
        self.inductors_active = false;
        self.meiotic_reset_done = false; // сброс флага — новое поколение может снова пройти этот этап
    }
}

impl Default for DifferentiationStatus {
    fn default() -> Self { Self::new(PotencyLevel::Totipotent) }
}

/// ECS-компонент обратимой модуляции клетки (CDATA).
///
/// Изменяется под влиянием **внешних** сигналов: нишевых факторов, паракрина,
/// воспаления (InflammagingState), ростовых факторов.
/// Не меняет [`DifferentiationStatus`] — только адаптирует поведение клетки
/// в рамках уже зафиксированного статуса дифференцировки.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModulationState {
    /// Уровень активности [0..1]: 0 = покой (G0), 1 = максимальная активность.
    pub activity_level: f32,
    /// Обратимый покой (G0-квесценция): `true` при `activity_level < 0.2`.
    pub is_quiescent: bool,
    /// Сила нишевых сигналов, получаемых клеткой [0..1].
    pub niche_signal_strength: f32,
    /// Ответ на острый стресс [0..1]: шаперонный стресс-ответ (HSP70, HSP90).
    pub stress_response: f32,
    /// SASP-вклад этой клетки в окружающую нишу [0..1].
    /// Ненулевой только у сенесцентных клеток.
    pub sasp_output: f32,
    /// Эпигенетическая пластичность [0..1]: насколько клетка может модулировать
    /// экспрессию в рамках текущего дифференцировочного статуса.
    /// Снижается по мере прохождения уровней дифференцировки.
    pub epigenetic_plasticity: f32,
}

impl Default for ModulationState {
    fn default() -> Self {
        Self {
            activity_level: 1.0,
            is_quiescent: false,
            niche_signal_strength: 1.0,
            stress_response: 0.0,
            sasp_output: 0.0,
            epigenetic_plasticity: 1.0,
        }
    }
}

// ============================================================
// Митохондриальное состояние (Трек E)
// ============================================================

/// ECS-компонент митохондриального здоровья (CDATA Трек E).
///
/// Митохондрии формируют кислородный щит вокруг центросомы.
/// При дисфункции митохондрий (мутации мтДНК, фрагментация, избыток ROS)
/// щит ослабевает → больше O₂ проникает к центриолям → ускоряется
/// отщепление индукторов.
/// Клональное состояние стволовой ниши.
///
/// Каждая основательская ниша получает уникальный `clone_id` при инициализации.
/// При симметричном делении (заполнение пустого слота пула) дочь наследует
/// тот же `clone_id` и инкрементирует `generation`.
///
/// Используется для моделирования клонального гемопоэза (CHIP):
/// клоны с более медленным истощением индукторов постепенно вытесняют
/// стареющие линии — демографический дрейф без отбора по fitness.
///
/// Источник: «Centrioles as determinants» (2026-01-27) + Jaiswal et al. 2014 (CHIP).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClonalState {
    /// Уникальный ID клональной линии (назначается при основании ниши, не меняется).
    pub clone_id: u64,
    /// Номер поколения от основателя (0 = основатель, 1 = первое дочернее, ...).
    pub generation: u32,
    /// Возраст организма (дни) на момент основания данной клональной линии.
    pub founder_age_days: f64,
}

impl ClonalState {
    /// Создать нового основателя.
    pub fn founder(clone_id: u64) -> Self {
        Self { clone_id, generation: 0, founder_age_days: 0.0 }
    }

    /// Создать клональную дочь (тот же clone_id, generation+1).
    pub fn daughter(&self) -> Self {
        Self {
            clone_id: self.clone_id,
            generation: self.generation + 1,
            founder_age_days: self.founder_age_days,
        }
    }
}

/// Маркер для lazy-init HumanDevelopmentModule.
///
/// Добавляется `AsymmetricDivisionModule` при NichePool-спавне новой ниши.
/// `HumanDevelopmentModule` обнаруживает его в начале `step()`, инициализирует
/// `HumanDevelopmentComponent` и удаляет маркер.
/// Это позволяет NichePool-заменам стареть как полноценные ниши.
#[derive(Debug, Clone, Copy, Default)]
pub struct NeedsHumanDevInit;

///
/// # Петли обратной связи
/// 1. `mtdna_mutations ↑` → `ros_production ↑` → `mtdna_mutations ↑` (цикл)
/// 2. `ros_production ↑` → `fusion_index ↓` (фрагментация) → митофагия менее эффективна
/// 3. `ros_production ↑` → `ros_boost` → `CentriolarDamageState.ros_level ↑`
///    (через `human_development_module`, лаг 1 шаг)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MitochondrialState {
    /// Накопление мутаций мтДНК [0..1].
    /// 0 = здоровый геном; 1 = критическая мутационная нагрузка.
    pub mtdna_mutations: f32,
    /// Индекс слияния митохондрий [0..1].
    /// 1.0 = нитевидная сеть (молодые, здоровые);
    /// 0.0 = полная фрагментация (стареющие).
    pub fusion_index: f32,
    /// Продукция ROS митохондриями [0..1].
    /// 0.0 = нормальный уровень; 1.0 = критическая суперпродукция.
    pub ros_production: f32,
    /// Мембранный потенциал (ΔΨm) [0..1].
    /// 1.0 = максимальный потенциал (молодые митохондрии);
    /// снижается при дисфункции → митофагия через PINK1/Parkin теряет эффективность.
    pub membrane_potential: f32,
    /// Поток митофагии [0..1]: скорость очистки дисфункциональных митохондрий.
    /// Снижается при низком `membrane_potential`.
    pub mitophagy_flux: f32,
    /// Вклад митохондрий в кислородный щит центросомы [0..1].
    /// 1.0 = полный щит; 0.0 = щита нет.
    pub mito_shield_contribution: f32,
    /// Плотность перинуклеарного митохондриального кластера [0..1] (P9).
    /// Зависит от `fusion_index` и локального ROS; добавляет пространственный
    /// барьер диффузии O₂ к центросоме поверх скалярного `mito_shield`.
    /// `perinuclear_barrier = perinuclear_density × 0.15`
    pub perinuclear_density: f32,
}

impl Default for MitochondrialState {
    fn default() -> Self {
        Self {
            perinuclear_density: 1.0,
            mtdna_mutations: 0.0,
            fusion_index: 1.0,
            ros_production: 0.0,
            membrane_potential: 1.0,
            mitophagy_flux: 1.0,
            mito_shield_contribution: 1.0,
        }
    }
}

impl MitochondrialState {
    /// Создать молодое митохондриальное состояние (синоним `default()`).
    pub fn pristine() -> Self { Self::default() }

    /// Вычислить вклад в ROS-буст центриолярных повреждений.
    /// Масштаб задаётся параметром `ros_production_boost`.
    pub fn ros_boost(&self, ros_production_boost: f32) -> f32 {
        self.ros_production * ros_production_boost
    }
}


#[cfg(test)]
mod asymmetry_tests {
    use super::*;

    #[test]
    fn test_blood_is_mother_to_stem() {
        assert_eq!(TissueType::Blood.asymmetry_rule(), CentrioleAsymmetryRule::MotherToStem);
    }

    #[test]
    fn test_neural_is_mother_to_stem() {
        assert_eq!(TissueType::Neural.asymmetry_rule(), CentrioleAsymmetryRule::MotherToStem);
    }

    #[test]
    fn test_epithelial_is_symmetric() {
        assert_eq!(TissueType::Epithelial.asymmetry_rule(), CentrioleAsymmetryRule::Symmetric);
    }

    #[test]
    fn test_mother_bias_blood_higher_than_skin() {
        // HSC-ниша: старая мать → LT-HSC → больший mother_bias
        let blood_bias = TissueType::Blood.default_mother_bias();
        let skin_bias  = TissueType::Skin.default_mother_bias();
        assert!(blood_bias > skin_bias,
            "Blood mother_bias={} должен быть > Skin mother_bias={}", blood_bias, skin_bias);
    }

    #[test]
    fn test_all_biases_in_valid_range() {
        let tissues = [
            TissueType::Neural, TissueType::Blood, TissueType::Epithelial,
            TissueType::Muscle, TissueType::Skin,  TissueType::Germline,
            TissueType::Connective, TissueType::Bone, TissueType::Cartilage,
            TissueType::Adipose, TissueType::Liver, TissueType::Kidney,
            TissueType::Heart, TissueType::Lung,
        ];
        for t in tissues {
            let b = t.default_mother_bias();
            assert!(b >= 0.0 && b <= 1.0,
                "{:?}: mother_bias={} вне диапазона [0,1]", t, b);
        }
    }
}

// ---------------------------------------------------------------------------
// Тесты морфогенных механизмов (P13)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod morphogen_tests {
    use super::*;

    /// GLI-активация: Hill-нелинейность (n=2, K=0.5)
    #[test]
    fn test_gli_activation_hill_nonlinearity() {
        let mut dam = CentriolarDamageState::pristine();

        // При полных ресничках (1.0) → GLI ≈ 0.80 (1.0² / (0.25 + 1.0) = 0.80)
        dam.ciliary_function = 1.0;
        let gli_full = dam.gli_activation();
        assert!((gli_full - 0.80).abs() < 0.01,
            "GLI при cilia=1.0 должен быть ≈0.80, получено {:.4}", gli_full);

        // При EC50 (0.5) → GLI = 0.5
        dam.ciliary_function = 0.5;
        let gli_half = dam.gli_activation();
        assert!((gli_half - 0.5).abs() < 0.01,
            "GLI при cilia=0.5 (EC50) должен быть 0.5, получено {:.4}", gli_half);

        // При нуле → GLI = 0
        dam.ciliary_function = 0.0;
        let gli_zero = dam.gli_activation();
        assert_eq!(gli_zero, 0.0, "GLI при cilia=0 должен быть 0");

        // Нелинейность: при cilia=0.3 ответ < 0.3 (порог работает)
        dam.ciliary_function = 0.3;
        let gli_low = dam.gli_activation();
        assert!(gli_low < 0.3,
            "Hill-нелинейность: GLI({:.1}) < {:.1} при n=2, K=0.5", gli_low, 0.3);
    }

    /// GLI строго монотонно растёт с ciliary_function
    #[test]
    fn test_gli_activation_monotone() {
        let mut dam = CentriolarDamageState::pristine();
        let mut prev = -1.0f32;
        for i in 0..=10 {
            dam.ciliary_function = i as f32 / 10.0;
            let gli = dam.gli_activation();
            assert!(gli >= prev, "GLI должен быть монотонно возрастающим");
            assert!(gli >= 0.0 && gli <= 1.0, "GLI вне [0..1]: {}", gli);
            prev = gli;
        }
    }

    /// Старение (CEP164↓) → GLI резко падает из-за Hill-нелинейности
    #[test]
    fn test_gli_drops_faster_than_linear_with_cilia_loss() {
        let mut dam = CentriolarDamageState::pristine();

        dam.ciliary_function = 0.8;
        let gli_80 = dam.gli_activation();

        dam.ciliary_function = 0.2;
        let gli_20 = dam.gli_activation();

        // Линейный ответ: 0.2/0.8 = 0.25 (25% относительно 80%)
        // Hill-ответ должен быть значительно меньше (нелинейный порог)
        let ratio_gli = gli_20 / gli_80;
        let ratio_linear = 0.2 / 0.8;
        assert!(ratio_gli < ratio_linear,
            "Hill-нелинейность: ratio_gli={:.3} должен быть < ratio_linear={:.3}",
            ratio_gli, ratio_linear);
    }

    /// TissueState: морфогенные поля инициализируются в правильных диапазонах
    #[test]
    fn test_tissue_state_morphogen_fields_default() {
        let ts = TissueState::new(TissueType::Blood);
        assert!(ts.gli_activation >= 0.0 && ts.gli_activation <= 1.0);
        assert!(ts.shh_concentration >= 0.0 && ts.shh_concentration <= 1.0);
        assert!(ts.bmp_balance >= 0.0 && ts.bmp_balance <= 1.0);
        assert!(ts.wnt_activity >= 0.0 && ts.wnt_activity <= 1.0);
        // Молодая ткань: низкий BMP (Noggin преобладает)
        assert!(ts.bmp_balance < 0.5,
            "Молодая ткань должна иметь низкий bmp_balance, получено {}", ts.bmp_balance);
    }

    /// detail_level=1: только GLI, остальные поля не обновляются
    #[test]
    fn test_morphogen_detail_level_1_updates_only_gli() {
        let mut ts = TissueState::new(TissueType::Blood);
        // Установим необычные значения BMP/Wnt/Shh — они не должны меняться при level=1
        ts.bmp_balance = 0.99;
        ts.wnt_activity = 0.01;
        ts.shh_concentration = 0.01;

        ts.update_morphogen_fields(0.5, 1);

        // GLI должен быть обновлён (cilia=0.5 → GLI=0.5)
        assert!((ts.gli_activation - 0.5).abs() < 0.01,
            "GLI должен быть обновлён при detail_level=1");
        // BMP и Wnt — без изменений
        assert!((ts.bmp_balance - 0.99).abs() < 0.01,
            "bmp_balance не должен меняться при detail_level=1");
        assert!((ts.wnt_activity - 0.01).abs() < 0.01,
            "wnt_activity не должна меняться при detail_level=1");
    }

    /// detail_level=3: все поля обновляются
    #[test]
    fn test_morphogen_detail_level_3_updates_all() {
        let mut ts = TissueState::new(TissueType::Blood);
        ts.bmp_balance = 0.99;

        ts.update_morphogen_fields(0.5, 3);

        // При cilia=0.5 BMP-баланс должен снизиться (noggin_activity = 0.35 → bmp ≈ 0.52)
        assert!(ts.bmp_balance < 0.99,
            "bmp_balance должен обновиться при detail_level=3, получено {}", ts.bmp_balance);
        assert!(ts.wnt_activity > 0.0 && ts.wnt_activity <= 1.0,
            "wnt_activity должна быть в [0..1]");
    }

    /// С потерей ресничек BMP_balance растёт (Noggin ↓ → BMP ↑)
    #[test]
    fn test_bmp_increases_with_cilia_loss() {
        let mut ts = TissueState::new(TissueType::Blood);

        ts.update_morphogen_fields(1.0, 2);
        let bmp_healthy = ts.bmp_balance;

        ts.update_morphogen_fields(0.1, 2);
        let bmp_damaged = ts.bmp_balance;

        assert!(bmp_damaged > bmp_healthy,
            "BMP-баланс должен расти при потере ресничек: {:.3} > {:.3}",
            bmp_damaged, bmp_healthy);
    }
}
