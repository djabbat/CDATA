use crate::{CellData, IoResult};
use csv::Writer;
use std::path::Path;

pub fn write_csv(path: impl AsRef<Path>, cells: &[CellData]) -> IoResult<()> {
    let mut wtr = Writer::from_path(path)?;
    
    wtr.write_record(CellData::csv_headers())?;
    
    for cell in cells {
        wtr.write_record(cell.to_csv_record())?;
    }
    
    wtr.flush()?;
    Ok(())
}
