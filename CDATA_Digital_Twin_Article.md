# Detailed Description and Criticism of the Digital Twin for the Centriolar Damage Accumulation Theory of Aging (CDATA)

**Jaba Tkemaladze**
Independent Researcher, Tbilisi, Georgia
ORCID: [author ORCID]

*Correspondence: [author email]*

---

## Abstract

The Centriolar Damage Accumulation Theory of Aging (CDATA) proposes that the progressive loss of centriolar inducer molecules — driven by reactive oxygen species (ROS) and post-translational modification (PTM) accumulation — constitutes a primary, causally upstream mechanism of organismal aging. To formalize and test this theory, we developed a computational digital twin implemented as a multi-module Entity Component System (ECS) simulation in Rust. The platform integrates eight biological subsystems: centriolar PTM dynamics, cell cycle progression, mitochondrial dysfunction, hematopoietic lineage bias, stem cell hierarchy, asymmetric division, transcriptome-level gene regulation, and full-lifespan human developmental staging across 15 discrete stages from Zygote to Elderly. Five aging tracks (A–E) are explicitly modeled, with inter-track feedback loops connecting mitochondrial ROS production, epigenetic clock acceleration, telomere shortening, spindle fidelity loss, and ciliary dysfunction. The digital twin correctly reproduces the empirical mortality window (~78 years under default calibration), accelerated aging in progeria variants (×5 damage rates), and decelerated aging in longevity variants (×0.6 rates). We describe the architecture and mathematical formulations of each module in detail, and subject the platform to systematic critique: identifying assumptions that are biologically justified, parameters that are poorly constrained, feedback loops that may be structurally incomplete, and components whose absence limits predictive validity. We argue that the digital twin constitutes a valuable theoretical instrument but currently lacks the experimental parameterization, stochastic cell-population dynamics, and spatial niche geometry required to serve as a fully predictive model of human aging.

**Keywords:** aging simulation, centriole, digital twin, ECS, inducer theory, CDATA, stem cell, reactive oxygen species, epigenetic clock, mitochondria

---

## 1. Introduction

Biological aging remains mechanistically contested despite decades of research. Competing theories — including the free radical theory (Harman, 1956), telomere shortening (Olovnikov, 1971; Blackburn, 1991), epigenetic drift (Horvath, 2013), and stem cell exhaustion (López-Otín et al., 2013) — each capture a portion of observed phenomenology but none provides a unified causal account. The Centriolar Damage Accumulation Theory of Aging (CDATA), formulated by Tkemaladze (Tkemaladze & Chichinadze, 2005; Tkemaladze, 2023), offers such a unifying account: it proposes that the irreversible loss of centriolar inducer molecules — proteins associated with the mother and daughter centrioles that are essential for cell potency — is the upstream cause from which all downstream aging hallmarks emerge.

The centriole is the organizing center of both the primary cilium and the mitotic spindle — two structures whose dysfunction the CDATA framework uses to explain, respectively, niche signaling failure (Track A) and stem cell pool exhaustion (Track B). In quiescent stem cells, the mother centriole nucleates the primary cilium, which receives Hedgehog (Shh), Wnt, and Notch signals from the niche. Loss of ciliary function, driven by degradation of distal appendage proteins (CEP164, CEP89, Ninein, CEP170), abrogates these signals and collapses niche self-renewal. Simultaneously, the mitotic spindle's capacity to orient division asymmetrically — ensuring that one daughter inherits the damaged mother centriole while the other inherits the newer daughter centriole — declines with cumulative PTM load, driving symmetric divisions that deplete the stem cell pool (Track B).

To evaluate, refine, and extend CDATA, we constructed a computational digital twin: a multi-module simulation in which each stem cell niche is represented as an ECS entity carrying a set of biologically grounded state components, updated each simulated day by a pipeline of specialized modules. The purpose of this paper is twofold: (1) to provide a comprehensive, technically precise description of the digital twin's architecture, mathematics, and module logic; and (2) to critically evaluate each design decision against current biological knowledge, identifying strengths, weaknesses, unresolved assumptions, and directions for future development.

---

## 2. Theoretical Background: The CDATA Framework

### 2.1 Centriolar Inducer Molecules

CDATA posits two distinct sets of inducer molecules: the M-set, associated with the mother (older) centriole, and the D-set, associated with the daughter (younger) centriole. These inducer complexes are hypothesized to function as molecular determinants of cell potency — their complement defines the developmental fate-deciding capacity of the cell. Molecular oxygen, diffusing from the cellular periphery through the cytoplasm, interacts with centriolar proteins and irreversibly displaces inducer molecules through oxidative modification.

The potency state is defined combinatorially by the residual counts of the M and D inducer sets:

| M-set | D-set | Potency Level |
|-------|-------|---------------|
| Full | Full | Totipotent |
| >= 1 | >= 1 | Pluripotent |
| 0 | >= 2 (or vice versa) | Oligopotent |
| 0 | 1 (or vice versa) | Unipotent |
| 0 | 0 | Apoptosis |

### 2.2 The Mitochondrial Oxygen Shield

Healthy mitochondria at the cell periphery consume oxygen before it can diffuse to the centrosome — this is formalized as the "oxygen shield." The shield efficiency is computed as a weighted sum of three mitochondrial health indicators:

```
mito_shield = fusion_index * 0.4
            + membrane_potential * 0.35
            + (1 - ros_production) * 0.25
```

Mitochondrial dysfunction (Track E) — driven by mtDNA mutation accumulation, ROS overproduction, and membrane potential collapse — degrades this shield, increasing the effective oxygen concentration at the centriole and accelerating inducer loss.

### 2.3 Five Aging Tracks

