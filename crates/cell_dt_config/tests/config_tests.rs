use cell_dt_config::*;
use tempfile::tempdir;

#[test]
fn test_default_config() {
    let config = FullConfig::default();
    assert!(config.centriole_module.enabled);
    assert!(config.cell_cycle_module.enabled);
    assert_eq!(config.simulation.max_steps, 10000);
}

#[test]
fn test_save_load_toml() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test.toml");
    let file_path_str = file_path.to_str().unwrap();
    
    let config = FullConfig::default();
    ConfigLoader::save_toml(&config, file_path_str).unwrap();
    
    let loaded = ConfigLoader::from_toml(file_path_str).unwrap();
    assert_eq!(loaded.simulation.max_steps, config.simulation.max_steps);
    assert_eq!(loaded.centriole_module.acetylation_rate, config.centriole_module.acetylation_rate);
}
