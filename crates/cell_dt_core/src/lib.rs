//! Ядро платформы симуляции клеточной дифференцировки

pub mod components;
pub mod error;
pub mod module;
pub mod simulation;
pub mod world;

pub use components::*;
pub use error::*;
pub use module::*;
pub use simulation::*;
pub use world::*;

pub use hecs;

/// Константы для тестов
#[cfg(test)]
pub mod test_constants {
    pub const TEST_CELL_COUNT: usize = 10;
    pub const TEST_STEPS: u64 = 100;
    pub const TEST_DT: f64 = 0.1;
}
