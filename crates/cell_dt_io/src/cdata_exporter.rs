//! CDATA-экспортёр: собирает данные CDATA-компонентов из ECS и сохраняет в CSV.
//!
//! Колонки: step, entity_id, tissue, age_years, stage, damage_score,
//!          myeloid_bias, spindle_fidelity, ciliary_function, frailty, phenotype_count

use crate::IoResult;
use csv::Writer;
use human_development_module::HumanDevelopmentComponent;
use myeloid_shift_module::MyeloidShiftComponent;
use cell_dt_core::hecs::World;
use std::path::{Path, PathBuf};

/// Одна строка CDATA-экспорта (одна сущность, один шаг)
#[derive(Debug, Clone)]
pub struct CdataRecord {
    pub step: u64,
    pub entity_id: u64,
    pub tissue: String,
    pub age_years: f64,
    pub stage: String,
    pub damage_score: f32,
    /// Миелоидный сдвиг (0.0, если `MyeloidShiftComponent` отсутствует)
    pub myeloid_bias: f32,
    pub spindle_fidelity: f32,
    pub ciliary_function: f32,
    /// Frailty = 1 − functional_capacity
    pub frailty: f32,
    pub phenotype_count: usize,
}

impl CdataRecord {
    pub fn csv_headers() -> Vec<&'static str> {
        vec![
            "step", "entity_id", "tissue", "age_years", "stage",
            "damage_score", "myeloid_bias", "spindle_fidelity",
            "ciliary_function", "frailty", "phenotype_count",
        ]
    }

    pub fn to_csv_record(&self) -> Vec<String> {
        vec![
            self.step.to_string(),
            self.entity_id.to_string(),
            self.tissue.clone(),
            format!("{:.4}", self.age_years),
            self.stage.clone(),
            format!("{:.6}", self.damage_score),
            format!("{:.6}", self.myeloid_bias),
            format!("{:.6}", self.spindle_fidelity),
            format!("{:.6}", self.ciliary_function),
            format!("{:.6}", self.frailty),
            self.phenotype_count.to_string(),
        ]
    }
}

/// Экспортёр CDATA-данных из ECS-мира в CSV-файлы.
///
/// # Использование
/// ```ignore
/// use cell_dt_io::CdataExporter;
///
/// let mut exporter = CdataExporter::new("output/cdata", "run");
/// // в цикле симуляции:
/// exporter.collect(sim.world(), sim.current_step());
/// if step % 100 == 0 {
///     exporter.save_snapshot(step).unwrap();
/// }
/// ```
pub struct CdataExporter {
    output_dir: PathBuf,
    prefix: String,
    buffer: Vec<CdataRecord>,
}

impl CdataExporter {
    pub fn new(output_dir: impl AsRef<Path>, prefix: &str) -> Self {
        let output_dir = output_dir.as_ref().to_path_buf();
        let _ = std::fs::create_dir_all(&output_dir);
        Self {
            output_dir,
            prefix: prefix.to_string(),
            buffer: Vec::new(),
        }
    }

    /// Собрать снимок всех сущностей с `HumanDevelopmentComponent` на данном шаге.
    pub fn collect(&mut self, world: &World, step: u64) {
        for (entity, (comp, myeloid_opt)) in world
            .query::<(&HumanDevelopmentComponent, Option<&MyeloidShiftComponent>)>()
            .iter()
        {
            let record = CdataRecord {
                step,
                entity_id: entity.to_bits().get(),
                tissue: format!("{:?}", comp.tissue_type),
                age_years: comp.age_years(),
                stage: format!("{:?}", comp.stage),
                damage_score: comp.damage_score(),
                myeloid_bias: myeloid_opt.map_or(0.0, |m| m.myeloid_bias),
                spindle_fidelity: comp.centriolar_damage.spindle_fidelity,
                ciliary_function: comp.centriolar_damage.ciliary_function,
                frailty: comp.frailty(),
                phenotype_count: comp.active_phenotypes.len(),
            };
            self.buffer.push(record);
        }
    }

    /// Сохранить буфер в CSV-файл и очистить буфер.
    /// Путь: `<output_dir>/<prefix>_cdata_step_<NNNNNN>.csv`
    pub fn save_snapshot(&mut self, step: u64) -> IoResult<PathBuf> {
        let path = self.output_dir.join(format!(
            "{}_cdata_step_{:06}.csv",
            self.prefix, step
        ));
        write_cdata_csv(&path, &self.buffer)?;
        self.buffer.clear();
        Ok(path)
    }

    /// Число записей в буфере (до сохранения)
    pub fn buffered_records(&self) -> usize {
        self.buffer.len()
    }
}

/// Записать `CdataRecord`-записи в CSV-файл по указанному пути.
pub fn write_cdata_csv(path: impl AsRef<Path>, records: &[CdataRecord]) -> IoResult<()> {
    let mut wtr = Writer::from_path(path)?;
    wtr.write_record(CdataRecord::csv_headers())?;
    for rec in records {
        wtr.write_record(rec.to_csv_record())?;
    }
    wtr.flush()?;
    Ok(())
}
