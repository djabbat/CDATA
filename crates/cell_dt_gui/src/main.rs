//! Запуск графического конфигуратора Cell DT

use cell_dt_gui::ConfigApp;
use eframe::{NativeOptions, egui};

fn main() -> eframe::Result<()> {
    let options = NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1024.0, 768.0])
            .with_min_inner_size([800.0, 600.0])
            .with_resizable(true)
            .with_active(true),
        ..Default::default()
    };
    
    eframe::run_native(
        "Cell DT - Конфигуратор симуляции",
        options,
        Box::new(|_cc| Box::new(ConfigApp::new())),
    )
}
