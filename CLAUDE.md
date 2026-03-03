# Cell DT — CLAUDE.md

Rust-платформа для симуляции клеточного старения на основе теории CDATA
(Centriolar Damage Accumulation Theory of Aging, Jaba Tkemaladze).

---

## Архитектура

### ECS и модули
- **ECS:** крейт `hecs` — каждая сущность (entity) = одна стволовая ниша
- **Модули:** трейт `SimulationModule` (`name`, `step`, `initialize`, `get_params`, `set_params`)
- **Оркестрация:** `SimulationManager` — последовательный вызов всех зарегистрированных модулей
- **Параллелизм:** Rayon через `SimulationConfig::num_threads`

### Воркспейс (14 крейтов)
```
cell_dt_core                   ← ECS, компоненты, трейт модуля
cell_dt_io                     ← CSV-экспорт
cell_dt_viz                    ← 2D/3D визуализация
cell_dt_config                 ← TOML/YAML конфигурация
cell_dt_gui                    ← GUI (egui)
cell_dt_python                 ← PyO3-биндинги
cell_dt_modules/
  centriole_module             ← параметры повреждений (step() — заглушка)
  cell_cycle_module            ← прогрессия фаз G1/S/G2/M
  transcriptome_module         ← экспрессия генов, TF, сигнальные пути
  asymmetric_division_module   ← тип деления из потентности + spindle_fidelity
  stem_cell_hierarchy_module   ← иерархия потентности (синхр. с CDA)
  human_development_module     ← ядро CDATA: повреждения → ткань → смерть
  myeloid_shift_module         ← миелоидный сдвиг + InflammagingState обратная связь
examples                       ← бинарные примеры
```

---

## Ключевые типы (cell_dt_core/src/components.rs)

### Индукторная система
```rust
CentrioleInducerSet       // один комплект на одну центриоль
CentriolarInducerPair     // пара M+D; метод potency_level() → PotencyLevel
InducerDetachmentParams   // mother_bias, base_detach_probability, age_bias_coefficient
PotencyLevel              // Totipotent | Pluripotent | Oligopotent | Unipotent | Apoptosis
```

### Повреждения и воспаление
```rust
CentriolarDamageState     // 5 молекулярных + 4 аппендажных + производные метрики
                          // (standalone ECS-компонент, синхронизируется из HumanDevelopmentComponent каждый step)
InflammagingState         // канал обратной связи myeloid_shift_module → human_development_module
                          // поля: ros_boost, niche_impairment, sasp_intensity
```

### Цикл, ткань, организм
```rust
CellCycleStateExtended    // фаза, прогресс, циклины, чекпоинты, growth_factors
TissueState               // stem_cell_pool, regeneration_tempo, senescent_fraction
OrganismState             // age_years, frailty_index, cognitive_index, is_alive
```

---

## Теория CDATA — логика симуляции

### Индукторы и потентность
- У материнской центриоли — комплект **M**; у дочерней — комплект **D** (разные молекулы)
- **O₂**, проникая к центриолям, необратимо отщепляет индукторы
- При делении новая дочерняя центриоль получает индукторы в **текущем** количестве родителя
- Потентность = состояние обоих комплектов:

```
M=полный И D=полный → Totipotent
M≥1 И D≥1          → Pluripotent
Одна = 0, другая ≥2 → Oligopotent
Одна = 0, другая = 1 → Unipotent
M=0 И D=0           → Apoptosis (запрограммированный)
```

### Кислородный барьер
- Митохондрии у периферии клетки поглощают O₂ → центриоли защищены
- Повреждения (ROS, агрегаты) ослабляют щит → O₂ проникает к центру
- `centrosomal_oxygen_level(damage)` = `1 − mito_shield`
- `mito_shield` = `1 − ros×0.5 − aggregates×0.3 − carbonylation×0.2`

### Асимметрия M/D
- Материнская центриоль старше → больше ПТМ → связи слабее → теряет индукторы чаще
- Параметр `mother_bias` ([0..1]) управляет соотношением вероятностей
- Возраст через `age_bias_coefficient` лишь корректирует `mother_bias`, не является причиной

### Три трека старения
| Трек | Механизм | Маркер |
|------|----------|--------|
| A (цилии) | CEP164↓ → Shh/Wnt↓ → нет самообновления | `ciliary_function → regeneration_tempo` |
| B (веретено) | spindle_fidelity↓ → симм. деление → истощение пула | `pool_exhaustion_probability()` |
| C (миелоидный) | spindle↓ + cilia↓ + ROS↑ → PU.1 > Ikaros → сдвиг к миелоиду | `myeloid_bias → inflammaging_index` |

### Калибровка (DamageParams::default)
- `senescence_threshold = 0.75` → смерть ≈ 78 лет
- `midlife_damage_multiplier = 1.6` после 40 лет (антагонистическая плейотропия)
- `ros_feedback_coefficient = 0.12` (петля обратной связи)
- Варианты: `DamageParams::progeria()` (×5), `DamageParams::longevity()` (×0.6)

### Миелоидный сдвиг (myeloid_shift_module)
```
myeloid_bias = (1-spindle)^1.5×0.45 + (1-cilia)×0.30 + ros×0.15 + aggregates×0.10

Обратная связь → InflammagingState → human_development_module:
  ros_boost        = inflammaging_index × 0.15  → ускоряет ros_level
  niche_impairment = inflammaging_index × 0.08  → снижает regeneration_tempo
  sasp_intensity   → активирует AgingPhenotype::ImmuneDecline при > 0.4
```

Калибровка: возраст 70 лет → `myeloid_bias ≈ 0.45` (ModerateShift).

---

## Параметры панели управления

