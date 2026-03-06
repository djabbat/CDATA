//! Симулятор уровня организма: интеграция тканевых метрик
//! в глобальные показатели старения

use cell_dt_core::components::{DevelopmentalStage, OrganismState, TissueType};
use crate::{
    development::{stage_for_age, DevelopmentParams},
    tissues::TissueSimulator,
    HumanDevelopmentParams,
};

pub struct OrganismSimulator {
    pub state:  OrganismState,
    params: DevelopmentParams,
}

impl OrganismSimulator {
    pub fn new(params: &HumanDevelopmentParams) -> Self {
        Self {
            state:  OrganismState::new(),
            params: params.development.clone(),
        }
    }

    /// Увеличить возраст и обновить стадию развития
    pub fn advance(&mut self, dt_years: f64) {
        if !self.state.is_alive {
            return;
        }

        self.state.age_years += dt_years;
        self.state.developmental_stage = stage_for_age(self.state.age_years);

        // Смерть по максимальной продолжительности жизни
        if self.state.age_years >= self.params.max_lifespan_years {
            self.state.is_alive = false;
            self.state.developmental_stage = DevelopmentalStage::Death;
        }
    }

    /// Интегрировать тканевые метрики → глобальные показатели организма
    pub fn integrate_tissue_metrics(&mut self, tissues: &[TissueSimulator]) {
        if !self.state.is_alive {
            return;
        }

        let neural = find_tissue(tissues, TissueType::Neural);
        let hsc    = find_tissue(tissues, TissueType::Blood);
        let muscle = find_tissue(tissues, TissueType::Muscle);
        let gut    = find_tissue(tissues, TissueType::Epithelial);

        // --- Когнитивный индекс (нейральные стволовые клетки) ---
        if let Some(t) = neural {
            self.state.cognitive_index =
                (t.state.functional_capacity * 0.7
                 + t.damage.ciliary_function * 0.3)
                .max(0.0);
        }

        // --- Иммунный резерв (гемопоэтические стволовые клетки) ---
        if let Some(t) = hsc {
            self.state.immune_reserve =
                (t.state.functional_capacity * 0.8
                 + (1.0 - t.state.senescent_fraction) * 0.2)
                .max(0.0);
        }

        // --- Мышечная масса (саркопения) ---
        if let Some(t) = muscle {
            self.state.muscle_mass = t.state.functional_capacity.max(0.0);
        }

        // --- Inflammaging (SASP): запускается при сенесценции кишечника и HSC ---
        let gut_senescence = gut.map(|t| t.state.senescent_fraction).unwrap_or(0.0);
        let hsc_senescence = hsc.map(|t| t.state.senescent_fraction).unwrap_or(0.0);
        self.state.inflammaging_score =
            ((gut_senescence + hsc_senescence) / 2.0).min(1.0);

        // --- Индекс дряхлости (frailty) ---
        let total_capacity: f32 = tissues.iter()
            .map(|t| t.state.functional_capacity)
            .sum::<f32>() / tissues.len() as f32;
        self.state.frailty_index = (1.0 - total_capacity).max(0.0);

        // --- Смерть при критической дряхлости ---
        if self.state.frailty_index >= self.params.senescence_death_frailty {
            self.state.is_alive = false;
            self.state.developmental_stage = DevelopmentalStage::Death;
        }
    }
}

fn find_tissue(
    tissues: &[TissueSimulator],
    tissue_type: TissueType,
) -> Option<&TissueSimulator> {
    tissues.iter().find(|t| t.state.tissue_type == tissue_type)
}
