//! Ткани и органы человека


/// Псевдоним для [`cell_dt_core::components::TissueType`].
///
/// Ранее существовал как отдельный enum с теми же вариантами.
/// Слит в единый тип в `cell_dt_core` — псевдоним сохранён
/// для обратной совместимости публичного API.
pub use cell_dt_core::components::TissueType as HumanTissueType;

/// Параметры развития тканей
#[derive(Debug, Clone)]
pub struct TissueDevelopmentParams {
    /// Скорость роста ткани
    pub growth_rate: f32,
    /// Максимальный размер
    pub max_size: f32,
    /// Скорость дифференцировки
    pub differentiation_rate: f32,
    /// Зависимость от центриоли
    pub centriole_dependence: f32,
    /// Уровень васкуляризации
    pub vascularization: f32,
    /// Иннервация
    pub innervation: f32,
}

impl TissueDevelopmentParams {
    pub fn new(
        growth_rate: f32,
        max_size: f32,
        differentiation_rate: f32,
        centriole_dependence: f32,
        vascularization: f32,
        innervation: f32,
        _p_sym: f32,  // Неиспользуемый параметр
    ) -> Self {
        Self {
            growth_rate,
            max_size,
            differentiation_rate,
            centriole_dependence,
            vascularization,
            innervation,
        }
    }
}
