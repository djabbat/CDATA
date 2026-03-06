#!/bin/bash

echo "🔬 Cell DT Performance Tests"
echo "============================\n"

# Сборка проекта
echo "Building project..."
cargo build --release

# Запуск тестов производительности
echo -e "\nRunning performance tests...\n"
RUST_LOG=info cargo run --release --bin performance_test

# Показываем статистику по ядрам процессора
echo -e "\n💻 System Info:"
echo "CPU cores: $(nproc)"
echo "Memory: $(free -h | grep Mem | awk '{print $2}')"
