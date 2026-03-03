## Cell DT — TODO / Статус

> Подробный приоритизированный список: см. **RECOMMENDATION.md**
> Последнее обновление: 2026-03-04

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
- O₂-зависимое отщепление индукторов (M/D комплекты)
- Накопление повреждений: 5 молекулярных типов + 4 аппендажных + ROS-петля
- Трек A (цилии → регенерация) и Трек B (веретено → пул стволовых)
- 10 фенотипов старения + `ImmuneDecline`
- 3 пути смерти: сенесценс / апоптоз через индукторы / критическая дряхлость
- Inflammaging-буст: читает `InflammagingState`, применяет `ros_boost` и `niche_impairment`
- Синхронизация standalone `CentriolarDamageState` каждый step()
- Калибровка: смерть ≈ 78 лет (normal), прогерия (×5), долгожители (×0.6)

### PTM-накопление (`centriole_module`) ✅ NEW
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
- Growth factors синхронизированы с `CentriolarDamageState`
- 6 unit-тестов

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

1. **AsymmetricDivisionModule — спавн дочерних сущностей** — при Asymmetric → `world.spawn()` новой сущности с унаследованным `CentriolarInducerPair`
2. **Транскриптом → Клеточный цикл** — Cyclin D уровни из `GeneExpressionState` → длительность G1 (`p21 > 0.7` → G1-арест, `p16 > 0.8` → постоянный арест)
3. **AsymmetricDivision → TissueState** — высокий `exhaustion_count` → снижать `stem_cell_pool` в `human_development_module.update_tissue_state()`
4. **StemCellHierarchyModule — пластичность** — при `enable_plasticity=true` и `Oligopotent` + `spindle_fidelity > 0.6` → вероятность перехода в `Pluripotent`

## ⬜ Долгосрочные планы

- Теломерный Трек C (`TelomereState`)
- Эпигенетические часы Трек D (`EpigeneticClockState`)
- Митохондриальный модуль (`mitochondrial_module`)
- Python биндинги (`cell_dt_python`) — `run_simulation() → DataFrame`
- GUI панель управления — слайдеры для всех параметров

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

# Все тесты (37 тестов)
cargo test

# Документация
cargo doc --open

# С подробным логом
RUST_LOG=debug cargo run --bin myeloid_shift_example
```
