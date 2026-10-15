//! Генератори задач математики 5 класу (повний курс: 35 навичок).

mod bug_messages;
mod common;
mod d_add;
mod d_sub;
mod eq_word;
mod f_add_diff;
mod f_add_same;
mod f_sub_diff;
mod f_sub_same;
mod figures;
mod frac_pct;
mod kit;
mod lessons;
mod lessons_ext;
mod lessons_ext2;
mod n_add_multi;
mod n_div_multi;
mod n_mul_multi;
mod n_order;
mod n_sub_multi;
mod nat;
mod sheet_content;
mod sheet_ext;
mod units_geo;

pub use d_add::DAdd;
pub use d_sub::DSub;
pub use f_add_diff::FAddDiff;
pub use f_add_same::FAddSame;
pub use f_sub_diff::FSubDiff;
pub use f_sub_same::FSubSame;
pub use lessons::MathLessons;
pub use n_add_multi::NAddMulti;
pub use n_div_multi::NDivMulti;
pub use n_mul_multi::NMulMulti;
pub use n_order::NOrder;
pub use n_sub_multi::NSubMulti;
pub use sheet_content::MathSheetContent;

use tutor_core::generator::SkillGenerator;

pub fn all_generators() -> Vec<Box<dyn SkillGenerator>> {
    vec![
        Box::new(NAddMulti),
        Box::new(DAdd),
        Box::new(FAddSame),
        Box::new(FAddDiff),
        Box::new(NSubMulti),
        Box::new(NMulMulti),
        Box::new(NDivMulti),
        Box::new(NOrder),
        Box::new(DSub),
        Box::new(FSubSame),
        Box::new(FSubDiff),
        // Розширення курсу (рішення A20): натуральні числа.
        Box::new(nat::N_ROUND),
        Box::new(nat::N_DIVREM),
        Box::new(nat::N_DIVTEST),
        Box::new(nat::N_GCD),
        Box::new(nat::N_LCM),
        Box::new(nat::N_POWER),
        // Рівняння й текстові задачі.
        Box::new(eq_word::EQ_ADD_SUB),
        Box::new(eq_word::EQ_MUL),
        Box::new(eq_word::WORD_1),
        Box::new(eq_word::WORD_2),
        Box::new(eq_word::MOTION),
        // Величини й геометрія.
        Box::new(units_geo::U_LEN),
        Box::new(units_geo::U_MASS),
        Box::new(units_geo::U_TIME),
        Box::new(units_geo::G_PERIM),
        Box::new(units_geo::G_AREA),
        Box::new(units_geo::G_VOL),
        Box::new(units_geo::A_ANGLE),
        // Дроби, відсотки, десяткові, дані.
        Box::new(frac_pct::F_MIXED),
        Box::new(frac_pct::F_REDUCE),
        Box::new(frac_pct::F_OF),
        Box::new(frac_pct::P_PCT),
        Box::new(frac_pct::D_MUL10),
        Box::new(frac_pct::AVG),
    ]
}

pub fn generator_for(skill_id: &str) -> Option<Box<dyn SkillGenerator>> {
    all_generators()
        .into_iter()
        .find(|g| g.skill_id() == skill_id)
}

/// Дитяче пояснення типової помилки (нейтральне, без чисел і без відповіді): «Схоже, … Спробуй…».
/// `None`, якщо для цієї помилки тексту ще немає (тоді показується загальний відгук).
pub fn bug_message(skill: &str, bug: &str) -> Option<&'static str> {
    bug_messages::bug_message(skill, bug)
}
