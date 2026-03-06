use cell_dt_core::*;
use cell_dt_core::hecs::World;

#[test]
fn test_simulation_with_cells() {
    let config = SimulationConfig {
        max_steps: 10,
        dt: 0.1,
        ..Default::default()
    };
    
    let mut sim = SimulationManager::new(config);
    
    // Добавляем тестовые клетки
    let world = sim.world_mut();
    for _ in 0..5 {
        world.spawn(());
    }
    
    sim.initialize().unwrap();
    
    for _ in 0..5 {
        sim.step().unwrap();
    }
    
    assert_eq!(sim.current_step(), 5);
    assert_eq!(sim.current_time(), 0.5);
}

#[test]
fn test_multiple_modules() {
    struct ModuleA;
    struct ModuleB;
    
    impl SimulationModule for ModuleA {
        fn name(&self) -> &str { "module_a" }
        fn step(&mut self, _world: &mut World, _dt: f64) -> SimulationResult<()> { Ok(()) }
        fn get_params(&self) -> serde_json::Value { serde_json::json!({}) }
        fn set_params(&mut self, _params: &serde_json::Value) -> SimulationResult<()> { Ok(()) }
    }
    
    impl SimulationModule for ModuleB {
        fn name(&self) -> &str { "module_b" }
        fn step(&mut self, _world: &mut World, _dt: f64) -> SimulationResult<()> { Ok(()) }
        fn get_params(&self) -> serde_json::Value { serde_json::json!({}) }
        fn set_params(&mut self, _params: &serde_json::Value) -> SimulationResult<()> { Ok(()) }
    }
    
    let config = SimulationConfig::default();
    let mut sim = SimulationManager::new(config);
    
    // Проверяем регистрацию модулей
    let result1 = sim.register_module(Box::new(ModuleA));
    assert!(result1.is_ok());
    
    let result2 = sim.register_module(Box::new(ModuleB));
    assert!(result2.is_ok());
    
    // Проверяем, что нельзя зарегистрировать модуль с тем же именем
    let result3 = sim.register_module(Box::new(ModuleA));
    assert!(result3.is_err());
}

#[test]
fn test_world_operations() {
    let config = SimulationConfig::default();
    let mut sim = SimulationManager::new(config);
    
    // Добавляем клетки
    let entities: Vec<_> = (0..10).map(|_| {
        sim.world_mut().spawn(())
    }).collect();
    
    assert_eq!(entities.len(), 10);
    
    // Проверяем количество клеток
    let count = sim.world().query::<()>().iter().count();
    assert_eq!(count, 10);
}
