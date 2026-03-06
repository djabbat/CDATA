# CDATA — Human Lifecycle Simulation

**Centriolar Damage Accumulation Theory of Aging**
Implementation on the Cell DT Platform (Rust)

Theory author: Jaba Tkemaladze (2005–2026)

---

## Table of Contents

1. [The CDATA Hypothesis](#1-the-cdata-hypothesis)
2. [Key Mechanisms](#2-key-mechanisms)
3. [Module Architecture](#3-module-architecture)
4. [Core Components](#4-core-components)
5. [human_development_module](#5-human_development_module)
6. [Developmental Stages](#6-developmental-stages)
7. [Parameters and Calibration](#7-parameters-and-calibration)
8. [Running the Simulation](#8-running-the-simulation)
9. [Results and Interpretation](#9-results-and-interpretation)
10. [References](#10-references)

---

## 1. The CDATA Hypothesis

**CDATA (Centriolar Damage Accumulation Theory of Aging)** is a theory that
explains organismal aging as an inevitable consequence of the cell
differentiation program.

The central claim: **the mother centriole of a stem cell is the only biological
structure that irreversibly accumulates molecular damage throughout the entire
lifespan of an organism**, because:

1. The centriole replicates by a template mechanism — the old (mother) centriole
   is never fully rebuilt from scratch.
2. During asymmetric division of a stem cell, the daughter that retains
   stem identity **always inherits the old mother centriole**.
3. This means all chronologically accumulated molecular damage remains inside
   the stem cell — while the replaced differentiated cells receive relatively
   "fresh" centrioles.

**The paradox of aging:** tissues are continuously renewed by stem cells, yet
the organism still ages — precisely because stem cells carry an ever-growing
burden of damage in their mother centrioles.

---

## 2. Key Mechanisms

### 2.1 The Differentiation Inducer System

According to the theory (Tkemaladze 2005/2023), the centriole contains two sets
of **irreversible differentiation inducers**:

| Structure | Function | Initial count |
|-----------|----------|---------------|
| **S-structure** | Somatic lineage | ~50 inducers (≈ Hayflick limit) |
| **H-structure** | Germline | ~4 inducers (until meiosis) |

At each **differentiation-inducing mitosis**, one S-inducer is released from the
centriole, enters the nucleus, switches off one gene network and activates
another — this is the irreversible change in morphogenetic status.

```
S_count = 50  →  totipotent (zygote)
S_count = 40  →  pluripotent (ICM of blastocyst)
S_count = 25  →  multipotent (adult stem cells)
S_count = 10  →  unipotent
S_count = 0   →  terminal differentiation / apoptosis
```

### 2.2 Accumulation of Molecular Damage

Five types of irreversible damage accumulate in the mother centriole:

```
STRESSORS (ROS, UV, metabolic by-products)
        │
        ▼
MOTHER CENTRIOLE — the non-renewable keeper of time
        │
        ├── Protein carbonylation (SAS-6, CEP135)
        │     → loss of structural integrity, PCM destabilisation
        │
        ├── α-tubulin hyperacetylation (↓ HDAC6, ↓ SIRT2)
        │     → reduced microtubule flexibility, impaired transport
        │
        ├── Protein aggregates (CPAP, CEP290)
        │     → block of the duplication machinery, PCM disorganisation
        │
        ├── Phosphorylation dysregulation (PLK4, NEK2, PP1)
        │     → ectopic amplification or failure of centriole disengagement
        │
        └── Loss of distal appendages (CEP164, CEP89, Ninein, CEP170)
              → inability to assemble the primary cilium
```

### 2.3 Two Parallel Pathways to Pathology

Once damage accumulates, the centriole loses two critical functions:

**Track A — ciliary failure ("centriolar blindness"):**
```
Loss of CEP164/CEP89 → no primary cilium
→ stem cell is deaf to Shh and Wnt niche signals
→ wrong cell-fate decisions
```

**Track B — failure of asymmetric cell division (ACD):**
```
PCM degradation → weak, misoriented mitotic spindle
→ random segregation of fate determinants (Numb, Prominin-1)
→ either pool exhaustion (both daughters differentiate)
→ or clonal hyperproliferation (both daughters remain stem cells)
```

### 2.4 The Positive Feedback Loop

```
Damaged centriole
    │
    ▼
Microtubule disorganisation
    │
    ▼
Impaired mitophagy → mitochondrial dysfunction
    │
    ▼
↑ ROS → more centriolar damage (self-amplifying cycle)
```

Additional loops:
- Mislocalisation of HDAC6/SIRT2 → global epigenetic alterations
- False DDR signal (BRCA1/53BP1 displaced) → p53/p21 → senescence

---

## 3. Module Architecture

```
cell_dt/
├── crates/
│   ├── cell_dt_core/
│   │   └── src/components.rs          # New CDATA components
│   └── cell_dt_modules/
│       └── human_development_module/
│           └── src/
│               ├── lib.rs             # Main module (SimulationModule)
│               ├── damage.rs          # Centriolar damage accumulation
│               ├── inducers.rs        # S/H inducer system
│               ├── development.rs     # Developmental stages and rates
│               ├── tissues.rs         # Tissue-specific stem niches
│               └── organism.rs        # Organism-level integration
└── examples/
    └── src/bin/
        └── human_lifecycle.rs         # Simulation from zygote to death
```

---

## 4. Core Components

Added to `cell_dt_core::components`:

### `DevelopmentalStage`

Enum of developmental stages:

| Stage | Age | Notes |
|-------|-----|-------|
| `Zygote` | Day 0 | No centrioles, totipotency |
| `Cleavage` | Days 1–4 | De novo centriole formation |
| `Blastocyst` | Days 4–14 | ICM (pluripotent) vs trophoblast |
| `Gastrulation` | Days 14–28 | Three germ layers |
| `Organogenesis` | Days 28–56 | Organ formation |
| `Fetal` | 2–9 months | Fetal period |
| `Postnatal` | 0–18 years | Growth and development |
| `Adult` | 18–40 years | Tissue homeostasis |
| `MiddleAge` | 40–65 years | Accelerating damage |
| `Senescent` | 65+ years | Pronounced aging |
| `Death` | — | Organismal death |

### `CentriolarInducers`

```rust
pub struct CentriolarInducers {
    pub s_count: u32,              // Remaining S-inducers
    pub s_max:   u32,              // Initial stock
    pub h_count: u32,              // Remaining H-inducers
    pub h_max:   u32,
    pub differentiation_divisions: u32,  // Division counter
}
```

Key methods:
- `s_status() -> f32` — morphogenetic status [0 = totipotent, 1 = terminal]
- `consume_s_inducer()` — spend one S-inducer on differentiation
- `is_terminally_differentiated()` — returns true when S_count = 0

### `CentriolarDamageState`

```rust
pub struct CentriolarDamageState {
    // Molecular damage
    pub protein_carbonylation:         f32,  // SAS-6, CEP135
    pub tubulin_hyperacetylation:      f32,  // α-tubulin Lys40
    pub protein_aggregates:            f32,  // CPAP, CEP290
    pub phosphorylation_dysregulation: f32,  // PLK4, NEK2, PP1

    // Appendage integrity [0..1]
    pub cep164_integrity: f32,
    pub cep89_integrity:  f32,
    pub ninein_integrity: f32,
    pub cep170_integrity: f32,

    // Derived functional metrics
    pub ciliary_function: f32,  // Primary cilium function
    pub spindle_fidelity: f32,  // Mitotic spindle accuracy
    pub ros_level:        f32,  // ROS level (feedback loop)
    pub is_senescent:     bool,
}
```

### `TissueState`

Aggregated regenerative metrics for a tissue:

```rust
pub struct TissueState {
    pub tissue_type:         TissueType,
    pub stem_cell_pool:      f32,  // Stem cell pool size [0..1]
    pub regeneration_tempo:  f32,  // Regeneration rate [0..1]
    pub senescent_fraction:  f32,  // Fraction of senescent cells [0..1]
    pub functional_capacity: f32,  // Tissue functional capacity [0..1]
}
```

### `OrganismState`

Global organism-level metrics:

```rust
pub struct OrganismState {
    pub age_years:           f64,
    pub developmental_stage: DevelopmentalStage,
    pub inflammaging_score:  f32,  // Systemic inflammation [0..1]
    pub frailty_index:       f32,  // Frailty index [0..1]
    pub cognitive_index:     f32,  // Cognitive reserve [0..1]
    pub immune_reserve:      f32,  // Immune reserve [0..1]
    pub muscle_mass:         f32,  // Muscle mass (sarcopenia) [0..1]
    pub is_alive:            bool,
}
```

---

## 5. human_development_module

### `damage.rs` — Damage Accumulation

The `accumulate_damage()` function implements all five damage types per time
step `dt_years`. Key features:

- **Age multiplier**: after age 40 the damage rate is multiplied by
  `midlife_damage_multiplier` (antagonistic pleiotropy)
- **ROS feedback loop**: `ros_boost = 1 + α × total_damage_score()`
- **Appendage loss is irreversible**: `cep164_integrity` only decreases, never
  recovers
- **Derived metrics** are updated automatically:
  ```
  ciliary_function = mean(appendages) × (1 − aggregates × 0.5)
  spindle_fidelity = (1 − structural_damage) × (1 − phospho × 0.3)
  ```

### `inducers.rs` — S/H Inducer System

Trait `InducerDivisionExt` for `CentriolarInducers`:

```rust
pub trait InducerDivisionExt {
    fn morphogenetic_level(&self) -> MorphogeneticLevel;
    fn asymmetric_divide(&mut self, spindle_ok: bool, rng_val: f32) -> DivisionOutcome;
}

pub enum DivisionOutcome {
    Asymmetric { stem_daughter, differentiating_daughter },
    SymmetricDifferentiation,  // pool exhaustion
    SymmetricSelfRenewal,      // clonal expansion
    TerminalDifferentiation,   // S_count = 0
}
```

When `spindle_ok = false` (damaged spindle), the division has a 0.5 probability
of becoming symmetric — either exhaustion or clonal expansion.

### `tissues.rs` — Tissue-Specific Niches

Each tissue has a unique `TissueProfile`:

| Tissue | Damage multiplier | Ciliary sensitivity | Biological basis |
|--------|------------------|---------------------|-----------------|
| `Hematopoietic` | ×1.3 | 0.9 | HSC: myeloid bias with age |
| `Neural` | ×0.8 | 1.3 | NSC: critical dependence on Shh cilium |
| `IntestinalCrypt` | ×1.2 | 0.7 | High division rate → pool exhausted faster |
| `Muscle` | ×0.9 | 0.8 | Satellite cells: moderate degradation |
| `Skin` | ×1.1 | 0.6 | Epithelium: moderate damage |
| `Germline` | ×0.5 | 1.0 | Germ cells: protected |

### `organism.rs` — Metric Integration

Mapping of tissue metrics → global organism indicators:

```
Neural.functional_capacity          →  cognitive_index
HSC.functional_capacity             →  immune_reserve
Muscle.functional_capacity          →  muscle_mass
(gut + HSC).senescent_fraction / 2  →  inflammaging_score
mean(all tissues).functional_capacity →  frailty_index
```

Death occurs when `frailty_index ≥ 0.95` or `max_lifespan_years` is reached.

---

## 6. Developmental Stages

```
ZYGOTE (day 0)
  No centrioles → totipotency
  S_count = 50, H_count = 4
  Minimal ROS
         │
         ▼  de novo centriole formation
CLEAVAGE (days 1–4)
  Newly formed centrioles — no accumulated damage
  ~2 divisions/day
         │
         ▼
BLASTOCYST (days 4–14)
  ICM: pluripotent cells, S_count ~40
  Trophoblast: onset of differentiation
         │
         ▼
GASTRULATION → ORGANOGENESIS (weeks 2–8)
  Three germ layers, organ formation
  First stem cell niches established
         │
         ▼
FETAL (8 weeks – 9 months)
  Rapid stem cell proliferation
  Centriolar damage remains minimal
         │
         ▼
POSTNATAL (0–18 years)
  Active growth → high division rate
  Slow linear damage accumulation begins
         │
         ▼
ADULT (18–40 years)
  Tissue homeostasis, stable regeneration
  Damage accumulates linearly
         │
         ▼  [key CDATA inflection point]
MIDDLE AGE (40–65 years)
  Antagonistic pleiotropy: ×1.6 damage rate
  HSC: onset of myeloid bias
  SASP activation
  Immunosenescence, inflammaging
         │
         ▼
SENESCENT (65+ years)
  Pronounced sarcopenia, immunodeficiency, cognitive decline
  HSC, intestinal, and skin pools exhausted
  Unstable spindle → clonal haematopoietic expansion
         │
         ▼
DEATH (~76–80 years, normal mode)
  Frailty index ≥ 0.95
  Residual function: Neural (cognition preserved the longest)
```

---

## 7. Parameters and Calibration

### Damage Parameters (`DamageParams`)

| Parameter | Default | Meaning |
|-----------|---------|---------|
| `base_ros_damage_rate` | 0.0018 /yr | Carbonylation of SAS-6, CEP135 |
| `acetylation_rate` | 0.0014 /yr | α-tubulin hyperacetylation |
| `aggregation_rate` | 0.0014 /yr | CPAP, CEP290 aggregates |
| `phospho_dysregulation_rate` | 0.0010 /yr | PLK4/NEK2/PP1 imbalance |
| `cep164_loss_rate` | 0.0027 /yr | CEP164 loss (cilium initiation) |
| `ninein_loss_rate` | 0.0020 /yr | Ninein loss (subdistal appendages) |
| `ros_feedback_coefficient` | 0.12 | Damage → ROS loop strength |
| `midlife_damage_multiplier` | 1.6 | Acceleration factor after age 40 |
| `senescence_threshold` | 0.75 | Damage threshold for senescence entry |

### Development Parameters (`DevelopmentParams`)

| Parameter | Default | Meaning |
|-----------|---------|---------|
| `s_inducers_initial` | 50 | Hayflick limit (~50 divisions) |
| `h_inducers_initial` | 4 | Divisions until meiosis |
| `max_lifespan_years` | 120 | Absolute maximum lifespan |
| `senescence_death_frailty` | 0.95 | Frailty threshold for death |

### Damage Presets

```rust
// Normal aging (~76 years)
DamageParams::default()

// Progeria HGPS (~20 years) — all rates ×5
DamageParams::progeria()

// Longevity (~95 years) — all rates ×0.6
DamageParams::longevity()
```

### Calibration Results

| Mode | Death (years) | Real-world analogue |
|------|--------------|---------------------|
| `normal` | ~76 | Average human lifespan |
| `longevity` | ~95 | Long-lived individuals |
| `progeria` | ~20 | HGPS (actual: 13–14 years) |

---

## 8. Running the Simulation

### Basic run

```bash
cargo run --bin human_lifecycle
```

### Aging modes

```bash
# Normal aging (~76 years)
cargo run --bin human_lifecycle

# Progeria (~20 years)
cargo run --bin human_lifecycle -- --mode progeria

# Longevity (~95 years)
cargo run --bin human_lifecycle -- --mode longevity
```

### Using the module in code

```rust
use human_development_module::{
    HumanDevelopmentModule, HumanDevelopmentParams,
    damage::DamageParams,
    development::DevelopmentParams,
};
use cell_dt_core::SimulationModule;

// Create with default parameters
let mut module = HumanDevelopmentModule::new();

// Or with custom parameters
let params = HumanDevelopmentParams {
    damage: DamageParams::longevity(),
    development: DevelopmentParams {
        s_inducers_initial: 50,
        max_lifespan_years: 120.0,
        ..Default::default()
    },
    steps_per_year: 10,
    seed: 42,
};
let mut module = HumanDevelopmentModule::with_params(params);

// Simulation step (dt = 1 step = 0.1 year at steps_per_year = 10)
let mut world = cell_dt_core::hecs::World::new();
module.initialize(&mut world).unwrap();
module.step(&mut world, 1.0).unwrap();

// Retrieve a metric snapshot
let snap = module.snapshot();
println!("Age:       {:.1} years", snap.age_years);
println!("Frailty:   {:.3}", snap.frailty);
println!("Cognition: {:.3}", snap.cognitive);
println!("Immunity:  {:.3}", snap.immune);
```

### Sample output

```
╔══════════════════════════════════════════════════════════════════╗
║    CDATA — Human Lifecycle Simulation                            ║
║    Centriolar Damage Accumulation Theory of Aging                ║
║    Mode: NORMAL                                                  ║
╚══════════════════════════════════════════════════════════════════╝

Age     Stage             Frailty  Cognit.  Immune  Muscle  Inflamm  Alive?
────────────────────────────────────────────────────────────────────────────
   0.1  Organogenesis     0.000    1.000    1.000   1.000   0.000    +
  18.1  Adult             0.212    0.936    0.758   0.889   0.002    +
  40.0  MiddleAge         0.689    0.811    0.197   0.596   0.027    +
  65.0  Senescent         0.898    0.532    0.166   0.084   0.150    +
  76.2  Death             0.950    0.394    0.138   0.000   0.269    ✗

  ✦ Organism died at age 76.2 years
  ✦ Frailty index: 0.950
```

---

## 9. Results and Interpretation

### Order of tissue pool exhaustion

In normal mode, tissues degrade in the following order:

1. **Skin** (`Skin`) — exhausted first (~50–55 years)
2. **Intestine** (`IntestinalCrypt`) — ~55–60 years
3. **HSC** (`Hematopoietic`) — ~60–65 years → immunodeficiency
4. **Muscle** (`Muscle`) — ~70–75 years → sarcopenia
5. **Neural SC** (`Neural`) — preserved the longest

This matches clinical observations: neurodegeneration is the latest stage of
aging.

### Key CDATA predictions modelled

| Phenomenon | Mechanism in model | Supported by data |
|------------|-------------------|-------------------|
| Immunosenescence | HSC: loss of CEP164 → no cilium → no Wnt → myeloid bias | Yes |
| Sarcopenia | Satellite cells: defective spindle → symmetric exhaustion | Yes |
| Cognitive decline | NSC: Shh dependence → loss of neurogenesis | Yes |
| Inflammaging | Senescent HSC + intestine → SASP | Yes |
| Progeria | All rates ×5 → death ~20 years | HGPS: 13–14 years |
| Stem cell transplantation | Reset of `CentriolarDamageState` → lifespan +>30% | Mouse experimental data |

### Why neural stem cells last the longest

This is not accidental — neural stem cells have:
- The lowest damage multiplier (×0.8)
- The highest ciliary sensitivity (×1.3) — they "hear" Shh longest
- The slowest division rate (0.8/year) → their pool depletes more slowly

---

## 10. References

1. **Tkemaladze J., Chichinadze K.** Centriolar mechanisms of differentiation
   and replicative aging of higher animal cells. *Biochemistry (Moscow)*, 70(11),
   2005.

2. **Tkemaladze J.** Old Centrioles Make Old Bodies. *Annals of Rejuvenation
   Science*, 1(1), 2026.

3. **Tkemaladze J.** Reduction, proliferation, and differentiation defects of
   stem cells over time: a consequence of selective accumulation of old centrioles
   in the stem cells? *Molecular Biology Reports*, 50(3):2751–2761, 2023.

4. **Tkemaladze J.** The Centriolar Theory of Differentiation Explains the
   Biological Meaning of the Centriolar Theory of Organismal Aging.
   *Longevity Horizon*, 1(3), 2025.

---

*Cell DT Platform — High-Performance Cell Differentiation Simulator*
*https://github.com/djabbat/cell_dt*
