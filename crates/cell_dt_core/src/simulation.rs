use crate::{
    SimulationError, SimulationModule, SimulationResult,
    Dead,
    hecs::World,
};
use std::time::Instant;
use log::{info, debug, warn};

#[derive(Debug, Clone)]
pub struct SimulationConfig {
    pub max_steps: u64,
    pub dt: f64,
    pub checkpoint_interval: u64,
    pub num_threads: Option<usize>,
    pub seed: Option<u64>,
    pub parallel_modules: bool,
    /// Интервал очистки мёртвых сущностей (в шагах). `None` — очистка отключена.
    /// Мёртвые сущности определяются наличием компонента `Dead`.
    pub cleanup_dead_interval: Option<u64>,
}

impl Default for SimulationConfig {
    fn default() -> Self {
        Self {
            max_steps: 1000,
            dt: 0.1,
            checkpoint_interval: 100,
            num_threads: None,
            seed: Some(42),
            parallel_modules: false,
            cleanup_dead_interval: None,
        }
    }
}

pub struct SimulationManager {
    world: World,
    /// Модули хранятся в Vec для гарантии порядка выполнения.
    /// Порядок определяется порядком вызовов `register_module()`.
    modules: Vec<(String, Box<dyn SimulationModule>)>,
    config: SimulationConfig,
    current_step: u64,
    current_time: f64,
}

impl SimulationManager {
    pub fn new(config: SimulationConfig) -> Self {
        if let Some(seed) = config.seed {
            info!("Using random seed: {}", seed);
        }

        if let Some(num_threads) = config.num_threads {
            rayon::ThreadPoolBuilder::new()
                .num_threads(num_threads)
                .build_global()
                .unwrap_or_else(|_| warn!("Failed to set Rayon thread pool"));
        }

        Self {
            world: World::new(),
            modules: Vec::new(),
            config,
            current_step: 0,
            current_time: 0.0,
        }
    }

    pub fn register_module(&mut self, module: Box<dyn SimulationModule>) -> SimulationResult<()> {
        let name = module.name().to_string();

        if self.modules.iter().any(|(n, _)| n == &name) {
            return Err(SimulationError::ModuleError(
                format!("Module '{}' already registered", name)
            ));
        }

        info!("Registering module: {} (position {})", name, self.modules.len());
        self.modules.push((name, module));
        Ok(())
    }

    pub fn initialize(&mut self) -> SimulationResult<()> {
        info!("Initializing simulation with {} modules", self.modules.len());

        // Передаём seed каждому модулю перед инициализацией
        if let Some(seed) = self.config.seed {
            for (name, module) in self.modules.iter_mut() {
                debug!("Setting seed {} for module: {}", seed, name);
                module.set_seed(seed);
            }
        }

        for (name, module) in self.modules.iter_mut() {
            debug!("Initializing module: {}", name);
            module.initialize(&mut self.world)?;
        }

        Ok(())
    }

    pub fn step(&mut self) -> SimulationResult<()> {
        if self.current_step >= self.config.max_steps {
            return Ok(());
        }

        let dt = self.config.dt;

        // Модули выполняются в порядке регистрации (Vec гарантирует порядок)
        for (_, module) in self.modules.iter_mut() {
            module.step(&mut self.world, dt)?;
        }

        // Периодическая очистка мёртвых сущностей (компонент Dead)
        if let Some(interval) = self.config.cleanup_dead_interval {
            if interval > 0 && self.current_step % interval == 0 {
                let removed = self.cleanup_dead_entities();
                if removed > 0 {
                    debug!("Cleanup step {}: удалено {} мёртвых сущностей", self.current_step, removed);
                }
            }
        }

        self.current_step += 1;
        self.current_time += dt;

        Ok(())
    }

    pub fn run(&mut self) -> SimulationResult<()> {
        self.initialize()?;

        info!(
            "Starting simulation: {} steps, dt = {}",
            self.config.max_steps,
            self.config.dt,
        );

        let start_time = Instant::now();

        while self.current_step < self.config.max_steps {
            self.step()?;
        }

        let total_time = start_time.elapsed();
        info!("Simulation completed in {:?}. Final time: {}", total_time, self.current_time);

        Ok(())
    }

    /// Удалить все сущности с компонентом [`Dead`] из ECS-мира.
    ///
    /// Вызывается автоматически в [`step()`] с интервалом `cleanup_dead_interval`.
    /// Может также вызываться вручную для немедленной очистки.
    /// Возвращает число удалённых сущностей.
    pub fn cleanup_dead_entities(&mut self) -> usize {
        let dead: Vec<hecs::Entity> = self.world
            .query::<&Dead>()
            .iter()
            .map(|(e, _)| e)
            .collect();
        let count = dead.len();
        for entity in dead {
            let _ = self.world.despawn(entity);
        }
        count
    }

