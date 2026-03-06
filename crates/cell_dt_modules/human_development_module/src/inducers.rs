//! Индукторы развития и кислородная логика отщепления (CDATA)

use serde::{Deserialize, Serialize};
use rand::Rng;
use cell_dt_core::components::{CentriolarDamageState, CentriolarInducerPair};

/// Уровни морфогенеза человека
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HumanMorphogeneticLevel {
    /// Зародышевый (0–2 недели)
    Embryonic,
    /// Эмбриональный (2–8 недель)
    Fetal,
    /// Плодный (9–40 недель)
    Prenatal,
    /// Постнатальный (после рождения)
    Postnatal,
    /// Взрослый
    Adult,
    /// Старение
    Aging,
}

/// Вспомогательные функции для морфогенетических уровней
pub struct HumanInducers;

impl HumanInducers {
    /// Определить морфогенетический уровень по возрасту (в днях)
    pub fn get_morphogenetic_level(age_days: f64) -> HumanMorphogeneticLevel {
        if age_days < 14.0 {
            HumanMorphogeneticLevel::Embryonic
        } else if age_days < 56.0 {
            HumanMorphogeneticLevel::Fetal
        } else if age_days < 280.0 {
            HumanMorphogeneticLevel::Prenatal
        } else if age_days < 6570.0 {
            // 18 лет
            HumanMorphogeneticLevel::Postnatal
        } else if age_days < 18250.0 {
            // 50 лет
            HumanMorphogeneticLevel::Adult
        } else {
            HumanMorphogeneticLevel::Aging
        }
    }
}

// ---------------------------------------------------------------------------
// Кислородная логика отщепления индукторов (CDATA)
// ---------------------------------------------------------------------------

/// Вычислить уровень O₂ у центриолей из молекулярного состояния центриоли.
///
/// В норме митохондрии поглощают весь кислород до центра клетки.
/// По мере накопления повреждений (ROS, агрегаты, карбонилирование)
/// митохондриальный щит слабеет, и O₂ проникает к центриолям.
///
/// Возвращает [0..1]: 0 = полный щит (центриоли защищены), 1 = максимальное воздействие.
pub fn centrosomal_oxygen_level(damage: &CentriolarDamageState) -> f32 {
    let mito_shield = (1.0
        - damage.ros_level         * 0.50   // ROS — главный разрушитель щита
        - damage.protein_aggregates * 0.30   // агрегаты CPAP/CEP290 блокируют митофагию
        - damage.protein_carbonylation * 0.20) // карбонилирование SAS-6 снижает активность
        .max(0.0);
    (1.0 - mito_shield).clamp(0.0, 1.0)
}

/// PTM-опосредованное истощение материнского комплекта индукторов.
///
/// Второй, независимый от O₂ механизм. Структурные ПТМ (ацетилирование,
/// карбонилирование, фосфорилирование) ослабляют связи индукторов с молекулярным
/// каркасом центриоли. Чем сильнее PTM-асимметрия мать−дочь, тем выше вероятность
/// потери индуктора материнским комплектом.
///
/// **CDATA:** это механизм ИСТОЩЕНИЯ стволовых клеток, а не нормальной
/// дифференцировки: клетку не выбор, а молекулярный износ выталкивает из
/// стволового состояния. Применяется ТОЛЬКО к матери.
///
/// Вероятность = `ptm_asymmetry × ptm_exhaustion_scale`
/// `ptm_asymmetry` = (mother_ptm_avg − daughter_ptm_avg).max(0.0)
pub fn detach_by_ptm_exhaustion(
    pair: &mut CentriolarInducerPair,
    ptm_asymmetry: f32,
    rng: &mut impl Rng,
) -> bool {
    if !pair.mother_set.has_any() { return false; }
    let scale = pair.detachment_params.ptm_exhaustion_scale;
    if scale <= 0.0 { return false; }
    let prob = (ptm_asymmetry * scale).clamp(0.0, 1.0);
    if prob > 0.0 && rng.gen::<f32>() < prob {
        pair.mother_set.detach_one();
        return true;
    }
    false
}

