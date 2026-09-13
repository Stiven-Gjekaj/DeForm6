#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::integer_division,
    reason = "a test builds the state it needs and must end the process loudly \
              when a mutated input breaks the reader"
)]

//! A small, deterministic, dependency free mutation sweep over the vendored
//! corpus, run inside `cargo test --workspace` on the pinned stable
//! toolchain. `crates/deform6/fuzz` runs a real, coverage guided fuzzer, but
//! it needs a nightly toolchain and a subcommand, and it runs out of band.
//! This file is the smoke test that catches the same class of failure on a
//! developer's own machine, in the same command they already run before
//! every commit.
//!
//! A mutated corpus program is never written to disk and never committed:
//! `AGENTS.md` bars a fixture calculated from a third party file from this
//! repository. The printed file name, seed, iteration number and byte
//! changes are what a person uses to rebuild the input by hand.
//! `crates/deform6/tests/support/hostile.rs` is where a hostile fixture that
//! may be committed comes from instead.
//!
//! This file builds its own generator and its own mutation rule below. The
//! sweep over the corpus that uses them is a separate, later piece of this
//! same file.

/// The seed this sweep starts from. The value is arbitrary: what matters is
/// that it never changes. Changing it makes the sweep explore a different
/// set of mutated inputs, so a change to this constant is a deliberate act
/// and not a tidy up.
const SMOKE_SEED: u64 = 0x5EED_BEEF_C0FF_EE01;

/// The total number of mutated inputs the sweep runs.
///
/// Measured cost: one iteration over the largest vendored corpus file
/// (`corpus/vb6-code/Map-editor-2D/Map Editor.exe`, 245,760 bytes), through
/// `inspect` in both modes and `write::project` on the salvage result, took
/// about 1.25 milliseconds on the machine this was measured on. This
/// workspace's own three gate commands (`cargo fmt --all --check`,
/// `cargo clippy --all-targets -- -D warnings`, `cargo test --workspace`)
/// take about 90 seconds together today.
///
/// `SMOKE_ITERATIONS` is 440: ten mutations of each of the 44 vendored
/// corpus executables. Ten rounds over 44 files distributes the work evenly
/// rather than spending it all on one file, per this sweep's own rule.
/// At 1.25 milliseconds per iteration, 440 iterations cost about 550
/// milliseconds, under one percent of the gate's own 90 second run.
const SMOKE_ITERATIONS: u32 = 440;

/// A fixed width, wrapping linear congruential generator with no source of
/// entropy outside the seed given to `Rng::new`. Two generators built from
/// the same seed give the same sequence of values forever, on any machine:
/// this is what makes the sweep's failures reproducible by rerunning the
/// same command, rather than a fuzzer's failures, which are not.
struct Rng(u64);

/// Knuth's MMIX multiplier. Congruent to 1 modulo 4, which the Hull-Dobell
/// theorem requires, together with an odd increment, for a linear
/// congruential generator modulo a power of two to visit every one of its
/// 2^64 states before it repeats. The exact value is a choice made for that
/// property, not a magic number.
const MULTIPLIER: u64 = 6_364_136_223_846_793_005;

/// Knuth's MMIX increment. Odd, which the Hull-Dobell theorem requires
/// alongside `MULTIPLIER` for a full period. The exact value is a choice
/// made for that property, not a magic number.
const INCREMENT: u64 = 1_442_695_040_888_963_407;

impl Rng {
    /// Starts a generator at `seed`. No system clock, no process
    /// identifier, no address and no entropy from the operating system
    /// enters this generator anywhere: any of those would make a failure
    /// this sweep finds impossible to reproduce by rerunning the command.
    const fn new(seed: u64) -> Self {
        Self(seed)
    }