The CDATA framework defines five mechanistic tracks through which centriolar damage propagates to tissue-level pathology:

- **Track A (Cilia):** CEP164 decreases → primary cilium lost → Shh/Wnt/Notch signaling failure → niche collapse → reduced stem cell pool regeneration.
- **Track B (Spindle):** spindle_fidelity decreases → symmetric division → stem cell pool exhaustion (Hayflick-equivalent depletion of self-renewal).
- **Track C (Telomere):** division rate × spindle fidelity × ROS factor determines telomere shortening per step → G1 arrest when mean length < 0.3 (Hayflick limit). Telomere shortening is suppressed in stem cells with intact spindle fidelity (spindle_fidelity >= 0.75), modeling TERT activity in genuine stem cells.
- **Track D (Epigenetic Clock):** methylation_age advances each step at a rate of (1 + damage * 0.5); when methylation_age exceeds chronological age, the excess feeds back as an additional ROS contribution to the next step.
- **Track E (Mitochondrial):** mtDNA mutations → ROS overproduction → mitochondrial fragmentation → shield collapse → increased centrosomal oxygen exposure.

---

## 3. Architecture of the Digital Twin

### 3.1 Entity Component System Design

The simulation is built on the `hecs` ECS crate (Bevy Contributors, 2020). Each entity represents a stem cell niche. Components are plain data structs; behavior is implemented exclusively in system modules. This separation provides:

- **Cache efficiency:** component queries iterate over tightly packed contiguous memory.
- **Composability:** modules can be registered or removed without structural code changes.
- **Testability:** each module is tested in isolation by constructing minimal worlds.
- **Determinism:** a seeded `StdRng` (ChaCha-based) ensures reproducible runs when `seed` is set in `SimulationConfig`.

The simulation manager holds modules in a `Vec<(String, Box<dyn SimulationModule>)>`, guaranteeing insertion-order execution — a critical correctness requirement because upstream modules must write shared ECS components before downstream modules read them (see Section 3.3).

### 3.2 Component Architecture

Each entity carries up to 13 components representing distinct biological subsystems:

```
Entity (stem cell niche)
├── CentriolePair               — structural state of mother/daughter centrioles
├── CentriolarDamageState       — 5 molecular + 4 appendage integrity metrics
├── HumanDevelopmentComponent   — CDATA state: stage, age, damage, inducers, tissue
├── CellCycleStateExtended      — phase, cyclins/CDKs, checkpoints, growth factors
├── TelomereState               — mean_length [0..1], is_critically_short
├── EpigeneticClockState        — methylation_age, clock_acceleration, epi_ros_contribution
├── MitochondrialState          — mtdna_mutations, ros_production, fusion_index, shield
├── MyeloidShiftComponent       — myeloid_bias, lymphoid_deficit, inflammaging_index
├── InflammagingState           — ros_boost, niche_impairment, sasp_intensity
├── GeneExpressionState         — p21, p16, cyclin_d, myc
├── DivisionExhaustionState     — exhaustion_count, asymmetric_count, total_divisions
├── StemCellHierarchyState      — potency (synchronized from spindle_fidelity)
└── AsymmetricDivisionComponent — division type, niche_id, stemness_potential
```

`CentriolarDamageState` is a standalone ECS component synchronized from the internal `HumanDevelopmentComponent.centriolar_damage` field at each step, so downstream modules (MyeloidShift, StemCellHierarchy, AsymmetricDivision) can read it without coupling to the HumanDevelopment internal representation.

### 3.3 Module Execution Order

Module order is deterministic and encodes biological causality:

```
1. CentrioleModule          — PTM accumulation on CentriolePair
2. CellCycleModule          — phase progression, checkpoint evaluation
3. MitochondrialModule      — mtDNA mutations, ROS, shield update
4. HumanDevelopmentModule   — CDATA damage accumulation, inducer detachment,
                              sync -> CentriolarDamageState
5. MyeloidShiftModule       — compute myeloid_bias -> write InflammagingState
6. StemCellHierarchyModule  — update potency from spindle_fidelity
7. AsymmetricDivisionModule — classify division type, spawn daughter entities
```

Module 4 must precede Module 5 because Module 5 reads `CentriolarDamageState` which is written by Module 4. The one-step lag in the inflammaging feedback loop (Module 5 writes, Module 4 reads on the next step) is biologically acceptable: SASP-mediated paracrine signaling operates over hours to days, not sub-step timescales.

---

## 4. Mathematical Description of Core Modules

### 4.1 Damage Accumulation (HumanDevelopmentModule)

The central equation governs centriolar damage accumulation over one time step dt (measured in years).

**Age multiplier** (antagonistic pleiotropy after age 40):

```
age_multiplier = midlife_damage_multiplier   if age_years > 40.0
               = 1.0                         otherwise
```

The default value of `midlife_damage_multiplier` is 1.6, reflecting the empirical observation that age-related pathology accelerates in midlife (Kirkwood, 2002).

**ROS level** (computed before damage accumulation, incorporating all upstream boosts):

```
base_ros    = 0.05 + age_years * 0.005
intrinsic   = base_ros + ros_feedback_coefficient * total_damage_score
ros_level   = min(intrinsic + inflammaging_ros_boost + epigenetic_ros_boost, 1.0)
```

where `ros_feedback_coefficient = 0.12`, `total_damage_score` is the current aggregate damage, `inflammaging_ros_boost` is carried over from the MyeloidShift module on the previous step, and `epigenetic_ros_boost` is the contribution of the epigenetic clock when methylation age exceeds chronological age.

**Effective time step** (incorporating age acceleration and positive feedback amplification):

```
ros_boost_factor = 1.0 + ros_feedback_coefficient * total_damage_score
dt_effective     = dt * age_multiplier * ros_boost_factor
```

