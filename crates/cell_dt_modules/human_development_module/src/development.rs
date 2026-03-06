//! Параметры и логика прохождения стадий развития

use cell_dt_core::components::DevelopmentalStage;
use serde::{Deserialize, Serialize};

/// Параметры прохождения стадий развития
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevelopmentParams {
    /// Исходный запас H-индукторов (число индукторных делений до мейоза)
    pub h_inducers_initial: u32,
    /// Максимальная продолжительность жизни (лет) — смерть от старости
    pub max_lifespan_years: f64,
    /// Возраст смерти (лет) при фатальной сенесценции основных тканей
    pub senescence_death_frailty: f32,
}

impl Default for DevelopmentParams {
    fn default() -> Self {
        Self {
            h_inducers_initial:     4,
            max_lifespan_years:     120.0,
            senescence_death_frailty: 0.95,
        }
    }
}

/// Определить стадию развития по возрасту
pub fn stage_for_age(age_years: f64) -> DevelopmentalStage {
    // Эмбриональные стадии — в днях (переводим через дробные годы)
    let age_days = age_years * 365.25;
    match age_days {
        d if d < 1.0    => DevelopmentalStage::Zygote,
        d if d < 4.0    => DevelopmentalStage::Cleavage,
        d if d < 14.0   => DevelopmentalStage::Blastocyst,
        d if d < 28.0   => DevelopmentalStage::Gastrulation,
        d if d < 56.0   => DevelopmentalStage::Organogenesis,
        // Плодный период: 8 недель — 9 месяцев (~0.75 года)
        _ if age_years < 0.75   => DevelopmentalStage::Fetal,
        // Постнатальное развитие до 18 лет
        _ if age_years < 18.0   => DevelopmentalStage::Postnatal,
        // Взрослый: 18–40
        _ if age_years < 40.0   => DevelopmentalStage::Adult,
        // Средний возраст: 40–65
        _ if age_years < 65.0   => DevelopmentalStage::MiddleAge,
        // Пожилой: 65+
        _                       => DevelopmentalStage::Senescent,
    }
}

/// Скорость дифференцирующих делений (делений в год) — зависит от стадии
pub fn division_rate_per_year(stage: DevelopmentalStage) -> f32 {
    match stage {
        // В эмбриональный период деления очень частые
        DevelopmentalStage::Zygote        => 365.0 * 2.0,  // ~2/день
        DevelopmentalStage::Cleavage      => 365.0 * 1.5,
        DevelopmentalStage::Blastocyst    => 365.0 * 1.0,
        DevelopmentalStage::Gastrulation  => 365.0 * 0.5,
        DevelopmentalStage::Organogenesis => 365.0 * 0.3,
        DevelopmentalStage::Fetal         => 52.0,          // ~1/неделю
        DevelopmentalStage::Postnatal     => 24.0,          // ~2/месяц
        DevelopmentalStage::Adult         => 12.0,          // ~1/месяц
        // После 40 лет темп регенерации падает
        DevelopmentalStage::MiddleAge     => 6.0,
        DevelopmentalStage::Senescent     => 2.0,
        DevelopmentalStage::Death         => 0.0,
    }
}

/// Базовый уровень ROS в зависимости от стадии (относительные единицы)
pub fn base_ros_level(stage: DevelopmentalStage) -> f32 {
    match stage {
        DevelopmentalStage::Zygote        |
        DevelopmentalStage::Cleavage      |
        DevelopmentalStage::Blastocyst    => 0.02,  // De-novo центриоли — минимальный ущерб
        DevelopmentalStage::Gastrulation  |
        DevelopmentalStage::Organogenesis => 0.04,
        DevelopmentalStage::Fetal         => 0.05,
        DevelopmentalStage::Postnatal     => 0.06,
        DevelopmentalStage::Adult         => 0.08,
        DevelopmentalStage::MiddleAge     => 0.12,
        DevelopmentalStage::Senescent     => 0.20,
        DevelopmentalStage::Death         => 1.0,
    }
}
