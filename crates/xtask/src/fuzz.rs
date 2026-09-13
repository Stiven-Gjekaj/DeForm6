//! `fuzz-pr`: the bounded pull request fuzz campaign. `fuzz-cron`: the
//! longer, iteration bounded scheduled campaign. Both wrap
//! `cargo +nightly fuzz run` for the one target `parse` names, so nobody
//! types a `cargo fuzz` call by hand anywhere in this repository or in CI.
//!
//! The fuzz crate lives at `crates/deform6/fuzz`, not at the repository
//! root, so every `cargo fuzz` call in this module names that path with
//! `--fuzz-dir`. A call without it creates a second fuzz directory next to
//! the real one, which is the named risk this module answers.
//!
//! Both commands build their own argument vector and pass it straight to
//! the `cargo` binary. Neither builds a shell string: a byte a caller adds
//! after this command's own flags reaches libFuzzer as one argument, never
//! as text a shell could re-split or re-interpret.

use std::process::Command;

/// The fuzz crate's own directory, relative to the repository root.
///
/// The fuzz crate is not at the repository root, so every `cargo fuzz`
/// call must name this path or the tool creates a second fuzz directory
/// beside it, one nobody meant to make.
pub const FUZZ_DIR: &str = "crates/deform6/fuzz";

/// The name `cargo fuzz init` gave the one target this crate holds, after
/// this plan renamed it from the generated `fuzz_target_1`.
const FUZZ_TARGET_NAME: &str = "parse";

/// The resident set limit, in mebibytes, both campaigns pass explicitly.
///
/// This value happens to match libFuzzer's own default today. It is
/// stated anyway because it is the check for an allocation sized from a
/// length field, a named project constraint and not an incidental number:
/// a future libFuzzer release changing its own default must not change
/// this project's safety bound without a reader seeing it move in a diff.
pub const RSS_LIMIT_MB: u32 = 2048;

/// The wall clock bound, in seconds, the pull request campaign passes.
///
/// Roadmap success criterion 1 fixes this command's own literal command
/// line at sixty seconds, because the thing that command bounds is how
/// long a person waits on a pull request.
pub const PR_MAX_TOTAL_TIME: u32 = 60;

/// The iteration bound the scheduled campaign passes instead of a wall
/// clock bound.
///
/// A wall clock bound is flaky on a varying CI machine: a slow runner
/// covers less ground than a fast one for the same minute. An iteration
/// count is machine independent, so the scheduled campaign takes one.
///
/// The number is chosen against the leak `DETECT_LEAKS` turns off, not
/// picked first and justified after. `crate::error::damaged`'s own doc
/// comment measures the longest message this crate's `Box::leak` call
/// sites can produce at 237 bytes (the control tree walk's own unexplained
/// tail message, `vb/controltree.rs`, with every numeric field at its
/// widest). At `RSS_LIMIT_MB` mebibytes (2048 * 1024 * 1024 =
/// 2_147_483_648 bytes), the leak alone would reach the resident set limit
/// at 2_147_483_648 / 237 = 9_058_166 iterations. `CRON_RUNS` is set at
/// least ten times below that number: 9_058_166 / 500_000 is close to 18,
/// so this run stops with the leak at roughly a nineteenth of the bound it
/// would otherwise reach, leaving room for whatever else the process holds
/// at the same time.
pub const CRON_RUNS: u32 = 500_000;

/// Whether libFuzzer's own leak detector runs. `0` means it does not.
///
/// `crate::error::damaged` leaks one short string per refusal, by design:
/// see that helper's own doc comment for the memory cost this crate
/// accepts rather than widens `Refusal::Damaged` to avoid. A strict refusal
/// on a `Recoverable` defect now reaches it through
/// `crate::error::refusal_for_defect`, so a fuzz run refuses almost every
/// input it generates and would otherwise report this deliberate leak as a
/// crash on nearly every iteration. Turning the detector off is a stated,
/// reasoned choice, recorded as an open finding in `.planning/WINDOWS.md`,
/// not a silent flag: `CRON_RUNS` is the number that keeps the leak this
/// flag stops reporting below `RSS_LIMIT_MB` on its own.
pub const DETECT_LEAKS: u32 = 0;

/// Runs `cargo` with `args`, giving a loud, named failure when the process
/// exits non-zero.
///
/// A non-zero exit from `cargo fuzz run` means the fuzzer found something.
/// That is not this command's own failure: it is the phase working. The
/// message names the artifact directory a person reads next, and the
/// crash to test procedure plan 05-05 writes for turning what is there
/// into a committed regression test.
fn run_cargo(args: &[String]) -> Result<(), String> {
    let status = Command::new("cargo")
        .args(args)
        .status()
        .map_err(|err| format!("running cargo {args:?}: {err}"))?;
    if status.success() {
        return Ok(());
    }
    Err(format!(
        "cargo {args:?} exited {status}. The fuzzer likely found a crashing \
         input. Read the artifact under {FUZZ_DIR}/artifacts/{FUZZ_TARGET_NAME}/ \
         and follow the crash to test procedure plan 05-05 writes."
    ))
}

