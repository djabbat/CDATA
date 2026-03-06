//! CDATA-временные ряды: визуализирует `damage_score`, `myeloid_bias`,
//! `spindle_fidelity` и `frailty` как функции времени (в годах).
//!
//! Строит PNG-файл с 4-панельным графиком через `plotters`.

use cell_dt_core::hecs::World;
use human_development_module::HumanDevelopmentComponent;
use myeloid_shift_module::MyeloidShiftComponent;
use plotters::prelude::*;

/// Средние CDATA-метрики по всем живым нишам на одном шаге симуляции.
#[derive(Debug, Clone)]
pub struct CdataSnapshot {
    pub step: u64,
    /// Возраст (лет) — берётся от первой живой ниши (все ниши симулируют одного организма)
    pub age_years: f64,
    /// Среднее молекулярное повреждение по всем нишам [0..1]
    pub mean_damage_score: f32,
    /// Средний миелоидный сдвиг [0..1] (0.0, если модуль не зарегистрирован)
    pub mean_myeloid_bias: f32,
    /// Средняя целостность веретена [0..1]
    pub mean_spindle_fidelity: f32,
    /// Средняя дряхлость (frailty = 1 − functional_capacity) [0..1]
    pub mean_frailty: f32,
    /// Число живых ниш
    pub alive_count: usize,
}

impl CdataSnapshot {
    /// Собрать снимок из ECS-мира на текущем шаге.
    pub fn from_world(world: &World, step: u64) -> Option<Self> {
        let mut age_years = 0.0f64;
        let mut total_damage = 0.0f32;
        let mut total_myeloid = 0.0f32;
        let mut total_spindle = 0.0f32;
        let mut total_frailty = 0.0f32;
        let mut count = 0usize;
        let mut myeloid_count = 0usize;

        for (_, (comp, myeloid_opt)) in world
            .query::<(&HumanDevelopmentComponent, Option<&MyeloidShiftComponent>)>()
            .iter()
        {
            if !comp.is_alive { continue; }
            count += 1;
            age_years = comp.age_years();
            total_damage  += comp.damage_score();
            total_spindle += comp.centriolar_damage.spindle_fidelity;
            total_frailty += comp.frailty();
            if let Some(m) = myeloid_opt {
                total_myeloid += m.myeloid_bias;
                myeloid_count += 1;
            }
        }

        if count == 0 { return None; }

        Some(CdataSnapshot {
            step,
            age_years,
            mean_damage_score:   total_damage  / count as f32,
            mean_myeloid_bias:   if myeloid_count > 0 { total_myeloid / myeloid_count as f32 } else { 0.0 },
            mean_spindle_fidelity: total_spindle / count as f32,
            mean_frailty:        total_frailty / count as f32,
            alive_count: count,
        })
    }
}

/// Визуализатор временных рядов CDATA-метрик.
///
/// # Использование
/// ```ignore
/// use cell_dt_viz::CdataTimeSeriesVisualizer;
///
/// let mut viz = CdataTimeSeriesVisualizer::new(100);
/// // в цикле симуляции:
/// viz.collect(sim.world(), sim.current_step());
/// // в конце симуляции:
/// viz.plot("output/cdata_timeseries.png").unwrap();
/// ```
pub struct CdataTimeSeriesVisualizer {
    /// История снимков
    history: Vec<CdataSnapshot>,
    /// Собирать снимок только каждые N шагов
    collect_interval: u64,
}

impl CdataTimeSeriesVisualizer {
    pub fn new(collect_interval: u64) -> Self {
        Self {
            history: Vec::new(),
            collect_interval,
        }
    }

    /// Собрать снимок (если шаг кратен `collect_interval`)
    pub fn collect(&mut self, world: &World, step: u64) {
        if self.collect_interval > 0 && step % self.collect_interval != 0 { return; }
        if let Some(snap) = CdataSnapshot::from_world(world, step) {
            self.history.push(snap);
        }
    }

    /// Число накопленных снимков
    pub fn snapshot_count(&self) -> usize { self.history.len() }

    /// Построить 4-панельный PNG-график.
    ///
    /// Панели (сверху вниз):
    /// 1. `damage_score` (красный)
    /// 2. `myeloid_bias` (оранжевый)
    /// 3. `spindle_fidelity` (синий)
    /// 4. `frailty` (фиолетовый)
    pub fn plot(&self, output_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        if self.history.is_empty() { return Ok(()); }

        let root = BitMapBackend::new(output_path, (1200, 900)).into_drawing_area();
        root.fill(&WHITE)?;

        let panels = root.split_evenly((4, 1));

        let ages: Vec<f32> = self.history.iter().map(|s| s.age_years as f32).collect();
        let age_range = ages.first().copied().unwrap_or(0.0)
            ..ages.last().copied().unwrap_or(1.0);

        let metrics: [(&str, Vec<f32>, RGBColor); 4] = [
            (
                "Damage Score",
                self.history.iter().map(|s| s.mean_damage_score).collect(),
                RGBColor(200, 30, 30),
            ),
            (
                "Myeloid Bias",
                self.history.iter().map(|s| s.mean_myeloid_bias).collect(),
                RGBColor(230, 120, 0),
            ),
            (
                "Spindle Fidelity",
                self.history.iter().map(|s| s.mean_spindle_fidelity).collect(),
                RGBColor(30, 90, 200),
            ),
            (
                "Frailty",
                self.history.iter().map(|s| s.mean_frailty).collect(),
                RGBColor(130, 30, 180),
            ),
        ];

        for (panel, (label, values, color)) in panels.iter().zip(metrics.iter()) {
            let y_max = values.iter().cloned().fold(0.0f32, f32::max).max(0.01);

            let mut chart = ChartBuilder::on(panel)
                .caption(*label, ("sans-serif", 18))
                .margin(8)
                .x_label_area_size(25)
                .y_label_area_size(45)
                .build_cartesian_2d(age_range.clone(), 0.0f32..y_max * 1.1)?;

            chart.configure_mesh()
                .x_desc("Age (years)")
                .y_desc(*label)
                .draw()?;

            chart.draw_series(LineSeries::new(
                ages.iter().zip(values.iter()).map(|(&x, &y)| (x, y)),
                color,
            ))?;
        }

        root.present()?;
        Ok(())
    }
}