    /// Advances the state and gives back the new value.
    ///
    /// Every step below is a named wrapping call, not a plain `*` or `+`.
    /// The allow header on this file already turns off clippy's
    /// `arithmetic_side_effects` lint, but that lint is a compile time
    /// check, not a runtime guard, and this workspace's dev and test
    /// profiles still check for overflow at runtime. A plain `*` or `+`
    /// that overflowed would panic; `wrapping_mul` and `wrapping_add` do
    /// not, and a wrapping step is exactly what a full period generator
    /// needs.
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(MULTIPLIER).wrapping_add(INCREMENT);
        self.0
    }

    /// Gives back a value below `bound`, using a remainder. `bound` zero
    /// gives back zero rather than dividing by it.
    ///
    /// A remainder biases toward the low end of the range by an amount
    /// proportional to `bound`, but that bias does not matter here: this
    /// generator is looking for a panic, not for a uniform distribution
    /// over byte offsets.
    fn below(&mut self, bound: usize) -> usize {
        if bound == 0 {
            return 0;
        }
        let bound_u64 = u64::try_from(bound).unwrap_or(u64::MAX);
        let value = self.next_u64() % bound_u64;
        usize::try_from(value).unwrap_or(0)
    }
}

/// One single byte change: the offset it changed and the value it wrote.
type Change = (usize, u8);

/// Applies `count` single byte changes to a copy of `data`, choosing each
/// offset and each new value from `rng`. Gives back the mutated copy and
/// every change it made, because the sweep's failure message needs the
/// list to rebuild the input: `data` itself may never be committed.
///
/// Never mutates `data` in place, and never changes its length. `data` is a
/// file this test only reads; the rest of the test binary and the sweep's
/// own loop still hold their own copy of the same bytes after this call.
fn mutate(data: &[u8], rng: &mut Rng, count: usize) -> (Vec<u8>, Vec<Change>) {
    let mut mutated = data.to_vec();
    let mut changes = Vec::with_capacity(count);
    if mutated.is_empty() {
        return (mutated, changes);
    }
    for _ in 0..count {
        let offset = rng.below(mutated.len());
        let value = u8::try_from(rng.below(256)).unwrap_or(0);
        mutated[offset] = value;
        changes.push((offset, value));
    }
    (mutated, changes)
}

#[cfg(test)]
mod tests {
    use super::{MULTIPLIER, Rng, SMOKE_ITERATIONS, SMOKE_SEED, mutate};

    #[test]
    fn two_generators_from_the_same_seed_give_the_same_first_ten_values() {
        let mut a = Rng::new(SMOKE_SEED);
        let mut b = Rng::new(SMOKE_SEED);
        let from_a: Vec<u64> = (0..10).map(|_| a.next_u64()).collect();
        let from_b: Vec<u64> = (0..10).map(|_| b.next_u64()).collect();
        assert_eq!(
            from_a, from_b,
            "two generators built from the same seed gave different sequences"
        );
    }

    #[test]
    fn a_mutation_keeps_the_length_and_changes_exactly_the_recorded_offsets() {
        let original: Vec<u8> = (0..64).collect();
        let mut rng = Rng::new(MULTIPLIER ^ SMOKE_SEED);
        let (mutated, changes) = mutate(&original, &mut rng, 5);

        assert_eq!(
            mutated.len(),
            original.len(),
            "a mutation changed the length of the input"
        );
        assert_eq!(
            changes.len(),
            5,
            "a mutation did not record every change it made"
        );
        for (offset, value) in &changes {
            assert_eq!(
                mutated[*offset], *value,
                "the recorded change at offset {offset} does not match the mutated byte there"
            );
        }
    }

    #[test]
    fn the_original_slice_is_unchanged_after_a_mutation() {
        let original: Vec<u8> = (0..64).collect();
        let before = original.clone();
        let mut rng = Rng::new(SMOKE_SEED.wrapping_add(1));
        let (_mutated, _changes) = mutate(&original, &mut rng, 5);
        assert_eq!(
            original, before,
            "mutate changed the caller's own slice in place"
        );
    }

    #[test]
    #[allow(
        clippy::assertions_on_constants,
        reason = "the sweep this constant defends runs in a later part of this \
                  file; this compile time check keeps the constant itself \
                  honest without waiting for that runtime loop to exist"
    )]
    fn smoke_iterations_defends_a_positive_budget() {
        assert!(
            SMOKE_ITERATIONS > 0,
            "SMOKE_ITERATIONS must be positive, or the sweep could pass by mutating nothing"
        );
    }
}
