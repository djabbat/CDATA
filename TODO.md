## Cell DT — TODO / Статус

> Подробный приоритизированный список: см. **RECOMMENDATION.md**
> Последнее обновление: 2026-03-03

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

### Миелоидный сдвиг (`myeloid_shift_module`) ✅ NEW
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

### Клеточный цикл (`cell_cycle_module`) 🟡
- Прогрессия фаз G1/S/G2/M с временными длительностями
- Учёт стресса и факторов роста

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

1. **`centriole_module.step()`** — реализовать PTM-накопление напрямую в standalone `CentriolarDamageState`
2. **`CellCycleModule` checkpoints** — G1/S: арест при `damage > threshold`; G2/M: арест при `spindle < 0.5`
3. **Inflammaging → DamageParams** — читать `InflammagingState` уже реализовано; проверить петлю на интеграционном тесте
4. **AsymmetricDivisionModule — спавн дочерних сущностей** — при Asymmetric → `world.spawn()` новой сущности с унаследованным `CentriolarInducerPair`
5. **Транскриптом → Клеточный цикл** — Cyclin D уровни из `GeneExpressionState` → длительность G1

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

# Все тесты (25 тестов)
cargo test

# Документация
cargo doc --open

# С подробным логом
RUST_LOG=debug cargo run --bin myeloid_shift_example
```