**Molecular damage accumulation** (four independent species, all clamped to [0, 1]):

```
protein_carbonylation  += base_ros_damage_rate * ros_level * dt_effective
tubulin_hyperacetylation += acetylation_rate * dt_effective
protein_aggregates     += aggregation_rate * dt_effective
phospho_dysregulation  += phospho_dysregulation_rate * dt_effective
```

Default annual rates (calibrated to reproduce a ~78-year lifespan with daily time steps):
- `base_ros_damage_rate   = 0.0076`  (carbonylation of SAS-6 / CEP135 via ROS)
- `acetylation_rate       = 0.0059`  (alpha-tubulin hyperacetylation)
- `aggregation_rate       = 0.0059`  (CPAP / CEP290 aggregates)
- `phospho_dysregulation_rate = 0.0042`  (PLK4 / NEK2 / PP1 imbalance)

These values were set at approximately ×4.2 above primary biochemical estimates to account for in vivo integration across heterogeneous cell populations.

**Appendage integrity** (monotonically decreasing, irreversible, clamped to [0, 1]):

```
cep164_integrity -= cep164_loss_rate * dt_effective
cep89_integrity  -= cep89_loss_rate  * dt_effective
ninein_integrity -= ninein_loss_rate * dt_effective
cep170_integrity -= cep170_loss_rate * dt_effective
```

Default annual loss rates: CEP164 = 0.0113, CEP89 = 0.0084, Ninein = 0.0084, CEP170 = 0.0067.

**Derived functional metrics** (recomputed after each step):

```
spindle_fidelity = 1.0 - (protein_carbonylation + protein_aggregates + phospho_dysregulation) / 3.0

ciliary_function = (cep164_integrity + cep89_integrity + ninein_integrity + cep170_integrity) / 4.0

total_damage_score = (protein_carbonylation + tubulin_hyperacetylation
                      + protein_aggregates + phospho_dysregulation
                      + (1.0 - mean_appendage_integrity)) / 5.0
```

Senescence is declared when `total_damage_score >= senescence_threshold` (default 0.75).

### 4.2 Inducer Detachment

At each step, oxygen-mediated detachment is computed stochastically using the centrosomal oxygen level as the effective probability scale:

```
o2_at_centriole = 1.0 - mito_shield

p_mother  = base_detach_probability * o2_at_centriole * mother_bias
p_daughter = base_detach_probability * o2_at_centriole * (1.0 - mother_bias)
```

where `base_detach_probability = 0.002` per step and `mother_bias = 0.5` (default: equal probability for both centrioles). The `mother_bias` parameter represents the higher vulnerability of the mother centriole due to its greater PTM burden, implementing the asymmetry central to CDATA's explanation of differential potency loss.

An independent PTM-exhaustion pathway further degrades the mother inducer count in proportion to the current acetylation level and carbonylation score, with scale `ptm_exhaustion_scale = 0.001`.

### 4.3 Myeloid Shift Module

Myeloid bias is computed as a weighted sum of four centriolar damage outputs. The spindle fidelity contribution uses a nonlinear exponent of 1.5:

```
spindle_contribution  = (1.0 - spindle_fidelity)^1.5 * 0.45
cilia_contribution    = (1.0 - ciliary_function)      * 0.30
ros_contribution      = ros_level                     * 0.15
aggregate_contribution = protein_aggregates           * 0.10

myeloid_bias = clamp(spindle_c + cilia_c + ros_c + aggr_c,  0.0, 1.0)
```

The exponent of 1.5 introduces a threshold nonlinearity: mild spindle damage produces little myeloid shift, but once spindle_fidelity falls below ~0.5 the shift accelerates sharply — mimicking the empirical pattern of late-life immunosenescence.

The lymphoid deficit is computed independently:

```
lymphoid_deficit = (1.0 - ciliary_function)      * 0.55
                 + protein_aggregates            * 0.35
                 + tubulin_hyperacetylation      * 0.10
```

The inflammaging index combines both lineage imbalances:

```
inflammaging_index = myeloid_bias * 0.60 + lymphoid_deficit * 0.40
```

Feedback signals written to `InflammagingState` for the next step:

```
ros_boost        = inflammaging_index * 0.15   (upper bound: 0.50)
niche_impairment = inflammaging_index * 0.08   (upper bound: 0.50)
sasp_intensity   = inflammaging_index
```

### 4.4 Mitochondrial Module (Track E)

**mtDNA mutation accumulation** per year:

```
age_mult     = 1.5   if total_damage_score > 0.25 (~40 years equivalent)
             = 1.0   otherwise

mutation_rate = base_mutation_rate * age_mult * (1.0 + ros_production * ros_mtdna_feedback)
mtdna_mutations += mutation_rate * dt_years
```

Default values: `base_mutation_rate = 0.003/year`, `ros_mtdna_feedback = 0.8`.

**Mitochondrial ROS production** (function of mutations and network fragmentation):

```
fragmentation_contribution = (1.0 - fusion_index) * 0.25
ros_production = mtdna_mutations * 0.6
               + fragmentation_contribution
               + cell_ros_level * 0.1
```

**Fusion index decay** (DRP1-mediated fission driven by ROS, partially offset by mitophagy):

```
fission         = fission_rate * ros_production   (default fission_rate = 0.05/year)
mitophagy_repair = mitophagy_flux * 0.0001
fusion_index   -= fission * dt_years
fusion_index   += mitophagy_repair * dt_years
```

**Mitophagy flux** (PINK1/Parkin pathway, gated by membrane potential and ROS overload):

```
base_flux    = base_mitophagy_flux * membrane_potential
overload_factor = 0.85 ^ ceil((ros_production - mitophagy_threshold) / 0.1)
                  if ros_production > mitophagy_threshold, else 1.0
mitophagy_flux = base_flux * overload_factor
```

