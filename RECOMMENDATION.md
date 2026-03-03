# Cell DT — Рекомендации по оптимизации модели CDATA

> **Статус:** Живой документ. Вычёркивать/удалять пункты по мере выполнения.
> Выполненные шаги помечаются `[x]`, невыполненные — `[ ]`.
> Последнее обновление: 2026-03-03

---

## ВЫПОЛНЕНО

- [x] **CentriolarInducers → CentriolarInducerPair** — полная замена системы индукторов:
  M-комплект (материнская центриоль) + D-комплект (дочерняя). O₂ отщепляет
  от обоих (если оба непусты) или только от непустого. Новые центриоли наследуют
  ТЕКУЩИЙ остаток, а не исходный максимум.
- [x] **CentriolarDamageState sync** — каждый `step()` синхронизирует отдельный
  ECS-компонент `CentriolarDamageState` из `HumanDevelopmentComponent.centriolar_damage`,
  чтобы другие модули могли его читать без зависимости от human_development_module.
- [x] **AsymmetricDivisionModule.step()** — реализован: читает `CentriolarDamageState`,
  классифицирует тип деления (Asymmetric / SelfRenewal / Differentiation / нет деления).
- [x] **StemCellHierarchyModule.step()** — реализован: читает `spindle_fidelity` как
  прокси-потентность и синхронизирует `StemCellHierarchyState`.
- [x] **CLAUDE.md** — написан и обновлён.
- [x] **`InflammagingState` в `cell_dt_core::components`** — добавлен shared ECS-компонент
  обратной связи: `ros_boost`, `niche_impairment`, `sasp_intensity`.
- [x] **`AgingPhenotype::ImmuneDecline`** — добавлен в `aging.rs`.
- [x] **`human_development_module` читает `InflammagingState`** — применяет `ros_boost`
  к ros_level и `niche_impairment` к regeneration_tempo. Активирует `ImmuneDecline` при `sasp > 0.4`.
- [x] **`human_development_module.initialize()` спавнит `InflammagingState::default()`**.
- [x] **`myeloid_shift_module`** — полностью реализован:
  - `MyeloidShiftComponent` (myeloid_bias, lymphoid_deficit, inflammaging_index, immune_senescence, phenotype)
  - `MyeloidShiftParams` (6 параметров через get/set_params)
  - Формула CDATA-обоснована: (1-spindle)^1.5×0.45 + (1-cilia)×0.30 + ros×0.15 + agg×0.10
  - Обратная связь → InflammagingState каждый step()
  - 7 unit-тестов пройдены
  - Пример `myeloid_shift_example.rs`
- [x] **TODO.md** — перезаписан с актуальным статусом.
- [x] **RECOMENDATION.md** (старый файл) — помечен как устаревший.

---

## 1. ПОДГОТОВКА К МЕЛОИДНОМУ СДВИГУ ✅ ВЫПОЛНЕНО

- [x] **`InflammagingState` в `cell_dt_core::components`** — добавлен.
- [x] **`human_development_module` читает `InflammagingState`** — применяет `ros_boost` и `niche_impairment`.
- [x] **`AgingPhenotype::ImmuneDecline`** — добавлен в `aging.rs`.
- [x] **`human_development_module.initialize()` спавнит `InflammagingState::default()`**.

---

## 2. МЕЛОИДНЫЙ СДВИГ ✅ ВЫПОЛНЕНО

### Биология и связь с CDATA

С возрастом гематопоэтические стволовые клетки (HSC) и стволовые клетки других тканей
смещают дифференцировку от лимфоидного пути к миелоидному. В рамках CDATA это происходит
через четыре конкретных молекулярных повреждения:

| Компонент CDATA | Механизм биологически | Вклад в myeloid_bias |
|---|---|---|
| `spindle_fidelity ↓` | Веретено не может сегрегировать fate-детерминанты (Numb, aPKC) → оба потомка миелоидные | **45%** |
| `ciliary_function ↓` (CEP164↓) | Нет реснички → нет Wnt/Notch/Shh → LT-HSC теряет лимфоидный нише-сигнал → PU.1 побеждает | **30%** |
| `ros_level ↑` | ROS → NF-κB → IL-6, TNF-α, IL-1β → SASP → миелоидная цитокиновая среда | **15%** |
| `protein_aggregates ↑` | Агрегаты белков захватывают IKZF1/Ikaros → снятие репрессии с миелоидных генов | **10%** |

**Обратные связи мелоидного сдвига → CDATA:**

```
myeloid_bias ↑
  → inflammaging_index ↑
      → ros_boost ↑     → DamageParams.ros_rate ускоряется (петля ROS усиливается)
      → niche_impairment↑ → TissueState.regeneration_tempo ↓
  → immune_senescence ↑
      → AgingPhenotype::ImmuneDecline активируется
      → lymphoid_deficit ↑ → снижение иммунного надзора (онкологический риск)
```

