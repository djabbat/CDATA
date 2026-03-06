//! Модуль старения на основе центриолярной теории

use serde::{Deserialize, Serialize};

/// Типы возраст-зависимых изменений
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgingPhenotype {
    /// Снижение пролиферативной активности
    ReducedProliferation,
    /// Накопление поврежденных белков
    ProteinAggregation,
    /// Дисфункция митохондрий
    MitochondrialDysfunction,
    /// Укорочение теломер
    TelomereShortening,
    /// Эпигенетические изменения
    EpigeneticChanges,
    /// Накопление сенесцентных клеток
    SenescentAccumulation,
    /// Дисрегуляция сигнальных путей
    SignalingDysregulation,
    /// Потеря протеостаза
    ProteostasisLoss,
    /// Дисфункция стволовых клеток
    StemCellExhaustion,
    /// Изменение межклеточной коммуникации
    AlteredCommunication,
    /// Иммунное старение: миелоидный сдвиг + лимфоидная недостаточность
    ImmuneDecline,
}

/// Связь с центриолярными изменениями
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CentrioleAgingLink {
    /// Потеря первичных ресничек
    pub cilia_loss: f32,
    /// Накопление центриолярных ПТМ
    pub ptm_accumulation: f32,
    /// Дисрегуляция центриолярного цикла
    pub cycle_dysregulation: f32,
    /// Потеря асимметрии центриолей
    pub asymmetry_loss: f32,
    /// Накопление центриолярных сателлитов
    pub satellite_accumulation: f32,
}

impl Default for CentrioleAgingLink {
    fn default() -> Self {
        Self {
            cilia_loss: 0.0,
            ptm_accumulation: 0.0,
            cycle_dysregulation: 0.0,
            asymmetry_loss: 0.0,
            satellite_accumulation: 0.0,
        }
    }
}
