use tutor_core::frac::Frac;
use tutor_core::generator::{SkillGenerator, WorkedExample};
use tutor_core::guard::analog_is_safe;
use tutor_core::rng::{seed_for, SeededRng};
use tutor_core::task::{validate_task, Task};

/// Перебирає кандидатів, доки задача не пройде `validate_task`. Детерміновано для (навичка, варіант).
pub fn build(
    skill: &'static str,
    variant: u64,
    mut make: impl FnMut(&mut SeededRng) -> Option<(Task, Frac)>,
) -> Task {
    let mut rng = SeededRng::new(seed_for(skill, variant));
    for _ in 0..2000 {
        if let Some((task, oracle)) = make(&mut rng) {
            if validate_task(&task, oracle).is_ok() {
                return task;
            }
        }
    }
    panic!("генератор {skill}: не вдалося побудувати коректну задачу для варіанта {variant}");
}

/// Шукає безпечний аналог серед сусідніх варіантів тієї ж навички.
pub fn find_analog(
    gen: &dyn SkillGenerator,
    task: &Task,
    avoid: &[Frac],
    describe: impl Fn(&Task) -> WorkedExample,
) -> Option<WorkedExample> {
    for j in 1..=300_u64 {
        let cand = gen.generate(task.variant.wrapping_add(j.wrapping_mul(1_000_003)));
        let ex = describe(&cand);
        if analog_is_safe(task, &ex, avoid) {
            return Some(ex);
        }
    }
    None
}