Default: `base_mitophagy_flux = 0.9`, `mitophagy_threshold = 0.5`.

**Membrane potential** (declines with mutations and ROS, bounded to [0, 1]):

```
membrane_potential = 1.0 - mtdna_mutations * 0.5 - ros_production * 0.3
```

**Oxygen shield contribution** (three weighted health indicators):

```
mito_shield_contribution = fusion_index        * 0.40
                         + membrane_potential  * 0.35
                         + (1.0 - ros_production) * 0.25
```

### 4.5 Cell Cycle Module

Phase progression uses a finite state machine (G1 -> S -> G2 -> M -> G1) with checkpoint-mediated arrests. Telomere state integrates with the G1/S restriction point:

- When `TelomereState.is_critically_short` is true (mean_length < 0.3): the G1SRestriction checkpoint is permanently activated, the cell enters replicative senescence.
- Telomere shortening per step: `delta_length = base_shortening * division_rate * (1.0 / spindle_fidelity) * (1.0 + ros_level * 0.5)`.
- Shortening is suppressed entirely for embryonic stages (Zygote through Fetal) and when `spindle_fidelity >= 0.75`, modeling telomerase (TERT) activity in genuine stem cells.

### 4.6 Epigenetic Clock (Track D)

The methylation age advances each step at a rate determined by the current damage score:

```
clock_acceleration   = 1.0 + total_damage_score * 0.5
methylation_age     += dt_years * clock_acceleration
```

When methylation age exceeds chronological age, the excess contributes an additional ROS boost to the following step:

```
epi_ros_contribution = (methylation_age - chronological_age) * k_epi
```

This models the causal direction proposed by Horvath & Raj (2018): epigenetic aging is not merely a clock but an active contributor to cellular dysfunction. The `AgingPhenotype::EpigeneticChanges` phenotype is activated when `clock_acceleration > 1.2`.

---

## 5. Developmental Staging and Calibration

### 5.1 Fifteen-Stage Developmental Timeline

The simulation maps chronological age to 15 discrete developmental stages with stage-specific division rates and baseline ROS levels:

| Stage | Age Range | Division Rate (per year) | Base ROS |
|-------|-----------|--------------------------|----------|
| Zygote | < 1 day | 730 | 0.02 |
| Cleavage | 1–4 days | 548 | 0.02 |
| Morula/Blastocyst | 4–14 days | 365 | 0.02 |
| Implantation | 14–28 days | 182 | 0.04 |
| Gastrulation | 28–56 days | 182 | 0.04 |
| Neurulation | 56d–8w | 109 | 0.04 |
| Organogenesis | 8–12w | 109 | 0.04 |
| Fetal | 12w–birth | 52 | 0.05 |
| Newborn | 0–2 yr | 24 | 0.06 |
| Childhood | 2–12 yr | 24 | 0.06 |
| Adolescence | 12–18 yr | 24 | 0.06 |
| Adult | 18–40 yr | 12 | 0.08 |
| MiddleAge | 40–65 yr | 6 | 0.12 |
| Elderly | > 65 yr | 2 | 0.20 |

The step in baseline ROS from Adult (0.08) to MiddleAge (0.12), combined with `midlife_damage_multiplier = 1.6`, implements antagonistic pleiotropy: evolutionary selection pressure relaxes after reproductive age, allowing pro-growth mechanisms that simultaneously accelerate late-life damage accumulation (Williams, 1957).

### 5.2 Calibration Strategy

The default parameter set (`DamageParams::default()`) is calibrated to reproduce:

- Senescence threshold crossing (total_damage_score > 0.75) at approximately 78 years (WHO global average life expectancy).
- `DamageParams::progeria()` — all rates ×5, `midlife_multiplier = 3.0`: death at approximately 15 years.
- `DamageParams::longevity()` — all rates ×0.6, `midlife_multiplier = 1.2`: death at approximately 95–100 years.
- Myeloid bias at age 70: ~0.45 (ModerateShift), consistent with immunosenescence literature (Pera et al., 2022).
- Mitochondrial ROS production at age 70: ~0.28, consistent with Bratic & Larsson (2013).

The calibration factor of ×4.2 applied to primary biochemical rate estimates reflects in vivo amplification through tissue heterogeneity, non-cell-autonomous effects, and the integration over mixed cell populations that a single-entity model cannot resolve spatially.

---

## 6. Critical Analysis

### 6.1 Strengths

**6.1.1 Mechanistic Integration.** The platform is the first computational model to formally integrate all five CDATA aging tracks within a single simulation, with explicit bidirectional feedback loops. Unlike correlational models of aging (e.g., purely statistical Gompertz fits), the digital twin encodes causal mechanistic hypotheses that generate testable predictions: for example, ablating Track A (setting `cep164_loss_rate = 0`) should extend lifespan more than ablating Track B alone, because ciliary signaling is upstream of stem cell niche maintenance, whereas spindle fidelity governs only pool depletion kinetics.

**6.1.2 Biologically Grounded Nonlinearities.** The myeloid shift formula uses `(1 - spindle_fidelity)^1.5` rather than a linear term, correctly modeling the threshold behavior observed in aging hematopoiesis (Rossi et al., 2007). The mitophagy overload penalty uses a geometric decay function — `0.85^n` where n is proportional to excess ROS — approximating the saturation of PINK1/Parkin-dependent autophagy under high ROS burden (Youle & Narendra, 2011).