/// Отщепить индукторы от центриолей при O₂-воздействии на центриолярную зону.
///
/// # Логика отщепления:
/// - **Оба комплекта непусты (тотипотент / плюрипотент):**
///   отщепляем от обоих с вероятностями `mother_prob` и `daughter_prob`.
///   Материнская центриоль уязвимее (больше накопленных ПТМ).
/// - **Один комплект пуст:**
///   отщепляем только от непустого с базовой вероятностью.
/// - **Оба пусты:**
///   ничего (апоптоз должен быть запущен выше).
///
/// Возвращает `true` если хотя бы один индуктор был отщеплён.
pub fn detach_by_oxygen(
    pair: &mut CentriolarInducerPair,
    oxygen_level: f32,
    age_years: f32,
    rng: &mut impl Rng,
) -> bool {
    if oxygen_level <= 0.0 {
        return false;
    }

    let m_has = pair.mother_set.has_any();
    let d_has = pair.daughter_set.has_any();
    let mut detached = false;
    let params = pair.detachment_params.clone();

    match (m_has, d_has) {
        (true, true) => {
            // Тотипотент или Плюрипотент: O₂ отщепляет от обоих комплектов
            if rng.gen::<f32>() < params.mother_prob(oxygen_level, age_years) {
                pair.mother_set.detach_one();
                detached = true;
            }
            if rng.gen::<f32>() < params.daughter_prob(oxygen_level, age_years) {
                pair.daughter_set.detach_one();
                detached = true;
            }
        }
        (false, true) => {
            // M уже пуст (Олиго/Унипотент): отщепляем только от D
            let p = oxygen_level * params.base_detach_probability;
            if rng.gen::<f32>() < p {
                pair.daughter_set.detach_one();
                detached = true;
            }
        }
        (true, false) => {
            // D уже пуст (Олиго/Унипотент): отщепляем только от M
            let p = oxygen_level * params.base_detach_probability;
            if rng.gen::<f32>() < p {
                pair.mother_set.detach_one();
                detached = true;
            }
        }
        (false, false) => {
            // Оба пусты — апоптоз уже должен быть инициирован
        }
    }

    detached
}

#[cfg(test)]
mod tests {
    use super::*;
    use cell_dt_core::components::{CentriolarDamageState, CentriolarInducerPair, PotencyLevel};

    // --- PTM-exhaustion tests ---

    #[test]
    fn test_ptm_exhaustion_zero_asymmetry_no_detach() {
        // Нет PTM-асимметрии → мать не теряет индукторы по этому пути
        let mut pair = CentriolarInducerPair::zygote(10, 8);
        let initial_m = pair.mother_set.remaining;
        let mut rng = rand::thread_rng();
        // 1000 попыток: без асимметрии — не должно ни разу сработать при scale=0.001
        for _ in 0..1000 {
            detach_by_ptm_exhaustion(&mut pair, 0.0, &mut rng);
        }
        assert_eq!(pair.mother_set.remaining, initial_m,
            "no asymmetry → mother should not lose inducers");
    }

    #[test]
    fn test_ptm_exhaustion_zero_scale_disabled() {
        // ptm_exhaustion_scale = 0 → механизм отключён
        let mut pair = CentriolarInducerPair::zygote(10, 8);
        pair.detachment_params.ptm_exhaustion_scale = 0.0;
        let initial_m = pair.mother_set.remaining;
        let mut rng = rand::thread_rng();
        for _ in 0..1000 {
            detach_by_ptm_exhaustion(&mut pair, 0.9, &mut rng);
        }
        assert_eq!(pair.mother_set.remaining, initial_m,
            "scale=0 → mechanism disabled, mother unchanged");
    }

