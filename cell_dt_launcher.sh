#!/bin/bash

# Цвета для вывода
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${BLUE}╔════════════════════════════════════╗${NC}"
echo -e "${BLUE}║     Cell DT - Лаунчер              ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════╝${NC}"
echo ""

case "$1" in
    gui)
        echo -e "${GREEN}▶ Запуск графического конфигуратора...${NC}"
        cd /home/oem/Documents/Projects/rust/cell_dt
        cargo run --bin cell_dt_gui
        ;;
    run)
        CONFIG=${2:-"configs/default.toml"}
        echo -e "${GREEN}▶ Запуск симуляции с конфигурацией: ${YELLOW}$CONFIG${NC}"
        cd /home/oem/Documents/Projects/rust/cell_dt
        cargo run --bin run_simulation -- --config "$CONFIG" run --cells ${3:-1000}
        ;;
    example)
        EXAMPLE=${2:-"simple_simulation"}
        echo -e "${GREEN}▶ Запуск примера: ${YELLOW}$EXAMPLE${NC}"
        cd /home/oem/Documents/Projects/rust/cell_dt
        cargo run --bin "$EXAMPLE"
        ;;
    list)
        echo -e "${GREEN}▶ Доступные примеры:${NC}"
        cd /home/oem/Documents/Projects/rust/cell_dt/examples/src/bin
        ls -1 *.rs | sed 's/\.rs$//' | sed 's/^/  • /'
        ;;
    configs)
        echo -e "${GREEN}▶ Доступные конфигурации:${NC}"
        ls -1 configs/*.{toml,yaml,yml} 2>/dev/null | sed 's/^/  • /' || echo "  Нет конфигураций"
        ;;
    help|*)
        echo "Использование: $0 {команда} [параметры]"
        echo ""
        echo "Команды:"
        echo "  gui                    - Запустить графический конфигуратор"
        echo "  run [config] [cells]   - Запустить симуляцию с конфигурацией"
        echo "  example [name]         - Запустить пример"
        echo "  list                   - Список доступных примеров"
        echo "  configs                - Список конфигураций"
        echo "  help                   - Показать эту справку"
        echo ""
        echo "Примеры:"
        echo "  $0 gui"
        echo "  $0 run configs/production.toml 10000"
        echo "  $0 example stem_cell_example"
        ;;
esac