### Формулы

```
spindle_c  = (1 − spindle_fidelity)^1.5 × 0.45
cilia_c    = (1 − ciliary_function)  × 0.30
ros_c      = ros_level               × 0.15
aggr_c     = protein_aggregates      × 0.10

myeloid_bias = clamp(spindle_c + cilia_c + ros_c + aggr_c,  0.0, 1.0)

lymphoid_deficit   = myeloid_bias                          (упрощённая модель)
inflammaging_index = myeloid_bias × lymphoid_deficit × 0.8
immune_senescence  = inflammaging_index × 0.7 + (1 − cilia_c_raw × 2) × 0.3

ros_boost        = inflammaging_index × 0.15   → InflammagingState
niche_impairment = inflammaging_index × 0.08   → InflammagingState
sasp_intensity   = inflammaging_index           → InflammagingState
```

**Калибровочные ориентиры:**
- Возраст 20 лет (pristine): `myeloid_bias ≈ 0.02` — норма
- Возраст 50 лет: `myeloid_bias ≈ 0.25` — MildShift (умеренный, субклинический)
- Возраст 70 лет: `myeloid_bias ≈ 0.45` — ModerateShift (клинически значимый)
- Возраст 85 лет: `myeloid_bias ≈ 0.65` — SevereShift (иммуностарение)

### Технические шаги

- [x] **Создать crate `myeloid_shift_module`** — выполнено.

- [x] **`MyeloidShiftComponent`** — реализован:
  ```rust
  #[derive(Debug, Clone, Serialize, Deserialize)]
  pub struct MyeloidShiftComponent {
      pub myeloid_bias: f32,
      pub lymphoid_deficit: f32,
      pub inflammaging_index: f32,
      pub immune_senescence: f32,
      pub phenotype: MyeloidPhenotype,
  }

  #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
  pub enum MyeloidPhenotype {
      Healthy,        // myeloid_bias < 0.30
      MildShift,      // 0.30..0.50
      ModerateShift,  // 0.50..0.70  ← клинически значимый
      SevereShift,    // > 0.70      ← иммуностарение
  }
  ```

- [x] **`MyeloidShiftParams`** — реализован:
  ```rust
  pub struct MyeloidShiftParams {
      pub spindle_weight: f32,     // default 0.45
      pub cilia_weight: f32,       // default 0.30
      pub ros_weight: f32,         // default 0.15
      pub aggregate_weight: f32,   // default 0.10
      pub ros_boost_scale: f32,    // default 0.15
      pub niche_impair_scale: f32, // default 0.08
  }
  ```

- [x] **`MyeloidShiftModule.step()`** — реализован:
  1. Для каждой сущности с `(&CentriolarDamageState, &mut MyeloidShiftComponent, &mut InflammagingState)`:
  2. Вычислить `myeloid_bias` по формуле выше
  3. Вычислить `inflammaging_index`, `immune_senescence`
  4. Обновить `MyeloidShiftComponent`
  5. Записать в `InflammagingState { ros_boost, niche_impairment, sasp_intensity }`

- [x] **`MyeloidShiftModule.initialize()`** — реализован:
  - `MyeloidShiftComponent::default()`
  - `InflammagingState::default()` (если не было добавлено ранее)

- [x] **Unit-тесты** — 7 тестов пройдены (pristine, max_damage, spindle, cilia, calibration_age70, ros_boost, phenotype).

- [x] **Пример `myeloid_shift_example.rs`** — создан в `examples/src/bin/`.

- [x] **`CLAUDE.md`** — обновлён.

---

## 3. ЗАГЛУШКИ (существующие модули без реализации)

- [ ] **`centriole_module.step()`** — реализовать накопление PTM:
  - Читает `CentriolarDamageState` (standalone ECS) и `CellCycleStateExtended`
  - В фазе S/M: `protein_carbonylation += base_carbonylation_rate × dt`
  - В фазе M: `phosphorylation_dysregulation` растёт при каждом делении
  - Синхронизировать с `HumanDevelopmentComponent` через общий standalone компонент
  - *Примечание:* Пока `HumanDevelopmentModule` уже накапливает повреждения — избежать двойного счёта.
    Решение: `centriole_module` накапливает только PTM-специфичные повреждения, `human_development_module` — системные/возрастные.

- [ ] **`AsymmetricDivisionModule` — спавн дочерних сущностей** (опционально):
  - При `DivisionType::Asymmetric` → `world.spawn(...)` новой сущности с:
    - `CentriolarInducerPair` = результат `parent_pair.divide().1`
    - `CentriolarDamageState::pristine()` (молодая дочерняя клетка)
  - Ограничение: `max_entities: usize` параметр, не спавнить если превышено
  - *Риск:* Экспоненциальный рост числа сущностей — осторожно

