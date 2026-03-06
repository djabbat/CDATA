//! Модуль транскриптома
//! 
//! Реализует:
//! - Экспрессию генов
//! - Сигнальные пути (Wnt, TGF-β, Hippo, Notch)
//! - Транскрипционные факторы
//! - Взаимодействие с центриолью
//! - Регуляцию клеточного цикла через гены

use cell_dt_core::{
    SimulationModule, SimulationResult,
    components::{
        CentriolePair, CellCycleStateExtended,
        Phase, GeneExpressionState,
    },
    hecs::World,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use log::{info, debug, warn};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use std::collections::{HashMap, HashSet, VecDeque};

/// Типы сигнальных путей
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SignalingPathway {
    Wnt,      // Регуляция пролиферации
    TGFb,     // Ростовые факторы
    Hippo,    // Контроль размера органа
    Notch,    // Клеточная дифференцировка
    Hedgehog, // Полярность и паттернинг
    JAKSTAT,  // Воспаление и стресс
    MAPK,     // Митогенный сигналинг
    PI3K,     // Выживание и метаболизм
}

/// Типы транскрипционных факторов
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TranscriptionFactor {
    // Связанные с центриолью
    YAP,      // Hippo путь, связан с центриолью
    TAZ,      // Партнер YAP
    STAT3,    // JAK/STAT путь, найден на центриолях
    
    // Клеточный цикл
    P53,      // Опухолевый супрессор
    RB,       // Ретинобластома
    E2F,      // Фактор транскрипции S-фазы
    MYC,      // Протоонкоген
    
    // Сигнальные пути
    CTNNB1,   // β-catenin (Wnt путь)
    SMAD,     // TGF-β путь
    GLI,      // Hedgehog путь
    NFKB,     // Воспаление
}

/// Категории генов
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GeneCategory {
    Cyclin,
    CDK,
    Checkpoint,
    DNArepair,
    Apoptosis,
    Stemness,
    Differentiation,
    Metabolism,
    Cytoskeleton,
    Centriole,
}

/// Гены и их функции
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gene {
    pub name: String,
    pub expression_level: f32,      // 0.0 - 1.0
    pub basal_expression: f32,       // Базальный уровень
    pub max_expression: f32,         // Максимальный уровень
    pub half_life: f32,              // Период полураспада mRNA
    pub regulated_by: Vec<TranscriptionFactor>, // Регуляторы
    pub affects_pathways: Vec<SignalingPathway>, // Влияет на пути
    pub category: GeneCategory,
}

/// Состояние сигнального пути
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathwayState {
    pub pathway: SignalingPathway,
    pub activity: f32,               // 0.0 - 1.0
    pub ligands: Vec<String>,         // Лиганды
    pub receptors: Vec<String>,       // Рецепторы
    pub inhibitors: Vec<String>,      // Ингибиторы
    pub target_genes: Vec<String>,    // Гены-мишени
}

/// Состояние транскриптома клетки
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptomeState {
    // Экспрессия генов
    pub genes: HashMap<String, Gene>,
    pub expressed_genes: HashSet<String>,
    
    // Сигнальные пути
    pub pathways: HashMap<SignalingPathway, PathwayState>,
    
    // Транскрипционные факторы
    pub transcription_factors: HashMap<TranscriptionFactor, f32>, // активность
    
    // Эпигенетика
    pub chromatin_state: HashMap<String, f32>, // доступность хроматина
    pub methylation: HashMap<String, f32>,      // метилирование ДНК
    
    // Взаимодействие с центриолью
    pub centriole_related_genes: Vec<String>,
    pub centriole_signaling: f32,                // сигналы от центриоли
    
    // Статистика
    pub total_expression: f32,
    pub active_pathways: usize,
    pub differentiation_score: f32,               // 0-1, насколько клетка дифференцирована
}

impl TranscriptomeState {
    pub fn new() -> Self {
        let mut state = Self {
            genes: HashMap::new(),
            expressed_genes: HashSet::new(),
            pathways: HashMap::new(),
            transcription_factors: HashMap::new(),
            chromatin_state: HashMap::new(),
            methylation: HashMap::new(),
            centriole_related_genes: Vec::new(),
            centriole_signaling: 0.5,
            total_expression: 0.0,
            active_pathways: 0,
            differentiation_score: 0.0,
        };
        
        // Инициализируем гены
        state.initialize_genes();
        
        // Инициализируем сигнальные пути
        state.initialize_pathways();
        
        // Инициализируем транскрипционные факторы
        state.initialize_transcription_factors();
        
        state
    }
    
    fn initialize_genes(&mut self) {
        // Гены клеточного цикла
        self.add_gene(Gene {
            name: "CCND1".to_string(), // Cyclin D1
            expression_level: 0.1,
            basal_expression: 0.1,
            max_expression: 1.0,
            half_life: 0.5,
            regulated_by: vec![TranscriptionFactor::MYC, TranscriptionFactor::CTNNB1],
            affects_pathways: vec![SignalingPathway::Wnt],
            category: GeneCategory::Cyclin,
        });
        
        self.add_gene(Gene {
            name: "CCNE1".to_string(), // Cyclin E1
            expression_level: 0.05,
            basal_expression: 0.05,
            max_expression: 1.0,
            half_life: 0.4,
            regulated_by: vec![TranscriptionFactor::E2F],
            affects_pathways: vec![],
            category: GeneCategory::Cyclin,
        });
        
        self.add_gene(Gene {
            name: "CCNA2".to_string(), // Cyclin A2
            expression_level: 0.0,
            basal_expression: 0.0,
            max_expression: 1.0,
            half_life: 0.3,
            regulated_by: vec![TranscriptionFactor::E2F],
            affects_pathways: vec![],
            category: GeneCategory::Cyclin,
        });
        
        self.add_gene(Gene {
            name: "CCNB1".to_string(), // Cyclin B1
            expression_level: 0.0,
            basal_expression: 0.0,
            max_expression: 1.0,
            half_life: 0.2,
            regulated_by: vec![],
            affects_pathways: vec![],
            category: GeneCategory::Cyclin,
        });
        
        // Центриолярные гены
        self.add_gene(Gene {
            name: "CETN1".to_string(), // Centrin 1
            expression_level: 0.5,
            basal_expression: 0.5,
            max_expression: 1.0,
            half_life: 0.8,
            regulated_by: vec![],
            affects_pathways: vec![],
            category: GeneCategory::Centriole,
        });
        
        self.add_gene(Gene {
            name: "CETN2".to_string(), // Centrin 2
            expression_level: 0.5,
            basal_expression: 0.5,
            max_expression: 1.0,
            half_life: 0.8,
            regulated_by: vec![],
            affects_pathways: vec![],
            category: GeneCategory::Centriole,
        });
        
        self.add_gene(Gene {
            name: "PCNT".to_string(), // Pericentrin
            expression_level: 0.4,
            basal_expression: 0.4,
            max_expression: 1.0,
            half_life: 0.7,
            regulated_by: vec![],
            affects_pathways: vec![],
            category: GeneCategory::Centriole,
        });
        
        // Гены апоптоза
        self.add_gene(Gene {
            name: "TP53".to_string(), // p53
            expression_level: 0.2,
            basal_expression: 0.2,
            max_expression: 2.0,
            half_life: 0.1,
            regulated_by: vec![],
            affects_pathways: vec![],
            category: GeneCategory::Apoptosis,
        });
        
        self.add_gene(Gene {
            name: "BAX".to_string(), // pro-apoptotic
            expression_level: 0.1,
            basal_expression: 0.1,
            max_expression: 1.0,
            half_life: 0.3,
            regulated_by: vec![TranscriptionFactor::P53],
            affects_pathways: vec![],
            category: GeneCategory::Apoptosis,
        });

        // Ингибиторы клеточного цикла (CDK inhibitors)
        self.add_gene(Gene {
            name: "CDKN1A".to_string(), // p21 — временный G1/S арест (DNA damage, stress)
            expression_level: 0.0,
            basal_expression: 0.0,
            max_expression: 1.0,
            half_life: 0.2,
            regulated_by: vec![TranscriptionFactor::P53],
            affects_pathways: vec![],
            category: GeneCategory::Checkpoint,
        });

        self.add_gene(Gene {
            name: "CDKN2A".to_string(), // p16/INK4a — постоянный арест (сенесценция)
            expression_level: 0.0,
            basal_expression: 0.0,
            max_expression: 1.0,
            half_life: 0.8, // очень стабильный — накапливается годами
            regulated_by: vec![TranscriptionFactor::NFKB],
            affects_pathways: vec![],
            category: GeneCategory::Checkpoint,
        });

        // Гены стволовости
        self.add_gene(Gene {
            name: "NANOG".to_string(),
            expression_level: 0.0,
            basal_expression: 0.0,
            max_expression: 1.0,
            half_life: 0.4,
            regulated_by: vec![],
            affects_pathways: vec![],
            category: GeneCategory::Stemness,
        });
        
        self.add_gene(Gene {
            name: "POU5F1".to_string(), // OCT4
            expression_level: 0.0,
            basal_expression: 0.0,
            max_expression: 1.0,
            half_life: 0.4,
            regulated_by: vec![],
            affects_pathways: vec![],
            category: GeneCategory::Stemness,
        });
        
        self.add_gene(Gene {
            name: "SOX2".to_string(),
            expression_level: 0.0,
            basal_expression: 0.0,
            max_expression: 1.0,
            half_life: 0.4,
            regulated_by: vec![],
            affects_pathways: vec![],
            category: GeneCategory::Stemness,
        });
    }
    
    fn add_gene(&mut self, gene: Gene) {
        let name = gene.name.clone();
        if gene.category == GeneCategory::Centriole {
            self.centriole_related_genes.push(name.clone());
        }
        self.genes.insert(name, gene);
    }
    
    fn initialize_pathways(&mut self) {
        let pathways = vec![
            SignalingPathway::Wnt,
            SignalingPathway::TGFb,
            SignalingPathway::Hippo,
            SignalingPathway::Notch,
            SignalingPathway::Hedgehog,
            SignalingPathway::JAKSTAT,
            SignalingPathway::MAPK,
            SignalingPathway::PI3K,
        ];
        
        for pathway in pathways {
            self.pathways.insert(pathway, PathwayState {
                pathway,
                activity: 0.5,
                ligands: Vec::new(),
                receptors: Vec::new(),
                inhibitors: Vec::new(),
                target_genes: Vec::new(),
            });
        }
    }
    
    fn initialize_transcription_factors(&mut self) {
        let factors = vec![
            TranscriptionFactor::YAP,
            TranscriptionFactor::TAZ,
            TranscriptionFactor::STAT3,
            TranscriptionFactor::P53,
            TranscriptionFactor::RB,
            TranscriptionFactor::E2F,
            TranscriptionFactor::MYC,
            TranscriptionFactor::CTNNB1,
            TranscriptionFactor::SMAD,
            TranscriptionFactor::GLI,
            TranscriptionFactor::NFKB,
        ];
        
        for factor in factors {
            self.transcription_factors.insert(factor, 0.3);
        }
    }
    
    /// Обновление экспрессии генов
    pub fn update_expression(&mut self, rng: &mut impl Rng, dt: f32, cell_cycle: &CellCycleStateExtended, centriole: Option<&CentriolePair>) {
        
        // Влияние центриоли на транскрипцию
        if let Some(cent) = centriole {
            self.centriole_signaling = cent.mtoc_activity;
            
            // Центриоль активирует YAP/TAZ
            if let Some(yap) = self.transcription_factors.get_mut(&TranscriptionFactor::YAP) {
                *yap = (*yap + cent.mtoc_activity * 0.1 * dt).min(1.0);
            }
            
            // STAT3 активируется центриолью
            if let Some(stat3) = self.transcription_factors.get_mut(&TranscriptionFactor::STAT3) {
                *stat3 = (*stat3 + cent.mother.ptm_signature.phosphorylation_level * 0.1 * dt).min(1.0);
            }
        }
        
        // Обновляем каждый ген
        for gene in self.genes.values_mut() {
            // Базальная экспрессия
            let mut target = gene.basal_expression;
            
            // Регуляция транскрипционными факторами
            for regulator in &gene.regulated_by {
                if let Some(&activity) = self.transcription_factors.get(regulator) {
                    target += activity * 0.1;
                }
            }
            
            // Фаза клеточного цикла влияет на экспрессию
            match gene.category {
                GeneCategory::Cyclin => {
                    target += match (gene.name.as_str(), cell_cycle.phase) {
                        ("CCND1", Phase::G1) => 0.5,
                        ("CCNE1", Phase::G1) => 0.3,
                        ("CCNE1", Phase::S) => 0.5,
                        ("CCNA2", Phase::S) => 0.4,
                        ("CCNA2", Phase::G2) => 0.5,
                        ("CCNB1", Phase::G2) => 0.4,
                        ("CCNB1", Phase::M) => 0.6,
                        _ => 0.0,
                    };
                }
                GeneCategory::Centriole => {
                    // Центриолярные гены всегда активны
                    target += 0.3;
                }
                GeneCategory::Apoptosis => {
                    // Апоптозные гены активируются при стрессе
                    target += cell_cycle.growth_factors.stress_level * 0.5;
                }
                GeneCategory::Checkpoint => {
                    match gene.name.as_str() {
                        "CDKN1A" => {
                            // p21 индуцируется p53 (ДНК-повреждение) и оксидативным стрессом
                            let p53_activity = self.transcription_factors
                                .get(&TranscriptionFactor::P53).copied().unwrap_or(0.0);
                            target += p53_activity * 0.6
                                    + cell_cycle.growth_factors.dna_damage * 0.4
                                    + cell_cycle.growth_factors.oxidative_stress * 0.2;
                        }
                        "CDKN2A" => {
                            // p16 медленно накапливается при хроническом воспалении и стрессе
                            let nfkb = self.transcription_factors
                                .get(&TranscriptionFactor::NFKB).copied().unwrap_or(0.0);
                            target += nfkb * 0.3
                                    + cell_cycle.growth_factors.stress_level * 0.15;
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
            
            // Ограничиваем и добавляем случайные флуктуации
            target = target.clamp(0.0, gene.max_expression);
            target += (rng.gen::<f32>() - 0.5) * 0.05;
            target = target.clamp(0.0, gene.max_expression);
            
            // Медленно приближаемся к целевой экспрессии
            gene.expression_level += (target - gene.expression_level) * 0.1 * dt;
            gene.expression_level = gene.expression_level.clamp(0.0, gene.max_expression);
            
            // Отслеживаем выраженные гены
            if gene.expression_level > 0.1 {
                self.expressed_genes.insert(gene.name.clone());
            } else {
                self.expressed_genes.remove(&gene.name);
            }
        }
        
        // Обновляем сигнальные пути
        self.update_pathways(dt, cell_cycle);
        
        // Обновляем статистику
        self.total_expression = self.genes.values().map(|g| g.expression_level).sum();
        self.active_pathways = self.pathways.values().filter(|p| p.activity > 0.3).count();
        
        // Считаем степень дифференцировки
        let stemness_genes = ["NANOG", "POU5F1", "SOX2"];
        let stemness_expression: f32 = stemness_genes.iter()
            .filter_map(|&name| self.genes.get(name))
            .map(|g| g.expression_level)
            .sum();
        self.differentiation_score = 1.0 - (stemness_expression / 3.0).min(1.0);
    }
    
    /// Обновление сигнальных путей
    fn update_pathways(&mut self, dt: f32, cell_cycle: &CellCycleStateExtended) {
        for (pathway, state) in self.pathways.iter_mut() {
            // Базовая активность
            let mut activity = 0.3;
            
            // Модуляция в зависимости от условий
            match pathway {
                SignalingPathway::Hippo => {
                    // Hippo путь связан с центриолью
                    activity += self.centriole_signaling * 0.3;
                }
                SignalingPathway::Wnt => {
                    // Wnt путь влияет на пролиферацию
                    if let Some(ctnnb1) = self.transcription_factors.get(&TranscriptionFactor::CTNNB1) {
                        activity += ctnnb1 * 0.4;
                    }
                }
                SignalingPathway::JAKSTAT => {
                    // JAK/STAT путь - воспаление и стресс
                    activity += cell_cycle.growth_factors.stress_level * 0.5;
                }
                SignalingPathway::MAPK => {
                    // MAPK путь - митогенный сигналинг
                    activity += cell_cycle.growth_factors.growth_signal * 0.4;
                }
                SignalingPathway::PI3K => {
                    // PI3K путь - выживание
                    activity += cell_cycle.growth_factors.nutrient_level * 0.3;
                }
                _ => {}
            }
            
            // Плавно обновляем активность
            state.activity += (activity - state.activity) * 0.1 * dt;
            state.activity = state.activity.clamp(0.0, 1.0);
        }
    }
    
    /// Получить профиль экспрессии для scRNA-seq симуляции
    pub fn get_expression_profile(&self) -> HashMap<String, f32> {
        self.genes.iter()
            .map(|(name, gene)| (name.clone(), gene.expression_level))
            .collect()
    }
    
    /// Получить активность сигнальных путей
    pub fn get_pathway_activity(&self) -> HashMap<String, f32> {
        self.pathways.iter()
            .map(|(pathway, state)| (format!("{:?}", pathway), state.activity))
            .collect()
    }
    
    /// Проверить, является ли клетка стволовой
    pub fn is_stem_cell(&self) -> bool {
        let stemness_genes = ["NANOG", "POU5F1", "SOX2"];
        let stemness_score: f32 = stemness_genes.iter()
            .filter_map(|&name| self.genes.get(name))
            .map(|g| g.expression_level)
            .sum();
        stemness_score > 1.5
    }
    
    /// Получить тип клетки на основе транскриптома
    pub fn get_cell_type(&self) -> String {
        if self.is_stem_cell() {
            return "Stem".to_string();
        }
        
        if self.differentiation_score > 0.8 {
            return "Differentiated".to_string();
        }
        
        if self.pathways.get(&SignalingPathway::Wnt).map(|p| p.activity).unwrap_or(0.0) > 0.7 {
            return "Proliferating".to_string();
        }
        
        if self.genes.get("TP53").map(|g| g.expression_level).unwrap_or(0.0) > 1.0 {
            return "Stressed".to_string();
        }
        
        "Progenitor".to_string()
    }
}

impl Default for TranscriptomeState {
    fn default() -> Self {
        Self::new()
    }
}

/// Параметры модуля транскриптома
#[derive(Debug, Clone)]
pub struct TranscriptomeParams {
    pub mutation_rate: f32,
    pub noise_level: f32,
    pub signaling_strength: f32,
    pub enable_epigenetics: bool,
    pub stemness_maintenance: bool,
}

impl Default for TranscriptomeParams {
    fn default() -> Self {
        Self {
            mutation_rate: 0.001,
            noise_level: 0.05,
            signaling_strength: 1.0,
            enable_epigenetics: true,
            stemness_maintenance: true,
        }
    }
}

/// Модуль транскриптома
pub struct TranscriptomeModule {
    params: TranscriptomeParams,
    step_count: u64,
    expression_history: VecDeque<HashMap<String, f32>>,
    rng: StdRng,
}

impl TranscriptomeModule {
    pub fn new() -> Self {
        Self {
            params: TranscriptomeParams::default(),
            step_count: 0,
            expression_history: VecDeque::new(),
            rng: StdRng::from_entropy(),
        }
    }

    pub fn with_params(params: TranscriptomeParams) -> Self {
        Self {
            params,
            step_count: 0,
            expression_history: VecDeque::new(),
            rng: StdRng::from_entropy(),
        }
    }

    /// Обновление транскриптома для одной клетки
    fn update_transcriptome(&mut self, transcriptome: &mut TranscriptomeState,
                           cell_cycle: &CellCycleStateExtended,
                           centriole: Option<&CentriolePair>,
                           dt: f32) {
        transcriptome.update_expression(&mut self.rng, dt, cell_cycle, centriole);
    }

    /// Мутация генов (редкое событие)
    fn apply_mutation(&mut self, transcriptome: &mut TranscriptomeState) {
        if self.rng.gen::<f32>() < self.params.mutation_rate {
            // Выбираем случайный ген для мутации (не первый из HashMap!)
            let keys: Vec<String> = transcriptome.genes.keys().cloned().collect();
            if !keys.is_empty() {
                let idx = self.rng.gen_range(0..keys.len());
                if let Some(gene) = transcriptome.genes.get_mut(&keys[idx]) {
                    gene.expression_level = (gene.expression_level * 2.0).min(gene.max_expression);
                    gene.max_expression *= 1.5;
                    warn!("Gene {} mutated!", gene.name);
                }
            }
        }
    }
}

impl SimulationModule for TranscriptomeModule {
    fn name(&self) -> &str {
        "transcriptome_module"
    }

    fn set_seed(&mut self, seed: u64) {
        self.rng = StdRng::seed_from_u64(seed);
    }

    fn step(&mut self, world: &mut World, dt: f64) -> SimulationResult<()> {
        self.step_count += 1;
        let dt_f32 = dt as f32;
        
        debug!("Transcriptome module step {} with dt={}", self.step_count, dt);
        
        // Получаем все клетки с транскриптомом, клеточным циклом и центриолями
        let mut query = world.query::<(
            &mut TranscriptomeState,
            &CellCycleStateExtended,
            Option<&CentriolePair>,
            Option<&mut GeneExpressionState>,
        )>();

        for (_, (transcriptome, cell_cycle, centriole_opt, gene_expr_opt)) in query.iter() {
            self.update_transcriptome(transcriptome, cell_cycle, centriole_opt, dt_f32);
            self.apply_mutation(transcriptome);

            // Синхронизируем ключевые уровни генов в shared ECS-компонент
            if let Some(gene_expr) = gene_expr_opt {
                gene_expr.p21_level = transcriptome.genes.get("CDKN1A")
                    .map(|g| g.expression_level).unwrap_or(0.0);
                gene_expr.p16_level = transcriptome.genes.get("CDKN2A")
                    .map(|g| g.expression_level).unwrap_or(0.0);
                gene_expr.cyclin_d_level = transcriptome.genes.get("CCND1")
                    .map(|g| g.expression_level).unwrap_or(0.5);
                gene_expr.myc_level = transcriptome.transcription_factors
                    .get(&TranscriptionFactor::MYC).copied().unwrap_or(0.3);
            }
        }

        // Сохраняем историю экспрессии для анализа
        if self.step_count.is_multiple_of(100) {
            if let Some((_, (transcriptome, _, _, _))) = query.iter().next() {
                self.expression_history.push_back(transcriptome.get_expression_profile());

                // Ограничиваем историю — VecDeque даёт O(1) удаление с начала
                if self.expression_history.len() > 100 {
                    self.expression_history.pop_front();
                }
                
                // Логируем статистику
                info!("Transcriptome stats: expressed genes={}, active pathways={}", 
                      transcriptome.expressed_genes.len(), transcriptome.active_pathways);
            }
        }
        
        Ok(())
    }
    
    fn get_params(&self) -> Value {
        json!({
            "mutation_rate": self.params.mutation_rate,
            "noise_level": self.params.noise_level,
            "signaling_strength": self.params.signaling_strength,
            "enable_epigenetics": self.params.enable_epigenetics,
            "stemness_maintenance": self.params.stemness_maintenance,
            "step_count": self.step_count,
            "history_length": self.expression_history.len(),
        })
    }
    
    fn set_params(&mut self, params: &Value) -> SimulationResult<()> {
        if let Some(rate) = params.get("mutation_rate").and_then(|v| v.as_f64()) {
            self.params.mutation_rate = rate as f32;
        }
        if let Some(noise) = params.get("noise_level").and_then(|v| v.as_f64()) {
            self.params.noise_level = noise as f32;
        }
        if let Some(strength) = params.get("signaling_strength").and_then(|v| v.as_f64()) {
            self.params.signaling_strength = strength as f32;
        }
        if let Some(epigen) = params.get("enable_epigenetics").and_then(|v| v.as_bool()) {
            self.params.enable_epigenetics = epigen;
        }
        if let Some(stem) = params.get("stemness_maintenance").and_then(|v| v.as_bool()) {
            self.params.stemness_maintenance = stem;
        }
        
        Ok(())
    }
    
    fn initialize(&mut self, world: &mut World) -> SimulationResult<()> {
        info!("Initializing transcriptome module");
        
        // Собираем все сущности с клеточным циклом
        let entities: Vec<_> = world.query::<&CellCycleStateExtended>()
            .iter()
            .map(|(e, _)| e)
            .collect();
        
        let entity_count = entities.len();
        
        // Для каждой сущности добавляем транскриптом и GeneExpressionState
        for &entity in &entities {
            if !world.contains(entity) {
                continue;
            }
            world.insert(entity, (
                TranscriptomeState::new(),
                GeneExpressionState::default(),
            ))?;
        }
        
        info!("Initialized transcriptome for {} cells", entity_count);
        
        Ok(())
    }
}

impl Default for TranscriptomeModule {
    fn default() -> Self {
        Self::new()
    }
}
