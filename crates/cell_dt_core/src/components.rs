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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InducerDetachmentParams {
    /// Базовая вероятность отщепления на шаг при oxygen_level=1.0
    pub base_detach_probability: f32,
    /// Доля вероятности, приходящаяся на материнскую центриоль [0..1]
    /// 0.5 = равновероятно; >0.5 = материнская теряет чаще (она старше)
    pub mother_bias: f32,
    /// Коэффициент влияния возраста (лет) на mother_bias
    /// effective_bias = mother_bias + age_years × age_bias_coefficient
    pub age_bias_coefficient: f32,
}

impl Default for InducerDetachmentParams {
    fn default() -> Self {
        Self {
            base_detach_probability: 0.002,
            mother_bias: 0.6,
            age_bias_coefficient: 0.003,
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
            detachment_params: self.detachment_params.clone(),
        };
        let cell_b = CentriolarInducerPair {
            mother_set:  self.daughter_set.clone(),
            daughter_set: self.daughter_set.inherit_from(),
            division_count: 0,
            detachment_params: self.detachment_params.clone(),
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

        // Сенесценция — когда суммарный ущерб превышает порог
        let total_damage = self.total_damage_score();
        if total_damage > 0.75 {
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
}

impl Default for CentriolarDamageState {
    fn default() -> Self {
        Self::pristine()
    }
}

/// Тип ткани для специфики стволовых ниш
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TissueType {
    /// Нейральные стволовые клетки (СВЗ, зубчатая извилина)
    Neural,
    /// Гемопоэтические стволовые клетки (красный костный мозг)
    Hematopoietic,
    /// Кишечный эпителий (крипты Либеркюна)
    IntestinalCrypt,
    /// Мышечные клетки-сателлиты
    Muscle,
    /// Кожный эпителий (базальный слой)
    Skin,
    /// Половые клетки
    Germline,
}

/// Состояние ткани — агрегированные метрики регенеративного потенциала
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TissueState {
    pub tissue_type: TissueType,
    /// Размер пула стволовых клеток (0..1, относительно молодого организма)
    pub stem_cell_pool: f32,
    /// Темп регенерации (0..1, относительно молодого организма)
    pub regeneration_tempo: f32,
    /// Доля сенесцентных клеток [0..1]
    pub senescent_fraction: f32,
    /// Средний возраст материнской центриоли в нише (в делениях)
    pub mean_centriole_age: f32,
    /// Функциональная ёмкость ткани [0..1]
    pub functional_capacity: f32,
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
        }
    }

    /// Обновить функциональную ёмкость из текущих метрик
    pub fn update_functional_capacity(&mut self) {
        self.functional_capacity = self.stem_cell_pool
            * self.regeneration_tempo
            * (1.0 - self.senescent_fraction * 0.8);
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
        }
    }
}

impl Default for OrganismState {
    fn default() -> Self {
        Self::new()
    }
}

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
