# Cell DT - Конфигурационные файлы

## Доступные конфигурации

| Файл | Назначение |
|------|------------|
| `development.toml` | Для разработки и отладки |
| `production.toml` | Для продакшн запусков |
| `benchmark.toml` | Для тестирования производительности |
| `example.toml` | Пример конфигурации в TOML |
| `example.yaml` | Пример конфигурации в YAML |

## Использование

```bash
# Показать все конфигурации
./manage_configs.sh list

# Просмотреть конфигурацию
./manage_configs.sh show configs/development.toml

# Создать новую конфигурацию из шаблона
./manage_configs.sh create

# Проверить конфигурацию
./manage_configs.sh validate configs/example.toml

# Создать конфигурацию по умолчанию
./manage_configs.sh default
Структура конфигурации
toml
[simulation]        # Основные параметры симуляции
[centriole_module]  # Параметры модуля центриоли
[cell_cycle_module] # Параметры клеточного цикла
[transcriptome_module] # Параметры транскриптома
[io_module]         # Параметры ввода/вывода