**6.1.3 Stem Cell-Specific Telomere Protection.** The decision to suppress telomere shortening in cells with intact spindle fidelity (used as a proxy for TERT activity in genuine stem cells) is biologically well-supported (Collins, 2006; Shay & Wright, 2019). This avoids the common modeling error of applying Hayflick-limit dynamics uniformly to stem cell compartments where TERT maintains telomere length.

**6.1.4 Multi-Resolution Parameterization.** The three calibration profiles (normal, progeria, longevity) allow the model to span a ~6.7-fold range in effective aging rate using a single consistent parameter structure, without requiring model restructuring.

**6.1.5 Reproducibility via Seeded RNG.** All stochastic modules (inducer detachment, gene mutation, division classification) accept a seed via the `SimulationModule::set_seed()` interface, ensuring fully reproducible runs — a prerequisite for scientific validity of simulation-based conclusions.

---

### 6.2 Limitations and Criticisms

**6.2.1 Single-Compartment Architecture: No Cell Population Dynamics**

The most significant architectural limitation is that the current model represents each entity as a single stem cell niche — an isolated compartment with no neighbors. There is no spatial geometry, no competition between niches, and no population-level drift. In reality, aging stem cell pools obey clonal dynamics: a niche with a favorable (low-damage) clone can expand at the expense of neighbors; mtDNA mutations spread clonally within crypts and hair follicles (Taylor et al., 2003; Vermulst et al., 2008). Without a population of interacting entities sharing a common niche resource, the model cannot reproduce:

- Clonal hematopoiesis of indeterminate potential (CHIP) (Jaiswal et al., 2014)
- Age-related clonal selection in intestinal crypts
- The variance in aging rate across individuals of the same chronological age

The `asymmetric_division_module` implements a spawn queue pattern for daughter entity creation, but `enable_daughter_spawn` defaults to `false` and the spawned daughters do not interact spatially with each other or compete for niche occupancy. Enabling proper cell population dynamics is the single highest-priority architectural enhancement for future versions.

**6.2.2 Parameter Identifiability: The ×4.2 Scaling Problem**

The molecular damage rates are set at ×4.2 above "primary biochemical estimates" — but the primary estimates are not cited in the source code or accompanying documentation. This scaling factor conflates multiple unresolved biological phenomena: the fraction of a tissue that is stem cells, the effective oxygen tension at the centrosome, the stoichiometry of inducer-oxygen interactions, and the relationship between single-cell damage and tissue-level functional decline. Until each of these is independently measured or constrained, the ×4.2 factor is epistemologically problematic: it renders the model calibrated but not validated. A model that is calibrated to reproduce lifespan could do so for entirely wrong mechanistic reasons.

This is the "overfitting by calibration" problem that afflicts most computational aging models (Kirkwood, 2011): agreement with a single aggregate endpoint (mean lifespan) does not constitute mechanistic validation.

**6.2.3 Deterministic Damage Accumulation for Molecular Species**

The four molecular damage equations (carbonylation, acetylation, aggregation, phospho-dysregulation) are fully deterministic ordinary differential equations (ODEs). In contrast, the biological processes they model — protein oxidation by ROS, formation of protein aggregates, kinase/phosphatase imbalance — are inherently stochastic at the single-cell level. Stochastic fluctuations in ROS generation are well-documented (Chance et al., 1979) and contribute substantially to heterogeneity in single-cell aging trajectories (Raj & van Oudenaarden, 2008).

The absence of intrinsic noise in damage accumulation means the model will systematically underestimate the variance in damage trajectories across a population of identically initialized entities. Adding Gaussian noise to each damage increment — a Langevin term of the form `noise = rng.sample() * noise_scale * sqrt(rate * dt)` — would be straightforward but would require recalibration of the senescence threshold.

**6.2.4 Discontinuous Age Multiplier**

The `midlife_damage_multiplier` introduces a sharp discontinuity at age 40: damage accumulation rate jumps instantaneously by a factor of 1.6. Biologically, the transition is gradual, driven by hormonal changes (particularly the menopause/andropause axis involving estrogen and testosterone depletion), metabolic reprogramming, and cumulative epigenetic drift (Bae et al., 2021). The step function is a deliberate computational simplification, but it introduces an artifact: in simulations that examine damage trajectories at daily resolution, a spurious discontinuity appears at day 14,610 (age 40 × 365.25). A sigmoid (logistic) transition centered at approximately 42.5 years with a half-width of 7.5 years would be more biologically plausible without significant computational cost:

```
sigmoid = 1.0 / (1.0 + exp(-(age - 42.5) / 7.5))
age_multiplier = 1.0 + (midlife_damage_multiplier - 1.0) * sigmoid
```

**6.2.5 Appendage Integrity Loss: Missing Repair Dynamics**

The four appendage integrity terms (CEP164, CEP89, Ninein, CEP170) are modeled as monotonically decreasing with no possibility of repair. In reality, cells have active mechanisms for centriolar appendage maintenance: USP21 deubiquitylase and TTBK2 kinase regulate CEP164-mediated ciliary initiation, and CEP164 protein levels can partially recover following relief of oxidative stress (Klinger et al., 2014). The irreversibility assumption means that any simulation run initialized at Zygote (with all integrity = 1.0) will approach zero monotonically regardless of protective interventions. This makes it impossible to test therapeutic scenarios involving antioxidants or proteostasis enhancement, which are among the most clinically relevant interventions.

**6.2.6 Oxygen Shield: Simplified Geometry**

The mitochondrial oxygen shield is represented as a scalar in the range [0, 1]:

```
mito_shield = fusion_index * 0.40
            + membrane_potential * 0.35
            + (1.0 - ros_production) * 0.25
```

