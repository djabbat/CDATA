use crate::{SimulationResult, hecs::World};
use serde_json::Value;

pub trait SimulationModule: Send + Sync {
    fn name(&self) -> &str;
    fn step(&mut self, world: &mut World, dt: f64) -> SimulationResult<()>;
    fn get_params(&self) -> Value;
    fn set_params(&mut self, params: &Value) -> SimulationResult<()>;

    fn initialize(&mut self, _world: &mut World) -> SimulationResult<()> {
        Ok(())
    }

    fn cleanup(&mut self) -> SimulationResult<()> {
        Ok(())
    }

    /// Установить случайный seed для воспроизводимости симуляций.
    ///
    /// Вызывается [`SimulationManager::initialize()`] перед [`initialize()`].
    /// Модули с рандомностью должны переопределить этот метод и сохранить seed
    /// для использования в [`step()`] вместо `rand::thread_rng()`.
    ///
    /// По умолчанию — no-op (модуль не использует RNG или использует thread_rng).
    fn set_seed(&mut self, _seed: u64) {}
}