- [ ] **`StemCellHierarchyModule` — пластичность** (`plasticity_rate`):
  - При `enable_plasticity = true` и `potency == Oligopotent`:
    вероятность `plasticity_rate` перехода в `Pluripotent` если `spindle_fidelity > 0.6`
  - Имитирует де-дифференцировку при нишевых сигналах

- [ ] **`CellCycleModule` — enforced checkpoints**:
  - G1/S checkpoint: если `total_damage_score() > checkpoint_strictness` → не переходить в S, потенциально апоптоз
  - G2/M checkpoint (spindle assembly): если `spindle_fidelity < 0.5` → не переходить в M
  - Читать поле из `CentriolarDamageState` (standalone) — нет прямой зависимости от `human_development_module`

---

## 4. ОБРАТНЫЕ СВЯЗИ МЕЖДУ МОДУЛЯМИ

- [ ] **Мелоидный сдвиг → DamageParams (через `InflammagingState`)**:
  - Реализовать чтение в `human_development_module.step()` — см. п. 1
  - Это создаёт петлю: повреждение → миелоидный сдвиг → inflammaging → больше ROS → больше повреждений

- [ ] **Транскриптом → клеточный цикл**:
  - `transcriptome_module` обновляет уровни генов `CDKN1A (p21)`, `CDKN2A (p16)`, `MYC`
  - `cell_cycle_module` читает эти уровни из `GeneExpressionState` и модулирует скорость прохождения фаз
  - Конкретно: `p21_level > 0.7` → G1-арест; `p16_level > 0.8` → постоянный арест (сенесценс)

- [ ] **AsymmetricDivision → TissueState**:
  - Высокий `exhaustion_count` в `AsymmetricDivisionComponent` → снижать `tissue_state.stem_cell_pool`
  - Сейчас эти модули не взаимодействуют — добавить чтение в `human_development_module.update_tissue_state()`

- [ ] **MyeloidShift → AgingPhenotype**:
  - `human_development_module.update_aging_phenotypes()` читает `MyeloidShiftComponent` (если присутствует)
  - При `immune_senescence > 0.4` → `active_phenotypes.push(AgingPhenotype::ImmuneDecline)`

---

## 5. НОВЫЕ БИОЛОГИЧЕСКИЕ ТРЕКИ

### Трек C: Теломеры

- [ ] **`TelomereState`** — добавить в `cell_dt_core::components`:
  ```rust
  pub struct TelomereState {
      pub mean_length: f32,         // в единицах T/S ratio [0..1]
      pub shortening_per_division: f32, // default ≈ 0.002
      pub is_critically_short: bool,
  }
  ```
- [ ] **Логика**: каждое деление (`total_divisions +=1`) укорачивает теломеры на `shortening_per_division`
- [ ] **Связь с CDATA**: `spindle_fidelity ↓` → хромосомная нестабильность → ускоренное укорачивание
  (`shortening_rate *= 1 + (1 - spindle_fidelity) × 0.5`)
- [ ] **AgingPhenotype::TelomereShortening** — уже есть в enum, активировать при `mean_length < 0.3`

### Трек D: Эпигенетические часы

- [ ] **`EpigeneticClockState`** — в `cell_dt_core::components`:
  ```rust
  pub struct EpigeneticClockState {
      pub methylation_age: f32,    // биологический возраст по CpG-метилированию
      pub clock_acceleration: f32, // > 1 → часы спешат (стресс, болезнь)
  }
  ```
- [ ] **Модель**: `methylation_age` догоняет `chronological_age` в молодости,
  обгоняет при высоком `total_damage_score`
  ```
  d(methylation_age)/dt = 1.0 × clock_acceleration
  clock_acceleration = 1.0 + total_damage_score × 0.5
  ```
- [ ] **AgingPhenotype::EpigeneticChanges** — уже есть, активировать при
  `methylation_age > chronological_age × 1.1`

### Митохондриальный трек

- [ ] **Новый модуль `mitochondrial_module`** (более долгосрочный):
  - `MitochondrialState { mtdna_mutations: f32, fusion_index: f32, ros_production: f32 }`
  - Питает `ros_level` в `CentriolarDamageState`
  - Митофагия: при `ros_production > threshold` → дефект митофагии → больше дисфункциональных митохондрий
  - Прямая связь с CDATA: митохондриальный щит от O₂ слабеет при `ros_production ↑`

---

## 6. КАЛИБРОВКА И ВЕРИФИКАЦИЯ