    #[test]
    fn test_ptm_exhaustion_high_asymmetry_detaches_mother_only() {
        // Высокая PTM-асимметрия → мать теряет индукторы, дочь — нет
        let mut pair = CentriolarInducerPair::zygote(10, 8);
        pair.detachment_params.ptm_exhaustion_scale = 1.0; // 100% вероятность
        let initial_d = pair.daughter_set.remaining;
        let mut rng = rand::thread_rng();
        // При scale=1.0 и asymmetry=1.0, prob=1.0 → каждый вызов снимает один индуктор
        detach_by_ptm_exhaustion(&mut pair, 1.0, &mut rng);
        assert_eq!(pair.mother_set.remaining, 9,
            "high asymmetry → mother loses 1 inductor");
        assert_eq!(pair.daughter_set.remaining, initial_d,
            "daughter is unaffected by ptm exhaustion path");
    }

    #[test]
    fn test_ptm_exhaustion_daughter_unchanged_vs_oxygen() {
        // Кислородный путь затрагивает обе центриоли; PTM-путь — только мать.
        // Проверяем, что дочь остаётся нетронутой при PTM-истощении.
        let mut pair = CentriolarInducerPair::zygote(10, 8);
        pair.detachment_params.ptm_exhaustion_scale = 0.5;
        let d_before = pair.daughter_set.remaining;
        let mut rng = rand::thread_rng();
        for _ in 0..500 {
            detach_by_ptm_exhaustion(&mut pair, 0.8, &mut rng);
        }
        // Дочь должна быть неизменной
        assert_eq!(pair.daughter_set.remaining, d_before,
            "ptm exhaustion path must not touch daughter set");
    }

    #[test]
    fn test_morphogenetic_level() {
        assert!(matches!(
            HumanInducers::get_morphogenetic_level(7.0),
            HumanMorphogeneticLevel::Embryonic
        ));
        assert!(matches!(
            HumanInducers::get_morphogenetic_level(20000.0),
            HumanMorphogeneticLevel::Aging
        ));
    }

    #[test]
    fn test_centrosomal_oxygen_pristine() {
        let damage = CentriolarDamageState::pristine();
        // Молодая клетка: ROS=0.05, нет агрегатов → очень мало O₂ у центриолей
        let oxygen = centrosomal_oxygen_level(&damage);
        assert!(oxygen < 0.1, "pristine cell should have low centrosomal O₂, got {}", oxygen);
    }

    #[test]
    fn test_centrosomal_oxygen_damaged() {
        let mut damage = CentriolarDamageState::pristine();
        damage.ros_level = 0.8;
        damage.protein_aggregates = 0.7;
        let oxygen = centrosomal_oxygen_level(&damage);
        assert!(oxygen > 0.5, "damaged cell should have high centrosomal O₂, got {}", oxygen);
    }

    #[test]
    fn test_potency_progression() {
        let mut pair = CentriolarInducerPair::zygote(3, 2);
        assert_eq!(pair.potency_level(), PotencyLevel::Totipotent);

        pair.mother_set.detach_one();
        assert_eq!(pair.potency_level(), PotencyLevel::Pluripotent);

        pair.daughter_set.remaining = 0;
        assert_eq!(pair.potency_level(), PotencyLevel::Oligopotent);

        pair.mother_set.remaining = 1;
        assert_eq!(pair.potency_level(), PotencyLevel::Unipotent);

        pair.mother_set.remaining = 0;
        assert_eq!(pair.potency_level(), PotencyLevel::Apoptosis);
    }

    #[test]
    fn test_divide_inheritance() {
        let mut pair = CentriolarInducerPair::zygote(10, 8);
        // Симулируем частичную потерю
        pair.mother_set.remaining = 7;
        pair.daughter_set.remaining = 5;

        let (cell_a, cell_b) = pair.divide();

        // Клетка A наследует материнскую (7) + новую дочернюю (7, от матери)
        assert_eq!(cell_a.mother_set.remaining, 7);
        assert_eq!(cell_a.daughter_set.remaining, 7);
        assert_eq!(cell_a.daughter_set.inherited_count, 7); // не 8!

        // Клетка B наследует старую дочернюю (5) + новую дочернюю (5, от дочки)
        assert_eq!(cell_b.mother_set.remaining, 5);
        assert_eq!(cell_b.daughter_set.remaining, 5);
        assert_eq!(cell_b.daughter_set.inherited_count, 5); // не 8!
    }
}
