use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Основная конфигурация симуляции
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SimulationConfig {
    pub max_steps: u64,
    pub dt: f64,
    pub checkpoint_interval: u64,
    pub num_threads: Option<usize>,
    pub seed: Option<u64>,
    pub parallel_modules: bool,
    pub output_dir: PathBuf,
}

impl Default for SimulationConfig {
    fn default() -> Self {
        Self {
            max_steps: 10000,
            dt: 0.1,
            checkpoint_interval: 1000,
            num_threads: Some(8),
            seed: Some(42),
            parallel_modules: false,
            output_dir: PathBuf::from("results"),
        }
    }
}

/// Конфигурация модуля центриоли
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CentrioleConfig {
    pub enabled: bool,
    pub acetylation_rate: f32,
    pub oxidation_rate: f32,
    pub parallel_cells: bool,
}

impl Default for CentrioleConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            acetylation_rate: 0.02,
            oxidation_rate: 0.01,
            parallel_cells: true,
        }
    }
}

/// Конфигурация модуля клеточного цикла
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CellCycleConfig {
    pub enabled: bool,
    pub base_cycle_time: f32,
    pub checkpoint_strictness: f32,
    pub enable_apoptosis: bool,
    pub nutrient_availability: f32,
    pub growth_factor_level: f32,
    pub random_variation: f32,
}

impl Default for CellCycleConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            base_cycle_time: 24.0,
            checkpoint_strictness: 0.15,
            enable_apoptosis: true,
            nutrient_availability: 0.9,
            growth_factor_level: 0.85,
            random_variation: 0.25,
        }
    }
}

/// Конфигурация модуля транскриптома
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TranscriptomeConfig {
    pub enabled: bool,
    pub mutation_rate: f32,
    pub noise_level: f32,
}

impl Default for TranscriptomeConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            mutation_rate: 0.001,
            noise_level: 0.05,
        }
    }
}

/// Конфигурация модуля ввода/вывода
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct IOConfig {
    pub enabled: bool,
    pub output_format: String,
    pub compression: String,
    pub buffer_size: usize,
}

impl Default for IOConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            output_format: "csv".to_string(),
            compression: "none".to_string(),
            buffer_size: 1000,
        }
    }
}

/// Полная конфигурация
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FullConfig {
    pub simulation: SimulationConfig,
    pub centriole_module: CentrioleConfig,
    pub cell_cycle_module: CellCycleConfig,
    pub transcriptome_module: TranscriptomeConfig,
    pub io_module: IOConfig,
}

/// Загрузчик конфигурации
pub struct ConfigLoader;

impl ConfigLoader {
    /// Загрузка из TOML файла
    pub fn from_toml(path: &str) -> Result<FullConfig, anyhow::Error> {
        let contents = std::fs::read_to_string(path)?;
        let config: FullConfig = toml::from_str(&contents)?;
        Ok(config)
    }
    
    /// Загрузка из YAML файла
    pub fn from_yaml(path: &str) -> Result<FullConfig, anyhow::Error> {
        let contents = std::fs::read_to_string(path)?;
        let config: FullConfig = serde_yaml::from_str(&contents)?;
        Ok(config)
    }
    
    /// Сохранение в TOML
    pub fn save_toml(config: &FullConfig, path: &str) -> Result<(), anyhow::Error> {
        let contents = toml::to_string_pretty(config)?;
        std::fs::write(path, contents)?;
        Ok(())
    }
    
    /// Сохранение в YAML
    pub fn save_yaml(config: &FullConfig, path: &str) -> Result<(), anyhow::Error> {
        let contents = serde_yaml::to_string(config)?;
        std::fs::write(path, contents)?;
        Ok(())
    }
}