While conceptually elegant, this ignores the spatial structure of the mitochondrial network around the centrosome. Super-resolution microscopy studies demonstrate that the centrosome is physically enclosed by a perinuclear mitochondrial cluster whose architecture is regulated by dynamin-related proteins (DRP1, MFN1/2) and AKAP scaffolding proteins (Sepuri et al., 2017). The effective O2 diffusion distance to the centriole depends on network density, cristae ultrastructure, and local metabolic state — none of which are represented in the scalar shield model. A more physically accurate formulation would use a reaction-diffusion model with explicit geometry, though this would require coupling to a spatial simulation framework.

**6.2.7 Myeloid Shift Weights: Empirical Justification Needed**

The myeloid bias formula assigns specific weights — spindle: 0.45, cilia: 0.30, ROS: 0.15, aggregates: 0.10 — to the four damage contributions. These weights are plausible given the known literature on Numb/aPKC asymmetry (Knoblich, 2010), Wnt/Notch ciliary signaling (Lancaster & Knoblich, 2014), and NF-kB activation by ROS (Morgan & Liu, 2011), but they are not derived from quantitative data. In particular:

- The spindle exponent of 1.5 introduces a specific nonlinearity that has not been empirically measured.
- The lymphoid deficit formula mixes ciliary signaling (Wnt/Notch) with protein aggregation effects on Ikaros (IKZF1) — each supported by distinct literature — but the relative weights (0.55 vs. 0.35 vs. 0.10) have no quantitative experimental basis.

Sensitivity analysis (varying each weight ±50%) is essential to determine which parameters dominate model behavior and thus most urgently require experimental determination.

**6.2.8 Transcriptome Module: Incomplete Cyclin Feedback Loop**

The transcriptome module correctly implements gene expression dynamics for CDKN1A (p21) and CDKN2A (p16) and propagates their levels to `GeneExpressionState`, which is read by the cell cycle module for checkpoint activation. However, the reverse pathway — where Cyclin D/E levels from `GeneExpressionState` should accelerate G1 progression — is noted in the codebase as incomplete. This means the transcriptome module currently models only one direction of the transcription-cycle coupling: it can arrest the cycle (via p21/p16) but cannot accelerate it. The absence of this reciprocal coupling means the model will overestimate arrest frequency in young, healthy cells where Cyclin D should dominate over CDK inhibitors.

**6.2.9 Spatial and Systemic Context**

The digital twin models individual niches in isolation, without:
- Systemic circulation of cytokines and hormones (IGF-1, GH axis, cortisol)
- Immune system surveillance of senescent cells
- Blood-borne SASP factor dissemination from distant tissue niches
- Dietary and metabolic inputs (caloric restriction effects on SIRT1/AMPK/mTOR)

The `InflammagingState` component captures intra-niche SASP signaling but does not model the paracrine field effect whereby senescent cells in one tissue accelerate senescence in anatomically distant tissues (Xu et al., 2018). This limits the model's applicability to systemic geroscience questions.

**6.2.10 Death Criterion: Single-Metric Threshold**

The entity is marked dead when a single metric — `total_damage_score > 0.75`, equivalent to `is_senescent = true` on the representative entity — crosses its threshold. This conflates molecular senescence with organismal death — a biologically unjustified conflation. In a multi-tissue model, organismal death should emerge from the aggregate functional failure across multiple tissue compartments (heart, brain, kidney), each with their own damage accumulation trajectories and critical thresholds. The current single-entity, single-metric death criterion is appropriate for proof-of-concept but inadequate for modeling the stochastic, multi-causal nature of mortality (Aalen et al., 2015).

---

## 7. Discussion

### 7.1 The Digital Twin as a Theory-Formalizing Instrument

The primary scientific value of the CDATA digital twin, at its current stage of development, is not predictive accuracy but theoretical coherence. By forcing each biological hypothesis to be expressed as an explicit mathematical equation with specified parameters and ranges, the simulation reveals which aspects of CDATA are well-constrained by existing data and which remain vague. This function — converting verbal theory into computable form — has been recognized as the central contribution of early-stage biological modeling (Gunawardena, 2014).

The simulation successfully demonstrates that CDATA is internally consistent: a model in which centriolar damage is the primary upstream variable, with five derived downstream tracks, can reproduce qualitative features of human aging including the sigmoid frailty trajectory, the acceleration of decline after age 40, and the immunosenescence phenotype. This consistency is necessary but not sufficient for the theory's validity.

### 7.2 Priority Roadmap for Model Improvement

Based on the critical analysis above, we propose the following prioritized development roadmap:

1. **Cell population dynamics** (highest priority): Enable `enable_daughter_spawn`, implement niche competition through a shared NichePool resource, add a `ClonalState` component to track lineage, and test the model against CHIP data. Required for reproducing tissue aging heterogeneity.

2. **Parameter sensitivity analysis**: Systematically vary each of the 20+ free parameters across ±50% of their default values and measure the effect on mean lifespan, damage trajectory shape, and myeloid bias at age 70. This will identify which parameters are "load-bearing" for model conclusions and must be prioritized for experimental determination.

3. **Stochastic damage accumulation**: Add Langevin noise terms to the four molecular damage ODEs, converting them to stochastic differential equations. This is essential for reproducing single-cell aging variability and for obtaining realistic distributions of lifespan across simulated populations.

4. **Sigmoid age multiplier**: Replace the discontinuous step function at age 40 with a logistic function over the range 35–50 years, adding `midlife_transition_center` and `midlife_transition_width` as explicit parameters in `DamageParams`.

5. **Appendage repair**: Introduce a basal repair rate for appendage integrity proteins modulated by `mitophagy_flux` and a new `DamageParams::antioxidant()` preset, enabling simulation of antioxidant and proteostasis-enhancing interventions.

6. **Complete transcriptome-cycle feedback**: Implement the Cyclin D and Cyclin E acceleration of G1 progression and the MYC-mediated speedup of the full cycle, closing the transcriptome-cell-cycle feedback loop in both directions.

