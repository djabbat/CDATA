## Cell DT — TODO / Статус

> Подробный приоритизированный список: см. **RECOMMENDATION.md**
> Последнее обновление: 2026-03-04
> Тесты: **57/57 ✅** | Смерть: ~78 лет ✅ | mBias@70y: 0.57 ✅ | Треки A/B/C/D ✅

---

## ✅ Реализовано

### Ядро платформы (`cell_dt_core`)
- ECS (`hecs`) для управления стволовыми нишами
- `SimulationManager` с чекпоинтами и конфигурацией
- Модульная система через трейт `SimulationModule`
- Полный набор ECS-компонентов:
  - `CentriolarDamageState` (5 молекулярных + 4 аппендажных)
  - `CentriolarInducerPair` (M+D комплекты, `potency_level()`)
  - `CellCycleStateExtended`, `TissueState`, `OrganismState`
  - `InflammagingState` (канал обратной связи myeloid→damage)

### CDATA-ядро (`human_development_module`) ✅
- 15 стадий развития (Zygote → Elderly)
- **Два пути отщепления индукторов:**
  - O₂-путь: `detach_by_oxygen()`, равная вероятность M/D (`mother_bias=0.5`)
  - PTM-путь истощения: `detach_by_ptm_exhaustion()`, только мать (`ptm_exhaustion_scale=0.001`)
- Накопление повреждений: 5 молекулярных типов + 4 аппендажных + ROS-петля
- `DamageParams::normal_aging()` — именованный алиас для `default()`
- Трек A (цилии → регенерация) и Трек B (веретено → пул стволовых)
- **Трек C (теломеры):** `TelomereState` — shortening × spindle_f × ros_f → `AgingPhenotype::TelomereShortening`
- **Трек D (эпигенетика):** `EpigeneticClockState` — `methylation_age += dt × (1 + damage × 0.5)`
- 10 фенотипов старения + `ImmuneDecline` + `TelomereShortening`
- 3 пути смерти: сенесценс / апоптоз через индукторы / критическая дряхлость
- Inflammaging-буст: читает `InflammagingState`, применяет `ros_boost` и `niche_impairment`
- Синхронизация standalone `CentriolarDamageState` каждый step()
- `stage_history`: ограничен последними 20 стадиями (pop_front)
- **17 unit + lifecycle тестов**, включая 4 детерминированных интеграционных
- Калибровка: смерть ≈ 78 лет (normal), прогерия (×5), долгожители (×0.6)

### Транскриптом → Клеточный цикл ✅ NEW
- `GeneExpressionState` (p21, p16, cyclin_d, myc) добавлен в `cell_dt_core`
- `transcriptome_module` пишет CDKN1A/CDKN2A уровни в `GeneExpressionState` каждый step()
- `cell_cycle_module` читает `GeneExpressionState`:
  - p21 > 0.7 → `G1SRestriction` (временный, снимается когда p21 ≤ 0.7)
  - p16 > 0.8 → `DNARepair` (постоянный арест — сенесценция)
  - cyclin_d → укорачивает G1 (`G1_duration / (1 + cyclin_d × 0.5)`)
- 4 новых unit-теста

### AsymmetricDivision → TissueState ✅ NEW
- `DivisionExhaustionState` добавлен в `cell_dt_core` (shared ECS-компонент)
- `asymmetric_division_module` пишет `exhaustion_count` и `asymmetric_count`
- `human_development_module` читает `exhaustion_ratio` → уменьшает `stem_cell_pool`

### PTM → CentriolarDamageState bridge ✅ NEW
- `human_development_module.step()` читает `Option<&CentriolePair>`
- Конвертирует PTM в функциональные повреждения (масштаб 0.002/год при PTM=1.0):
  - `acetylation_level` → `tubulin_hyperacetylation`
  - `oxidation_level` → `protein_carbonylation`
  - `phosphorylation_level` → `phosphorylation_dysregulation`
  - `methylation_level × 0.5` → `protein_aggregates`
- 4 unit-теста: bridge_increases_hyperacetylation, bridge_increases_carbonylation, bridge_zero_with_no_ptm, bridge_scale_is_moderate

### Мониторинг индукторов и теломер (`myeloid_shift_example`) ✅
- Колонки M-ind / ΔM / D-ind / ΔD / Potency / **Tel** в ежегодной таблице
- Дельта за 10-летний интервал: `=` если нет изменений, `+/-N` иначе
- Секция `=== Inductor system ===` в финальном отчёте: remaining/inherited, fraction, division_count
- Калибровка (seed=42): M: 10→3, D: 8→3 за 70 лет; смерть ≈78 лет ✓

