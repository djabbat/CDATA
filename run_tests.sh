#!/bin/bash

# Цвета для вывода
GREEN='\033[0;32m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}================================${NC}"
echo -e "${BLUE}  Cell DT - Запуск всех тестов  ${NC}"
echo -e "${BLUE}================================${NC}\n"

# Функция для запуска тестов с замером времени
run_test_suite() {
    local name=$1
    local command=$2
    
    echo -e "${BLUE}▶ Запуск $name...${NC}"
    
    local start=$(date +%s.%N)
    eval $command
    local exit_code=$?
    local end=$(date +%s.%N)
    local duration=$(echo "$end - $start" | bc)
    
    if [ $exit_code -eq 0 ]; then
        echo -e "${GREEN}✅ $name завершены успешно (${duration}s)${NC}\n"
    else
        echo -e "${RED}❌ $name завершились с ошибкой (${duration}s)${NC}\n"
        exit 1
    fi
}

# 1. Линтинг
run_test_suite "cargo fmt (проверка форматирования)" "cargo fmt -- --check"

# 2. Clippy (статический анализ)
run_test_suite "cargo clippy" "cargo clippy -- -D warnings"

# 3. Unit тесты
run_test_suite "unit тесты" "cargo test --lib"

# 4. Интеграционные тесты
run_test_suite "интеграционные тесты" "cargo test --test '*'"

# 5. Тесты документации
run_test_suite "тесты документации" "cargo test --doc"

# 6. Проверка всех примеров
run_test_suite "проверка примеров" "cargo build --examples"

# 7. Бенчмарки (если есть)
if [ -d "benches" ]; then
    run_test_suite "бенчмарки" "cargo bench"
fi

echo -e "${GREEN}================================${NC}"
echo -e "${GREEN}  ✅ Все тесты успешно пройдены  ${NC}"
echo -e "${GREEN}================================${NC}"

# Показываем статистику покрытия, если установлен tarpaulin
if command -v cargo-tarpaulin &> /dev/null; then
    echo -e "\n${BLUE}📊 Статистика покрытия кода:${NC}"
    cargo tarpaulin --out Html
    echo -e "\n📁 Отчет сохранен в tarpaulin-report.html"
fi
