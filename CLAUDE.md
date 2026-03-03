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
  centriole_module             ← PTM-накопление на CentriolePair (mother/daughter, M-phase boost)
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
DivisionExhaustionState   // канал обратной связи asymmetric_division_module → human_development_module
                          // поля: exhaustion_count, asymmetric_count, total_divisions, exhaustion_ratio()
GeneExpressionState       // канал обратной связи transcriptome_module → cell_cycle_module
                          // поля: p21_level, p16_level, cyclin_d_level, myc_level
TelomereState             // Трек C; пишет human_development_module, читает cell_cycle_module
                          // поля: mean_length [0..1], shortening_per_division, is_critically_short
EpigeneticClockState      // Трек D; пишет human_development_module
                          // поля: methylation_age, clock_acceleration = 1 + damage × 0.5
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

### Четыре трека старения
| Трек | Механизм | Маркер |
|------|----------|--------|
| A (цилии) | CEP164↓ → Shh/Wnt↓ → нет самообновления | `ciliary_function → regeneration_tempo` |
| B (веретено) | spindle_fidelity↓ → симм. деление → истощение пула | `pool_exhaustion_probability()` |
| C (теломеры) | укорачивание = per_div × div_rate × spindle_f × ros_f → Хейфлик G1-арест | `TelomereState.is_critically_short → G1SRestriction` |
| D (эпигенетика) | methylation_age += dt × (1 + damage × 0.5) | `EpigeneticClockState.clock_acceleration` |
| (мелоидный) | spindle↓ + cilia↓ + ROS↑ → PU.1 > Ikaros → сдвиг к миелоиду | `myeloid_bias → inflammaging_index` |

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
| `base_detach_probability` | f32 | 0.002 | Базовая вероятность O₂-отщепления |
| `mother_bias` | f32 | 0.5 | Доля от M при O₂-воздействии (0.5 = равные M/D) |
| `age_bias_coefficient` | f32 | 0.0 | Вклад возраста в mother_bias (0.0 = не влияет) |
| `ptm_exhaustion_scale` | f32 | 0.001 | PTM-путь истощения матери (независим от O₂) |
| `enable_aging` | bool | true | Включить накопление повреждений |
| `enable_morphogenesis` | bool | true | Включить стадии развития |

### centriole_module
| Параметр | По умолч. | Описание |
|----------|-----------|----------|
| `acetylation_rate` | 0.0002 | Скорость ацетилирования тубулина (мать) / шаг |
| `oxidation_rate` | 0.0001 | Скорость окисления (мать) / шаг |
| `methylation_rate` | 0.00005 | Скорость метилирования (мать) / шаг |
| `phosphorylation_rate` | 0.0001 | Скорость фосфорилирования (мать) / шаг |
| `daughter_ptm_factor` | 0.4 | Дочерняя центриоль накапливает PTM в этой доле от материнской |
| `m_phase_boost` | 3.0 | Множитель PTM в M-фазе (максимальный стресс тубулина) |

### cell_cycle_module
| Параметр | По умолч. | Описание |
|----------|-----------|----------|
| `base_cycle_time` | 24.0 | Базовая длительность полного цикла (ч) |
| `checkpoint_strictness` | 0.0 | Порог арестов: 0=нет, 0.3=умеренный, 0.7=строгий |
| `growth_factor_sensitivity` | 0.3 | Чувствительность к факторам роста |
| `stress_sensitivity` | 0.2 | Чувствительность к стрессу |
| `enable_apoptosis` | true | Применять апоптоз при аресте |

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

# Все тесты (57 тестов)
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
| `human_development_module` | ✅ Полный | Ядро CDATA, 4 трека (A/B/C/D), PTM exhaustion, 17 тестов |
| `myeloid_shift_module` | ✅ Полный | myeloid_bias, InflammagingState, 7 тестов |
| `cell_cycle_module` | ✅ Полный | Фазы + checkpoints + Hayflick (TelomereState), 14 тестов |
| `centriole_module` | ✅ Полный | PTM-накопление в CentriolePair, M-phase boost, 6 тестов |
| `transcriptome_module` | 🟡 Частичный | Гены CDKN1A/CDKN2A + GeneExpressionState sync; нет обратной связи циклины→пролиферация |
| `asymmetric_division_module` | 🟡 Частичный | Классификация деления + DivisionExhaustionState; нет спавна дочерних сущностей |
| `stem_cell_hierarchy_module` | 🟡 Частичный | Синхронизация потентности из spindle_fidelity; нет пластичности (де-дифференцировки) |

---

## Незавершённые части

1. **Спавн дочерних сущностей** — `asymmetric_division_module` классифицирует деление, но не создаёт новые entity
2. **StemCellHierarchy пластичность** — де-дифференцировка при `enable_plasticity=true`
3. **Python-биндинги** — `cell_dt_python/src/lib.rs` не реализован
4. **Транскриптом → CellCycle через Cyclin D/E** — уровни из GeneExpressionState влияют на G1, но нет полной петли
5. **CSV/Viz экспорт** — инфраструктура есть, не подключена к SimulationManager

Полный список: см. **RECOMMENDATION.md**.

---

## Структура компонентов одной сущности

```
Entity (стволовая ниша)
├── CentriolePair                 ← структура центриолей, MTOC, цилии; ptm_signature (мать/дочь)
├── CentriolarDamageState         ← standalone; 5 молекулярных + 4 аппендажных
│                                   синхронизируется из HumanDevelopmentComponent каждый step()
├── InflammagingState             ← канал обратной связи myeloid→damage; ros_boost, niche_impairment
├── GeneExpressionState           ← p21/p16/cyclin_d/myc; читается cell_cycle_module
├── DivisionExhaustionState       ← exhaustion_count/asymmetric_count; читается human_dev
├── CellCycleStateExtended        ← фаза, прогресс, циклины/CDK, чекпоинты
├── HumanDevelopmentComponent     ← CDATA: стадия, возраст, damage, inducers, ткань
├── MyeloidShiftComponent         ← myeloid_bias, lymphoid_deficit, inflammaging_index, phenotype
├── StemCellHierarchyState        ← потентность (синхр. со spindle_fidelity)
├── AsymmetricDivisionComponent   ← тип деления, niche_id, stemness_potential
├── TelomereState                 ← mean_length [0..1], shortening_per_division, is_critically_short
└── EpigeneticClockState          ← methylation_age, clock_acceleration
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