    pub fn world(&self) -> &World {
        &self.world
    }

    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    pub fn current_step(&self) -> u64 {
        self.current_step
    }

    pub fn current_time(&self) -> f64 {
        self.current_time
    }

    pub fn config(&self) -> &SimulationConfig {
        &self.config
    }

    /// Имена зарегистрированных модулей в порядке выполнения.
    pub fn module_names(&self) -> Vec<&str> {
        self.modules.iter().map(|(n, _)| n.as_str()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Dead;

    struct TestModule;

    impl SimulationModule for TestModule {
        fn name(&self) -> &str { "test_module" }
        fn step(&mut self, _world: &mut World, _dt: f64) -> SimulationResult<()> { Ok(()) }
        fn get_params(&self) -> serde_json::Value { serde_json::json!({}) }
        fn set_params(&mut self, _params: &serde_json::Value) -> SimulationResult<()> { Ok(()) }
    }

    struct OrderTracker {
        name: String,
        order_log: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
    }

    impl SimulationModule for OrderTracker {
        fn name(&self) -> &str { &self.name }
        fn step(&mut self, _world: &mut World, _dt: f64) -> SimulationResult<()> {
            self.order_log.lock().unwrap().push(self.name.clone());
            Ok(())
        }
        fn get_params(&self) -> serde_json::Value { serde_json::json!({}) }
        fn set_params(&mut self, _params: &serde_json::Value) -> SimulationResult<()> { Ok(()) }
    }

    #[test]
    fn test_simulation_manager_new() {
        let config = SimulationConfig::default();
        let sim = SimulationManager::new(config);
        assert_eq!(sim.current_step(), 0);
        assert_eq!(sim.current_time(), 0.0);
    }

    #[test]
    fn test_register_module() {
        let config = SimulationConfig::default();
        let mut sim = SimulationManager::new(config);

        let result = sim.register_module(Box::new(TestModule));
        assert!(result.is_ok());

        // Попытка зарегистрировать тот же модуль должна вернуть ошибку
        let result2 = sim.register_module(Box::new(TestModule));
        assert!(result2.is_err());
    }

    #[test]
    fn test_module_execution_order_is_guaranteed() {
        // Vec гарантирует порядок выполнения = порядок регистрации
        let log = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));

        let config = SimulationConfig { max_steps: 1, ..Default::default() };
        let mut sim = SimulationManager::new(config);

        for name in &["alpha", "beta", "gamma", "delta"] {
            sim.register_module(Box::new(OrderTracker {
                name: name.to_string(),
                order_log: log.clone(),
            })).unwrap();
        }

        sim.initialize().unwrap();
        sim.step().unwrap();

        let execution_order = log.lock().unwrap().clone();
        assert_eq!(execution_order, vec!["alpha", "beta", "gamma", "delta"],
            "Порядок выполнения модулей должен строго соответствовать порядку регистрации");
    }

    #[test]
    fn test_step_increment() {
        let config = SimulationConfig {
            max_steps: 10,
            dt: 0.5,
            ..Default::default()
        };

        let mut sim = SimulationManager::new(config);

        for i in 0..5 {
            sim.step().unwrap();
            assert_eq!(sim.current_step(), i + 1);
            assert_eq!(sim.current_time(), (i + 1) as f64 * 0.5);
        }
    }

    #[test]
    fn test_cleanup_removes_dead_entities() {
        let mut sim = SimulationManager::new(SimulationConfig::default());
        let dead_entity = sim.world_mut().spawn((Dead,));
        let alive_entity = sim.world_mut().spawn((crate::components::Position::default(),));

        let removed = sim.cleanup_dead_entities();

        assert_eq!(removed, 1, "Должна быть удалена 1 мёртвая сущность");
        assert!(!sim.world().contains(dead_entity), "Dead entity должна быть удалена");
        assert!(sim.world().contains(alive_entity), "Alive entity должна оставаться");
    }

    #[test]
    fn test_cleanup_preserves_alive_entities() {
        let mut sim = SimulationManager::new(SimulationConfig::default());
        let entity1 = sim.world_mut().spawn((crate::components::Position::default(),));
        let entity2 = sim.world_mut().spawn((crate::components::Position::default(),));

        let removed = sim.cleanup_dead_entities();

        assert_eq!(removed, 0, "Без Dead-маркеров ничего не должно удаляться");
        assert!(sim.world().contains(entity1));
        assert!(sim.world().contains(entity2));
    }
}