### PTM-накопление (`centriole_module`) ✅
- Накопление PTM в `CentriolePair.mother/daughter.ptm_signature`
- Мать накапливает быстрее (daughter_ptm_factor=0.4)
- M-phase boost ×3.0 (тубулин максимально доступен)
- Не трогает `CentriolarDamageState` (двойной счёт исключён)
- 6 unit-тестов

### Миелоидный сдвиг (`myeloid_shift_module`) ✅
- Вычисление `myeloid_bias` из 4 компонент CDATA
- Обратная связь: `InflammagingState { ros_boost, niche_impairment, sasp_intensity }`
- `MyeloidPhenotype` (Healthy / MildShift / ModerateShift / SevereShift)
- 7 unit-тестов, включая калибровочный (возраст 70 лет → bias ≈ 0.45)

### Асимметричные деления (`asymmetric_division_module`) 🟡
- Классификация типа деления: Asymmetric / SelfRenewal / Differentiation
- Читает standalone `CentriolarDamageState`
- Статистика: `asymmetric_count`, `exhaustion_count`

### Иерархия стволовых клеток (`stem_cell_hierarchy_module`) 🟡
- Синхронизация потентности из `spindle_fidelity`
- Фабрики: embryonic / hematopoietic / neural stem cell

### Клеточный цикл (`cell_cycle_module`) ✅
- Прогрессия фаз G1/S/G2/M с временными длительностями
- Учёт стресса и факторов роста
- **G1/S checkpoint**: арест при `total_damage_score() > checkpoint_strictness`
- **G2/M checkpoint (SAC)**: арест при `spindle_fidelity < (1 - checkpoint_strictness)`
- **p21 > 0.7** → временный G1SRestriction (снимается при p21 ≤ 0.7)
- **p16 > 0.8** → постоянный DNARepair (сенесценция)
- **cyclin_d** → укорачивает G1 (`G1_duration / (1 + cyclin_d × 0.5)`)
- **Предел Хейфлика** → `TelomereState.is_critically_short` → постоянный G1SRestriction
- Growth factors синхронизированы с `CentriolarDamageState`
- **14 unit-тестов** (включая 4 теста Hayflick + 4 теста GeneExpressionState)

### Транскриптом (`transcriptome_module`) 🟡
- Экспрессия генов, транскрипционные факторы, сигнальные пути
- Взаимодействие с центриолью (частичное)

### Инфраструктура
- `cell_dt_io` — CSV-экспорт
- `cell_dt_viz` — 2D/3D визуализация
- `cell_dt_config` — TOML/YAML конфигурация
- `cell_dt_gui` — GUI (egui, частичный)
- `cell_dt_python` — PyO3-биндинги (каркас)

---

## 🔧 Следующие шаги (по приоритету)

1. **Спавн дочерних сущностей** (`asymmetric_division_module`) — создание новых entity при делении
2. **StemCellHierarchy пластичность** — де-дифференцировка при `enable_plasticity=true`
3. **CSV-экспорт** — подключить `DataExporter` к `SimulationManager`
4. **Python-биндинги** — `cell_dt_python` (PyO3, `run_simulation() → DataFrame`)

## ⬜ Долгосрочные планы

- Митохондриальный модуль (`mitochondrial_module`) — `MitochondrialState`, mtDNA, питает ros_level
- Python биндинги (`cell_dt_python`) — `run_simulation() → DataFrame`
- GUI панель управления — слайдеры для DamageParams, InducerDetachmentParams
- Очистка мёртвых сущностей из ECS-мира

---

## 📊 Полезные команды

```bash
# CDATA — 100 лет с миелоидным сдвигом
cargo run --bin myeloid_shift_example

# CDATA — 100 лет, полный вывод
cargo run --bin human_development_example

# Стволовые клетки
cargo run --bin stem_cell_example

# Клеточный цикл
cargo run --bin cell_cycle_example
cargo run --bin cell_cycle_advanced

# Транскриптом
cargo run --bin transcriptome_example

# I/O
cargo run --bin io_example

# Все тесты (57 тестов)
cargo test

# Документация
cargo doc --open

# С подробным логом
RUST_LOG=debug cargo run --bin myeloid_shift_example
```
