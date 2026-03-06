#!/bin/bash

echo "🔬 Cell DT - Графический конфигуратор"
echo "======================================"
echo ""

# Проверяем наличие директории configs
if [ ! -d "configs" ]; then
    mkdir -p configs
    echo "📁 Создана директория configs/"
fi

# Запускаем GUI
cd /home/oem/Documents/Projects/rust/cell_dt
cargo run --bin cell_dt_gui
