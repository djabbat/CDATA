#!/bin/bash

# Цвета для вывода
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== Cell DT - Управление конфигурациями ===${NC}\n"

case "$1" in
  list)
    echo -e "${GREEN}Доступные конфигурации:${NC}"
    ls -la configs/*.{toml,yaml,yml} 2>/dev/null | sed 's/^/  /'
    ;;
    
  show)
    if [ -z "$2" ]; then
      echo -e "${RED}Ошибка: укажите имя файла${NC}"
      echo "Использование: ./manage_configs.sh show configs/example.toml"
      exit 1
    fi
    echo -e "${GREEN}Содержимое $2:${NC}"
    cat "$2"
    ;;
    
  create)
    echo -e "${GREEN}Создание новой конфигурации...${NC}"
    cp configs/example.toml "configs/new_config_$(date +%Y%m%d_%H%M%S).toml"
    echo "✅ Создано"
    ;;
    
  validate)
    if [ -z "$2" ]; then
      echo -e "${RED}Ошибка: укажите файл для проверки${NC}"
      exit 1
    fi
    echo -e "${GREEN}Проверка $2...${NC}"
    cargo run --bin config_example -- --validate "$2" 2>/dev/null || \
      echo -e "${RED}❌ Файл не является валидной конфигурацией${NC}"
    ;;
    
  default)
    echo -e "${GREEN}Создание конфигурации по умолчанию...${NC}"
    cargo run --bin config_example
    ;;
    
  *)
    echo "Использование: $0 {list|show|create|validate|default}"
    echo ""
    echo "  list     - показать все конфигурации"
    echo "  show     - показать содержимое конфигурации"
    echo "  create   - создать новую конфигурацию из шаблона"
    echo "  validate - проверить конфигурацию"
    echo "  default  - создать конфигурацию по умолчанию"
    ;;
esac
