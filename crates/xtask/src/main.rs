//! The developer tools this workspace runs by hand, never shipped.
//!
//! `cargo run -p xtask -- update-ratios` rewrites `tests/ratios.toml`. Any
//! other argument, or none, prints the usage line and exits non-zero: a
//! mistyped subcommand is a loud failure, not a silent success.

fn main() {
    std::process::exit(run(std::env::args().skip(1).collect()));
}

/// Runs one subcommand and gives the process exit code.
///
/// Takes the argument vector directly (not `std::env::args()`) so a test
/// can drive this without spawning a process.
fn run(args: Vec<String>) -> i32 {
    match args.first().map(String::as_str) {
        Some("update-ratios") => update_ratios(),
        Some("--help" | "-h") => {
            println!("{USAGE}");
            0
        }
        Some(other) => {
            eprintln!("xtask: unknown subcommand {other:?}\n{USAGE}");
            1
        }
        None => {
            eprintln!("xtask: missing subcommand\n{USAGE}");
            1
        }
    }
}

const USAGE: &str = "usage: cargo run -p xtask -- update-ratios";

/// Rewrites `tests/ratios.toml`. Not implemented yet: task 3 fills this in.
fn update_ratios() -> i32 {
    eprintln!("xtask: update-ratios is not implemented yet");
    1
}