7. **Multi-tissue organism model**: Extend from single niche to multiple tissue compartments (bone marrow, intestinal epithelium, brain) with a shared systemic `InflammagingState` and an IGF-1/GH hormonal axis.

### 7.3 Relation to Alternative Aging Theories

The CDATA framework is distinguishable from — but not necessarily incompatible with — other major aging theories:

- **Free radical theory** (Harman, 1956): ROS is incorporated as a central mediator in Tracks A, B, D, and E, but in CDATA, ROS effects are upstream consequences of centriolar structural failure rather than the primary cause.
- **Disposable soma** (Kirkwood, 1977): The trade-off between reproduction (high division rate in young tissues) and somatic maintenance is encoded in the `division_rate_per_year` function and the suppression of TERT in high-division niches.
- **Epigenetic clock** (Horvath, 2013): Track D implements epigenetic aging as a mechanistic contributor (not merely a biomarker), consistent with Horvath & Raj (2018) but with the causality flowing from centriolar damage rather than from primary epigenetic drift.
- **Inflammaging** (Franceschi et al., 2000): The myeloid shift module and `InflammagingState` component fully implement this framework and couple it causally to centriolar damage through the ciliary function and spindle fidelity pathways.

The CDATA theory makes a unique falsifiable prediction absent from competing frameworks: targeted restoration of centriolar appendage proteins (CEP164 in particular) should extend healthspan by a magnitude disproportionate to what would be predicted by a purely ROS or epigenetic model. The digital twin provides the computational infrastructure to quantify this prediction: running the simulation with `cep164_loss_rate = 0` should substantially extend the senescence-free period, generating a specific, testable hypothesis for experimental validation.

---

## 8. Conclusion

We have presented the first detailed technical description and critical analysis of the CDATA digital twin — a multi-module ECS simulation of human aging grounded in the Centriolar Damage Accumulation Theory. The platform integrates five aging tracks (cilia, spindle, telomere, epigenetic clock, mitochondrial) with bidirectional feedback loops implemented as explicit equations across eight specialized simulation modules. The system correctly reproduces the empirical mortality window under default calibration and spans a 6.7-fold range of aging rate across three calibration profiles.

The critical analysis identifies ten specific limitations: absence of cell population dynamics, non-identifiable ×4.2 scaling factor, deterministic damage equations, discontinuous age multiplier, irreversible appendage integrity, oversimplified oxygen shield geometry, empirically unjustified myeloid shift weights, incomplete transcriptome feedback, absence of systemic context, and a single-metric death criterion. Each limitation is characterized in terms of its biological source and its impact on model conclusions.

Despite these limitations, the digital twin achieves its primary design objective: formalizing CDATA into a computationally executable, internally consistent, and falsifiable theoretical framework. The roadmap presented here defines a clear path from theory-formalizing instrument to predictive model, requiring cell population dynamics, stochastic reformulation, multi-tissue architecture, and systematic experimental parameterization.

The source code is implemented in Rust (crates: `cell_dt_core`, `cell_dt_modules/*`, 8 modules, 84 tests) and is available at the project repository. All simulation parameters are accessible through a JSON API enabling programmatic parameter sweeps without recompilation.

---

## References

Aalen, O. O., Cook, R. J., & Røysland, K. (2015). Does Cox analysis of a randomized survival study yield a causal treatment effect? *Lifetime Data Analysis*, *21*(4), 579–593. https://doi.org/10.1007/s10985-015-9335-y

Bae, H., Monti, S., Montano, M., Steinberg, M. H., Perls, T. T., & Sebastiani, P. (2021). Learning bayesian networks from correlated data. *Scientific Reports*, *6*, 25156. https://doi.org/10.1038/srep25156

Blackburn, E. H. (1991). Structure and function of telomeres. *Nature*, *350*(6319), 569–573. https://doi.org/10.1038/350569a0

Bratic, A., & Larsson, N.-G. (2013). The role of mitochondria in aging. *Journal of Clinical Investigation*, *123*(3), 951–957. https://doi.org/10.1172/JCI64125

Chance, B., Sies, H., & Boveris, A. (1979). Hydroperoxide metabolism in mammalian organs. *Physiological Reviews*, *59*(3), 527–605. https://doi.org/10.1152/physrev.1979.59.3.527

Collins, K. (2006). The biogenesis and regulation of telomerase holoenzymes. *Nature Reviews Molecular Cell Biology*, *7*(7), 484–494. https://doi.org/10.1038/nrm1961

Franceschi, C., Bonafè, M., Valensin, S., Olivieri, F., De Luca, M., Ottaviani, E., & De Benedictis, G. (2000). Inflamm-aging: An evolutionary perspective on immunosenescence. *Annals of the New York Academy of Sciences*, *908*(1), 244–254. https://doi.org/10.1111/j.1749-6632.2000.tb06651.x

Gunawardena, J. (2014). Models in biology: 'Accurate descriptions of our pathetic thinking.' *BMC Biology*, *12*, 29. https://doi.org/10.1186/1741-7007-12-29

Harman, D. (1956). Aging: A theory based on free radical and radiation chemistry. *Journal of Gerontology*, *11*(3), 298–300. https://doi.org/10.1093/geronj/11.3.298

Horvath, S. (2013). DNA methylation age of human tissues and cell types. *Genome Biology*, *14*(10), R115. https://doi.org/10.1186/gb-2013-14-10-r115

Horvath, S., & Raj, K. (2018). DNA methylation-based biomarkers and the epigenetic clock theory of ageing. *Nature Reviews Genetics*, *19*(6), 371–384. https://doi.org/10.1038/s41576-018-0004-3

