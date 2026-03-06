#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Пример использования Cell DT из Python
"""

import cell_dt
import numpy as np
import matplotlib.pyplot as plt
from collections import Counter

def basic_simulation():
    """Базовая симуляция"""
    print("=" * 50)
    print("Basic Simulation")
    print("=" * 50)
    
    # Создаем симуляцию
    sim = cell_dt.PySimulation(
        max_steps=500,
        dt=0.1,
        num_threads=4,
        seed=42
    )
    
    # Создаем популяцию клеток
    sim.create_population(100)
    print(f"Created {sim.cell_count()} cells")
    
    # Регистрируем модули
    sim.register_modules(
        enable_centriole=True,
        enable_cell_cycle=True,
        enable_transcriptome=False,
        cell_cycle_params=None  # используем параметры по умолчанию
    )
    
    # Запускаем симуляцию
    print("Running simulation...")
    cells = sim.run()
    print(f"Simulation completed at step {sim.current_step()}")
    
    # Анализируем результаты
    phases = [cell.cell_cycle.phase for cell in cells]
    phase_counts = Counter(phases)
    
    print("\nPhase distribution:")
    for phase, count in phase_counts.items():
        print(f"  {phase}: {count}")
    
    return cells

def advanced_simulation():
    """Продвинутая симуляция с транскриптомом и визуализацией"""
    print("\n" + "=" * 50)
    print("Advanced Simulation with Transcriptome")
    print("=" * 50)
    
    # Создаем симуляцию
    sim = cell_dt.PySimulation(
        max_steps=500,
        dt=0.1,
        num_threads=4,
        seed=42
    )
    
    # Создаем популяцию с транскриптомом
    sim.create_population_with_transcriptome(50)
    print(f"Created {sim.cell_count()} cells with transcriptome")
    
    # Регистрируем модули (без параметров клеточного цикла)
    sim.register_modules(
        enable_centriole=True,
        enable_cell_cycle=True,
        enable_transcriptome=True,
        cell_cycle_params=None  # используем параметры по умолчанию
    )
    
    # Запускаем симуляцию
    print("Running simulation...")
    cells = sim.run()
    print(f"Simulation completed at step {sim.current_step()}")
    
    # Анализируем результаты
    phases = [cell.cell_cycle.phase for cell in cells]
    phase_counts = Counter(phases)
    
    print("\nPhase distribution:")
    for phase, count in phase_counts.items():
        print(f"  {phase}: {count}")
    
    # Получаем данные центриолей как numpy array
    centriole_data = sim.get_centriole_data_numpy()
    print(f"\nCentriole data shape: {centriole_data.shape}")
    print(f"Average mother maturity: {np.mean(centriole_data[:, 0]):.3f}")
    print(f"Average daughter maturity: {np.mean(centriole_data[:, 1]):.3f}")
    
    return sim, cells

def main():
    """Основная функция"""
    print("Cell DT Python Interface Example")
    print("=" * 50)
    
    # Базовая симуляция
    cells = basic_simulation()
    
    # Продвинутая симуляция
    sim, cells2 = advanced_simulation()
    
    print("\n" + "=" * 50)
    print("All examples completed successfully!")
    print("=" * 50)

if __name__ == "__main__":
    main()