### human_development_module
| Параметр | Тип | По умолч. | Описание |
|----------|-----|-----------|----------|
| `time_acceleration` | f64 | 1.0 | Шагов в день |
| `mother_inducer_count` | u32 | 10 | Начальный M-комплект |
| `daughter_inducer_count` | u32 | 8 | Начальный D-комплект |
| `base_detach_probability` | f32 | 0.002 | Базовая вероятность отщепления |
| `mother_bias` | f32 | 0.6 | Доля от M при O₂-воздействии |
| `age_bias_coefficient` | f32 | 0.003 | Вклад возраста в mother_bias |
| `enable_aging` | bool | true | Включить накопление повреждений |
| `enable_morphogenesis` | bool | true | Включить стадии развития |

### myeloid_shift_module
| Параметр | По умолч. | Описание |
|----------|-----------|----------|
| `spindle_weight` | 0.45 | Вклад spindle_fidelity в myeloid_bias |
| `cilia_weight` | 0.30 | Вклад ciliary_function |
| `ros_weight` | 0.15 | Вклад ros_level |
| `aggregate_weight` | 0.10 | Вклад protein_aggregates |
| `ros_boost_scale` | 0.15 | Масштаб обратной связи → ROS |
| `niche_impair_scale` | 0.08 | Масштаб обратной связи → нише |

---

## Команды

```bash
# CDATA — 100-летняя симуляция с индукторами
cargo run --bin human_development_example

# CDATA + миелоидный сдвиг — 100 лет, вывод myeloid_bias каждые 10 лет
cargo run --bin myeloid_shift_example

# Стволовые клетки + асимметричные деления
cargo run --bin stem_cell_example

# Клеточный цикл
cargo run --bin cell_cycle_example
cargo run --bin cell_cycle_advanced

# Транскриптом
cargo run --bin transcriptome_example

# I/O
cargo run --bin io_example

# Производительность
cargo run --bin performance_test

# Все тесты (25 тестов)
cargo test

# Документация
cargo doc --open

# С подробным логом
RUST_LOG=debug cargo run --bin human_development_example
```

---

## Статус модулей

| Модуль | Статус | Примечание |
|--------|--------|------------|
| `human_development_module` | ✅ Полный | Ядро CDATA, 3 пути смерти, inflammaging-петля |
| `myeloid_shift_module` | ✅ Полный | myeloid_bias, InflammagingState, 7 unit-тестов |
| `cell_cycle_module` | 🟡 Частичный | Фазы работают; чекпоинты не форсируют арест |
| `transcriptome_module` | 🟡 Частичный | Гены, TF, пути; нет обратной связи → цикл |
| `asymmetric_division_module` | 🟡 Частичный | Классификация типа деления; нет спавна новых сущностей |
| `stem_cell_hierarchy_module` | 🟡 Частичный | Синхронизация потентности; нет полной пластичности |
| `centriole_module` | 🔴 Заглушка | step() не применяет damage rates к ECS |

---

## Незавершённые части

1. **`centriole_module.step()`** — пустой; должен применять `damage.rs` скорости напрямую к standalone `CentriolarDamageState`
2. **Форсирование чекпоинтов** — `cell_cycle_module` видит чекпоинты, но не останавливает клетку
3. **Спавн дочерних сущностей** — `asymmetric_division_module` классифицирует деление, но не создаёт новые entity
4. **Транскриптом → Клеточный цикл** — Cyclin D/E уровни из `GeneExpressionState` не влияют на длительность G1
5. **Python-биндинги** — `cell_dt_python/src/lib.rs` не реализован
6. **Теломерный трек** — нет `TelomereState` ECS-компонента
7. **Эпигенетические часы** — нет `EpigeneticClockState`

Полный список: см. **RECOMMENDATION.md**.

---

## Структура компонентов одной сущности

```
Entity (стволовая ниша)
├── CentriolePair                 ← структура центриолей, MTOC, цилии
├── CentriolarDamageState         ← standalone; 5 молекулярных + 4 аппендажных
│                                   синхронизируется из HumanDevelopmentComponent каждый step()
├── InflammagingState             ← канал обратной связи myeloid→damage; ros_boost, niche_impairment
├── CellCycleStateExtended        ← фаза, прогресс, циклины/CDK, чекпоинты
├── HumanDevelopmentComponent     ← CDATA: стадия, возраст, damage, inducers, ткань
├── MyeloidShiftComponent         ← myeloid_bias, lymphoid_deficit, inflammaging_index, phenotype
├── StemCellHierarchyState        ← потентность (синхр. со spindle_fidelity)
└── AsymmetricDivisionComponent   ← тип деления, niche_id, stemness_potential
```

---

## Порядок регистрации модулей (важно!)

```rust
sim.register_module(Box::new(CentrioleModule::...));          // 1
sim.register_module(Box::new(CellCycleModule::...));          // 2
sim.register_module(Box::new(HumanDevelopmentModule::...));   // 3 — синхр. CentriolarDamageState
sim.register_module(Box::new(MyeloidShiftModule::...));       // 4 — читает CDA, пишет InflammagingState
sim.register_module(Box::new(StemCellHierarchyModule::...));  // 5 — читает CDA
sim.register_module(Box::new(AsymmetricDivisionModule::...)); // 6 — читает CDA
```

`HumanDevelopmentModule` должен быть перед `MyeloidShiftModule`, так как он синхронизирует
standalone `CentriolarDamageState`. `MyeloidShiftModule` должен быть перед `HumanDevelopmentModule`
на **следующем** шаге (допустим лаг в один шаг для обратной связи через `InflammagingState`).

---

## Автор
Jaba Tkemaladze — теория CDATA, архитектура симуляции.
