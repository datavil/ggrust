//! Kotlin's `Random(seed)` (XorWow, Marsaglia 2003), so a seeded jitter places
//! points exactly where lets-plot does.
//!
//! What it does not do: unseeded randomness. A position without a seed uses
//! [`DEFAULT_SEED`], so renders are reproducible (lets-plot reseeds every render).

/// Seed of jitters that give none.
pub const DEFAULT_SEED: i64 = 0;

/// Kotlin `XorWowRandom`.
pub struct KotlinRandom {
    x: i32,
    y: i32,
    z: i32,
    w: i32,
    v: i32,
    addend: i32,
}

impl KotlinRandom {
    /// Kotlin `Random(seed: Long)`: the low and high 32 bits seed the generator.
    pub fn new(seed: i64) -> Self {
        let (low, high) = (seed as i32, (seed >> 32) as i32);
        let mut random = KotlinRandom {
            x: low,
            y: high,
            z: 0,
            w: 0,
            v: !low,
            addend: (low << 10) ^ ((high as u32 >> 4) as i32),
        };
        for _ in 0..64 {
            random.next_int();
        }
        random
    }

    fn next_int(&mut self) -> i32 {
        let mut t = self.x;
        t ^= (t as u32 >> 2) as i32;
        self.x = self.y;
        self.y = self.z;
        self.z = self.w;
        let v0 = self.v;
        self.w = v0;
        t = (t ^ (t << 1)) ^ v0 ^ (v0 << 4);
        self.v = t;
        self.addend = self.addend.wrapping_add(362_437);
        t.wrapping_add(self.addend)
    }

    /// The upper `count` bits of the next int (Kotlin `nextBits`).
    fn next_bits(&mut self, count: u32) -> i64 {
        i64::from((self.next_int() as u32) >> (32 - count))
    }

    /// Uniform in [0, 1) (Kotlin `nextDouble`: 26 + 27 bits).
    pub fn next_double(&mut self) -> f64 {
        let high = self.next_bits(26);
        let low = self.next_bits(27);
        ((high << 27) + low) as f64 / (1_i64 << 53) as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seeded_and_in_range() {
        // Exact values are checked against lets-plot renders in
        // tests/test_jitter_parity.py (Kotlin's Random runs there).
        let mut random = KotlinRandom::new(42);
        let values: Vec<f64> = (0..3).map(|_| random.next_double()).collect();
        assert!(values.iter().all(|value| (0.0..1.0).contains(value)));
        let mut again = KotlinRandom::new(42);
        assert_eq!(again.next_double(), values[0]);
    }
}