/// `fuzz-pr`: the bounded pull request campaign.
///
/// `-max_total_time` bounds it by wall clock, per roadmap success
/// criterion 1's own literal command line. `extra` is passed through after
/// this command's own flags, so a caller can add a corpus directory or a
/// seed without editing this file.
pub fn run_pr(extra: &[String]) -> i32 {
    let mut args = vec![
        "+nightly".to_owned(),
        "fuzz".to_owned(),
        "run".to_owned(),
        "--fuzz-dir".to_owned(),
        FUZZ_DIR.to_owned(),
        FUZZ_TARGET_NAME.to_owned(),
        "--".to_owned(),
        format!("-max_total_time={PR_MAX_TOTAL_TIME}"),
        format!("-rss_limit_mb={RSS_LIMIT_MB}"),
        format!("-detect_leaks={DETECT_LEAKS}"),
    ];
    args.extend(extra.iter().cloned());
    match run_cargo(&args) {
        Ok(()) => 0,
        Err(message) => {
            eprintln!("xtask: {message}");
            1
        }
    }
}

/// `fuzz-cron`: the longer, iteration bounded scheduled campaign.
///
/// `-runs` bounds it by iteration count, machine independent unlike a wall
/// clock bound. `extra` is passed through after this command's own flags,
/// so a caller can add a corpus directory or a seed without editing this
/// file.
pub fn run_cron(extra: &[String]) -> i32 {
    let mut args = vec![
        "+nightly".to_owned(),
        "fuzz".to_owned(),
        "run".to_owned(),
        "--fuzz-dir".to_owned(),
        FUZZ_DIR.to_owned(),
        FUZZ_TARGET_NAME.to_owned(),
        "--".to_owned(),
        format!("-runs={CRON_RUNS}"),
        format!("-rss_limit_mb={RSS_LIMIT_MB}"),
        format!("-detect_leaks={DETECT_LEAKS}"),
    ];
    args.extend(extra.iter().cloned());
    match run_cargo(&args) {
        Ok(()) => 0,
        Err(message) => {
            eprintln!("xtask: {message}");
            1
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing,
        clippy::arithmetic_side_effects,
        clippy::integer_division,
        reason = "a test builds the state it needs and must fail loudly when that state is wrong"
    )]

    use super::{CRON_RUNS, DETECT_LEAKS, FUZZ_DIR, PR_MAX_TOTAL_TIME, RSS_LIMIT_MB};

    /// `CRON_RUNS` must keep the leak `DETECT_LEAKS` stops reporting below
    /// `RSS_LIMIT_MB` on its own, using the same 237 byte measured
    /// worst case message `CRON_RUNS`'s own doc comment names, with a
    /// safety factor of at least ten.
    #[test]
    fn cron_runs_keeps_the_leak_at_least_ten_times_below_the_resident_set_limit() {
        const LONGEST_LEAKED_MESSAGE_BYTES: u64 = 237;
        let limit_bytes = u64::from(RSS_LIMIT_MB) * 1024 * 1024;
        let iterations_to_reach_limit = limit_bytes / LONGEST_LEAKED_MESSAGE_BYTES;
        assert!(
            u64::from(CRON_RUNS) * 10 <= iterations_to_reach_limit,
            "CRON_RUNS ({CRON_RUNS}) must sit at least ten times below the iteration \
             count ({iterations_to_reach_limit}) at which the leak alone would reach \
             the resident set limit"
        );
    }

    #[test]
    fn detect_leaks_is_off() {
        assert_eq!(DETECT_LEAKS, 0);
    }

    #[test]
    fn pr_max_total_time_matches_roadmap_success_criterion_one() {
        assert_eq!(PR_MAX_TOTAL_TIME, 60);
    }

    #[test]
    fn fuzz_dir_names_the_real_generated_crate() {
        assert_eq!(FUZZ_DIR, "crates/deform6/fuzz");
        // `cargo test` runs this binary with this crate's own manifest
        // directory as the working directory, not the workspace root, so
        // `FUZZ_DIR` is resolved against `CARGO_MANIFEST_DIR` here rather
        // than read as a path relative to the process's own cwd.
        let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        assert!(workspace_root.join(FUZZ_DIR).join("Cargo.toml").is_file());
    }
}