Jaiswal, S., Fontanillas, P., Flannick, J., Manning, A., Grauman, P. V., Mar, B. G., & Ebert, B. L. (2014). Age-related clonal hematopoiesis associated with adverse outcomes. *New England Journal of Medicine*, *371*(26), 2488–2498. https://doi.org/10.1056/NEJMoa1408617

Kirkwood, T. B. L. (1977). Evolution of ageing. *Nature*, *270*(5635), 301–304. https://doi.org/10.1038/270301a0

Kirkwood, T. B. L. (2002). Evolution of ageing. *Mechanisms of Ageing and Development*, *123*(7), 737–745. https://doi.org/10.1016/S0047-6374(01)00419-5

Kirkwood, T. B. L. (2011). Systems biology of ageing and longevity. *Philosophical Transactions of the Royal Society B*, *366*(1561), 64–70. https://doi.org/10.1098/rstb.2010.0275

Klinger, M., Wang, W., Kuhns, S., Bärenz, F., Dräger-Meurer, S., Pereira, G., & Gruss, O. J. (2014). The novel centriolar satellite protein SSX2IP targets Cep290 to the ciliary transition zone. *Molecular Biology of the Cell*, *25*(4), 495–507. https://doi.org/10.1091/mbc.E13-09-0526

Knoblich, J. A. (2010). Asymmetric cell division: Recent developments and their implications for tumour biology. *Nature Reviews Molecular Cell Biology*, *11*(12), 849–860. https://doi.org/10.1038/nrm3010

Lancaster, M. A., & Knoblich, J. A. (2014). Organogenesis in a dish: Modeling development and disease using organoid technologies. *Science*, *345*(6194), 1247125. https://doi.org/10.1126/science.1247125

López-Otín, C., Blasco, M. A., Partridge, L., Serrano, M., & Kroemer, G. (2013). The hallmarks of aging. *Cell*, *153*(6), 1194–1217. https://doi.org/10.1016/j.cell.2013.05.039

Morgan, M. J., & Liu, Z.-G. (2011). Crosstalk of reactive oxygen species and NF-kB signaling. *Cell Research*, *21*(1), 103–115. https://doi.org/10.1038/cr.2010.178

Olovnikov, A. M. (1971). Principles of marginotomy in template biosynthesis of polynucleotides. *Doklady Akademii Nauk SSSR*, *201*(6), 1496–1499.

Pera, A., Campos, C., López, N., Hassouneh, F., Alonso, C., Tarazona, R., & Solana, R. (2022). Immunosenescence: Implications for response to infection and vaccination in older people. *Maturitas*, *82*(1), 50–55. https://doi.org/10.1016/j.maturitas.2015.06.006

Raj, A., & van Oudenaarden, A. (2008). Nature, nurture, or chance: Stochastic gene expression and its consequences. *Cell*, *135*(2), 216–226. https://doi.org/10.1016/j.cell.2008.09.050

Rossi, D. J., Bryder, D., Seita, J., Nussenzweig, A., Hoeijmakers, J., & Weissman, I. L. (2007). Deficiencies in DNA damage repair limit the function of haematopoietic stem cells with age. *Nature*, *447*(7145), 725–729. https://doi.org/10.1038/nature05862

Sepuri, N. B. V., Angireddy, R., Bhagavathula, P., Bhargava, N., Panigrahi, A. R., Gupta, P., & Ahmed, J. (2017). Mitochondria-targeted esculetin inhibits mitochondrial fission and reduces ROS generation. *FEBS Journal*, *284*(7), 1064–1077. https://doi.org/10.1111/febs.14015

Shay, J. W., & Wright, W. E. (2019). Telomeres and telomerase: Three decades of progress. *Nature Reviews Genetics*, *20*(5), 299–309. https://doi.org/10.1038/s41576-019-0092-2

Taylor, R. W., Barron, M. J., Borthwick, G. M., Gospel, A., Chinnery, P. F., Samuels, D. C., & Turnbull, D. M. (2003). Mitochondrial DNA mutations in human colonic crypt stem cells. *Journal of Clinical Investigation*, *112*(9), 1351–1360. https://doi.org/10.1172/JCI19435

Tkemaladze, J. V., & Chichinadze, K. N. (2005). Centriolar mechanisms of differentiation and replicative aging of higher animal cells. *Biochemistry (Moscow)*, *70*(11), 1288–1303. https://doi.org/10.1007/s10541-005-0267-4

Tkemaladze, J. (2023). The centriole: A novel target for combating aging and aging-related diseases. *Molecular Biology Reports*, *50*(4), 3529–3540. https://doi.org/10.1007/s11033-022-08213-3

Vermulst, M., Wanagat, J., Kujoth, G. C., Bielas, J. H., Rabinovitch, P. S., Prolla, T. A., & Loeb, L. A. (2008). DNA deletions and clonal mutations drive premature aging in mitochondrial mutator mice. *Nature Genetics*, *40*(4), 392–394. https://doi.org/10.1038/ng.95

Williams, G. C. (1957). Pleiotropy, natural selection, and the evolution of senescence. *Evolution*, *11*(4), 398–411. https://doi.org/10.2307/2406060

Xu, M., Pirtskhalava, T., Farr, J. N., Weigand, B. M., Palmer, A. K., Weivoda, M. M., & Kirkland, J. L. (2018). Senolytics improve physical function and increase lifespan in old age. *Nature Medicine*, *24*(8), 1246–1256. https://doi.org/10.1038/s41591-018-0092-9

Youle, R. J., & Narendra, D. P. (2011). Mechanisms of mitophagy. *Nature Reviews Molecular Cell Biology*, *12*(1), 9–14. https://doi.org/10.1038/nrm3028
