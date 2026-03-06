use cell_dt_config::{ConfigLoader, FullConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Cell DT - Пример работы с конфигурацией ===\n");
    
    // 1. Создаем конфигурацию по умолчанию
    println!("1. Создание конфигурации по умолчанию:");
    let default_config = FullConfig::default();
    println!("   Шагов: {}", default_config.simulation.max_steps);
    println!("   Модуль центриоли: {}", default_config.centriole_module.enabled);
    println!("   Модуль клеточного цикла: {}", default_config.cell_cycle_module.enabled);
    
    // 2. Сохраняем конфигурацию в TOML
    println!("\n2. Сохранение конфигурации в TOML...");
    ConfigLoader::save_toml(&default_config, "configs/default.toml")?;
    println!("   ✅ Сохранено в configs/default.toml");
    
    // 3. Сохраняем конфигурацию в YAML
    println!("\n3. Сохранение конфигурации в YAML...");
    ConfigLoader::save_yaml(&default_config, "configs/default.yaml")?;
    println!("   ✅ Сохранено в configs/default.yaml");
    
    // 4. Загружаем и проверяем TOML конфигурацию
    println!("\n4. Загрузка TOML конфигурации...");
    let loaded_toml = ConfigLoader::from_toml("configs/example.toml")?;
    println!("   ✅ Загружено из configs/example.toml");
    println!("   Параметры симуляции:");
    println!("     - Шагов: {}", loaded_toml.simulation.max_steps);
    println!("     - dt: {}", loaded_toml.simulation.dt);
    println!("     - Потоков: {}", loaded_toml.simulation.num_threads.unwrap_or(1));
    
    // 5. Загружаем и проверяем YAML конфигурацию
    println!("\n5. Загрузка YAML конфигурации...");
    let loaded_yaml = ConfigLoader::from_yaml("configs/example.yaml")?;
    println!("   ✅ Загружено из configs/example.yaml");
    println!("   Параметры модулей:");
    println!("     - Центриоль: acetylation_rate = {}", loaded_yaml.centriole_module.acetylation_rate);
    println!("     - Клеточный цикл: strictness = {}", loaded_yaml.cell_cycle_module.checkpoint_strictness);
    println!("     - Транскриптом: mutation_rate = {}", loaded_yaml.transcriptome_module.mutation_rate);
    
    // 6. Создаем модифицированную конфигурацию
    println!("\n6. Создание пользовательской конфигурации...");
    let mut custom_config = FullConfig::default();
    custom_config.simulation.max_steps = 500;
    custom_config.simulation.dt = 0.05;
    custom_config.simulation.num_threads = Some(2);
    custom_config.centriole_module.acetylation_rate = 0.05;
    custom_config.cell_cycle_module.checkpoint_strictness = 0.05;
    custom_config.transcriptome_module.enabled = false;
    
    ConfigLoader::save_toml(&custom_config, "configs/custom.toml")?;
    println!("   ✅ Сохранено в configs/custom.toml");
    
    // 7. Выводим список всех конфигураций
    println!("\n7. Доступные конфигурационные файлы:");
    let configs = std::fs::read_dir("configs")?;
    for config in configs {
        let entry = config?;
        if let Some(ext) = entry.path().extension() {
            if ext == "toml" || ext == "yaml" || ext == "yml" {
                println!("   - {}", entry.file_name().to_string_lossy());
            }
        }
    }
    
    println!("\n✅ Пример завершен!");
    Ok(())
}
