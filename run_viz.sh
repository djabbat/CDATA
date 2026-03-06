#!/bin/bash

echo "🔬 Cell DT Platform with Visualization"
echo "======================================"

# Создаем директорию для выходных файлов
mkdir -p viz_output

# Запускаем пример с визуализацией
RUST_LOG=info cargo run --bin viz_example

echo -e "\n📊 Generated visualizations:"
ls -la viz_output/