- [ ] **Интеграционный тест: 100-летняя симуляция** — написать тест в `cell_dt_core/tests/`:
  - Default params → все ниши умирают в диапазоне 65–95 лет
  - `DamageParams::progeria()` → смерть в 10–20 лет
  - `DamageParams::longevity()` → смерть после 95 лет

- [ ] **Тест калибровки индукторов** — при `base_detach_probability=0.002`:
  - За 78 лет (28 470 шагов при dt=1 день): в среднем ≈8.7 M-отщеплений и ≈5.8 D-отщеплений
  - Проверить, что апоптоз через исчерпание индукторов наступает ПОСЛЕ сенесценса (не раньше)

- [ ] **Тест миелоидного сдвига** — проверить калибровочные ориентиры:
  - t=20 лет: `myeloid_bias < 0.10`
  - t=70 лет: `0.35 < myeloid_bias < 0.60`
  - t=85 лет: `myeloid_bias > 0.55`

- [ ] **`DamageParams::normal_aging()`** — добавить явную фабрику (сейчас только `Default::default()`, `progeria()`, `longevity()`):
  ```rust
  pub fn normal_aging() -> Self { Self::default() } // алиас для ясности
  ```

- [ ] **`stage_history` — ограничить размер VecDeque**:
  - Сейчас неограниченный рост за 100 лет симуляции
  - Добавить `stage_history.truncate(20)` или максимальную ёмкость

---

## 7. ИНФРАСТРУКТУРА И ЭКСПОРТ

- [ ] **CSV-экспорт через `cell_dt_io`** — подключить `DataExporter` к `SimulationManager`:
  - Выгружать каждые `checkpoint_interval` шагов
  - Колонки: `step, entity_id, tissue, age_years, stage, damage_score, myeloid_bias, spindle_fidelity, ciliary_function, frailty, phenotype_count`

- [ ] **Визуализация через `cell_dt_viz`** — добавить:
  - Временной ряд: `damage_score`, `myeloid_bias`, `spindle_fidelity`, `frailty` по оси времени
  - Стрелки трека A и трека B с текущими значениями в режиме реального времени

- [ ] **Python bindings `cell_dt_python`** — экспортировать:
  - `HumanDevelopmentComponent` как PyClass
  - `MyeloidShiftComponent` как PyClass
  - Функцию `run_simulation(params_dict) -> polars.DataFrame`

- [ ] **`cell_dt_gui` — панель управления** — добавить слайдеры для:
  - `base_detach_probability`, `mother_bias`, `age_bias_coefficient`
  - `spindle_weight`, `cilia_weight` (мелоидный сдвиг)
  - `DamageParams` преключатель: normal / progeria / longevity

---

## 8. ТЕХНИЧЕСКИЙ ДОЛГ

- [ ] **Дублирование tissue_type** — `TissueType` (в core) и `HumanTissueType` (в human_development_module)
  конвертируются через `map_tissue_type()` в `lib.rs`. Рассмотреть слияние или перенос `map_tissue_type` в core.

- [ ] **Логирование** — сейчас всё на уровне `debug!`. Добавить:
  - `trace!` для per-entity per-step событий
  - `info!` для milestone (смерть ниши, смена стадии, активация фенотипа)
  - `warn!` для биологически нереалистичных значений (damage > 1.0, myeloid_bias = 1.0)

- [ ] **Параметры `DamageParams` не доступны через панель управления** — добавить `get_params`/`set_params` для `base_ros_damage_rate`, `aggregation_rate`, `senescence_threshold`

- [ ] **`CellCycleStateExtended::new()` используется везде для спавна** — явно задокументировать, что он необходим для инициализации большинства модулей (все модули ищут сущности по наличию этого компонента).

- [ ] **Очистка dead-сущностей** — сейчас `is_alive = false` но сущность остаётся в мире.
  Добавить в `SimulationManager` опциональный проход удаления мёртвых сущностей каждые N шагов.

---

## ПОРЯДОК ВЫПОЛНЕНИЯ (рекомендуемый)

```
✅ 1  InflammagingState + AgingPhenotype::ImmuneDecline
✅ 2  myeloid_shift_module (crate + step + tests + example)
✅ 3  human_dev инициализирует InflammagingState
   4  centriole_module.step() (PTM-накопление)                → п. 3
   5  Обратные связи (inflammaging-петля интеграционный тест) → п. 4
   6  Интеграционные тесты и калибровка                       → п. 6
   7  CellCycleModule enforced checkpoints                    → п. 3
   8  Новые треки (теломеры, эпигенетические часы)            → п. 5
   9  Инфраструктура (CSV, Python, GUI)                       → п. 7
  10  Технический долг                                        → п. 8
```

---

*При каждом выполненном пункте: переместить в секцию "ВЫПОЛНЕНО" вверху, обновить дату.*
*Последнее обновление: 2026-03-03*
