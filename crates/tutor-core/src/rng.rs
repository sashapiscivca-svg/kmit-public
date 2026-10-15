//! Детермінований генератор: однакове насіння дає однакову послідовність на будь-якій платформі.

use rand_chacha::ChaCha8Rng;
use rand_core::{RngCore, SeedableRng};

/// Насіння з пари (id навички, номер варіанта). FNV-1a 64: фіксований алгоритм без залежності від платформи.
pub fn seed_for(skill: &str, variant: u64) -> u64 {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut h = OFFSET;
    for b in skill
        .bytes()
        .chain(std::iter::once(0xff))
        .chain(variant.to_le_bytes())
    {
        h ^= u64::from(b);
        h = h.wrapping_mul(PRIME);
    }
    h
}

pub struct SeededRng(ChaCha8Rng);

impl SeededRng {
    pub fn new(seed: u64) -> Self {
        SeededRng(ChaCha8Rng::seed_from_u64(seed))
    }

    /// Рівномірне ціле з `lo..=hi` (відбраковка, без зміщення). Панікує, якщо `lo > hi`.
    pub fn range(&mut self, lo: i64, hi: i64) -> i64 {
        assert!(lo <= hi, "range: lo > hi");
        let span = (i128::from(hi) - i128::from(lo) + 1) as u64;
        let limit = u64::MAX - (u64::MAX % span);
        loop {
            let x = self.0.next_u64();
            if x < limit {
                return lo + (x % span) as i64;
            }
        }
    }

    /// Рівномірне число з `[0, 1)` для симуляцій (використовує лише цілі операції та одне ділення).
    pub fn unit(&mut self) -> f64 {
        (self.0.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
}
