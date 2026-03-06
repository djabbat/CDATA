#!/bin/bash

echo "🔬 Cell DT Platform"
echo "==================="

case "$1" in
  test)
    echo "Running performance tests..."
    RUST_LOG=info cargo run --release --bin performance_test
    ;;
  *)
    echo "Running simple simulation..."
    RUST_LOG=info cargo run --bin simple_simulation
    ;;
esac
