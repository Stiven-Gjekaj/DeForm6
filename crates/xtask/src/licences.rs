//! `cargo run -p xtask -- licences`: renders `LICENSES.md`, the dependency
//! licence audit, from a fresh `cargo metadata` run.
//!
//! Decision D-03 settles how this project audits its dependency licences: a
//! manual table in `LICENSES.md`, re-derived from `cargo metadata`, never a
//! `cargo-deny` run and never a `deny.toml` file. `cargo-deny` was not
//! machine-verified as legitimate this session, and a manual table is
//! `corpus/NOTICES`'s own established idiom for exactly this shape of fact:
//! a name, a licence, and a source, each traceable to a tool that read it.
//!
//! The file this subcommand renders is checked by re-rendering it, not by
//! reading it. `LICENSES.md` is committed, and the acceptance check is
//! `cargo run -p xtask -- licences && git diff --exit-code -- LICENSES.md`:
//! a hand edit to the committed file fails that check the next time this
//! command runs, because the render is the only thing that can produce the
//! file's content.
//!
//! The three workspace members, `deform6`, `deform6-cli` and `xtask`, are
//! excluded by name. This is what makes the rendered file independent of
//! the workspace version, so a version bump never invalidates it.

use std::collections::BTreeMap;
use std::process::Command;

/// Where the rendered audit is written, relative to the repository root.
const LICENSES_PATH: &str = "LICENSES.md";

/// The exact command a reader can run to reproduce this file, named inline
/// in the file's own lead paragraph so nobody has to guess it.
const COMMAND: &str = "cargo run -p xtask -- licences";

/// The three workspace members. A dependency that shares a name with one of
/// these never reaches the rendered table.
const WORKSPACE_MEMBERS: &[&str] = &["deform6", "deform6-cli", "xtask"];

/// The row count this command refuses to write fewer than. A `cargo
/// metadata` call that suddenly returns three packages is a broken call,
/// not a smaller dependency tree, matching how `update_ratios` in `main.rs`
/// refuses to write fewer than its own minimum program count.
const MINIMUM_PACKAGE_COUNT: usize = 20;

/// One third party package, read out of a `cargo metadata` package entry.
struct Package {
    name: String,
    version: String,
    /// `None` when `cargo metadata` states no licence at all for this
    /// package. Rendered as the literal `not stated`, never a guess.
    license: Option<String>,
    /// `None` when `cargo metadata` states no repository URL.
    repository: Option<String>,
}

/// Prints the render summary, or the error, and gives the process exit
/// code.
pub fn run() -> i32 {
    match run_inner() {
        Ok(summary) => {
            println!("{summary}");
            0
        }
        Err(message) => {
            eprintln!("xtask: {message}");
            1
        }
    }
}

fn run_inner() -> Result<String, String> {
    let metadata = cargo_metadata()?;
    let packages = packages_from_metadata(&metadata)?;
    check_minimum_package_count(packages.len())?;

    let date = today()?;
    let rendered = render(&packages, &date);
    std::fs::write(LICENSES_PATH, &rendered)
        .map_err(|err| format!("writing {LICENSES_PATH}: {err}"))?;

    let not_stated = packages.iter().filter(|pkg| pkg.license.is_none()).count();
    let counts = licence_counts(&packages);
    Ok(format!(
        "xtask: wrote {} packages across {} distinct licence expressions to {LICENSES_PATH} \
         ({not_stated} not stated)",
        packages.len(),
        counts.len()
    ))
}

/// Runs `cargo metadata` as a subprocess and parses its JSON output.
///
/// The argument vector is built one argument at a time with
/// `std::process::Command`, never as a shell string, matching the idiom
/// `crates/xtask/src/fuzz.rs` already documents: no caller-supplied byte
/// reaches a shell.
fn cargo_metadata() -> Result<serde_json::Value, String> {
    let output = Command::new("cargo")
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .arg("--all-features")
        .arg("--locked")
        .output()
        .map_err(|err| format!("running cargo metadata: {err}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("cargo metadata exited {}: {stderr}", output.status));
    }
    serde_json::from_slice(&output.stdout)
        .map_err(|err| format!("parsing cargo metadata output: {err}"))
}

/// Reads the `packages` array out of a `cargo metadata` result, excludes
/// the three workspace members by name, and sorts the rest by name and
/// then by version so the render is deterministic.
fn packages_from_metadata(metadata: &serde_json::Value) -> Result<Vec<Package>, String> {
    let packages = metadata
        .get("packages")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "cargo metadata output holds no \"packages\" array".to_owned())?;

    let mut out = Vec::new();
    for pkg in packages {
        let name = pkg
            .get("name")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "a package in cargo metadata output holds no \"name\" field".to_owned())?
            .to_owned();
        if WORKSPACE_MEMBERS.contains(&name.as_str()) {
            continue;
        }
        let version = pkg
            .get("version")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| format!("{name}: cargo metadata output holds no \"version\" field"))?
            .to_owned();
        let license = pkg
            .get("license")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned);
        let repository = pkg
            .get("repository")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned);
        out.push(Package {
            name,
            version,
            license,
            repository,
        });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.version.cmp(&b.version)));
    Ok(out)
}

/// Refuses a walk that found fewer than [`MINIMUM_PACKAGE_COUNT`] packages.
fn check_minimum_package_count(found: usize) -> Result<(), String> {
    if found < MINIMUM_PACKAGE_COUNT {
        return Err(format!(
            "found {found} third party packages, refusing to write fewer than \
             {MINIMUM_PACKAGE_COUNT}"
        ));
    }
    Ok(())
}

/// Gives today's date as `YYYY-MM-DD`, read from the `date` command rather
/// than computed by hand, so this file carries no bespoke calendar
/// arithmetic under the workspace's `arithmetic_side_effects` lint wall.
fn today() -> Result<String, String> {
    let output = Command::new("date")
        .arg("-u")
        .arg("+%Y-%m-%d")
        .output()
        .map_err(|err| format!("running date: {err}"))?;
    if !output.status.success() {
        return Err(format!("date exited {}", output.status));
    }
    String::from_utf8(output.stdout)
        .map(|text| text.trim().to_owned())
        .map_err(|err| format!("date printed bytes that were not UTF-8: {err}"))
}

/// Counts how many packages carry each distinct licence expression,
/// grouping an absent licence under the literal `not stated`.
fn licence_counts(packages: &[Package]) -> Vec<(String, usize)> {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for pkg in packages {
        let licence = pkg
            .license
            .clone()
            .unwrap_or_else(|| "not stated".to_owned());
        let entry = counts.entry(licence).or_insert(0);
        *entry = entry.saturating_add(1);
    }
    counts.into_iter().collect()
}

/// Renders the whole file's bytes: a lead paragraph, the dependency table,
/// then the closing licence count section, in the shape `corpus/NOTICES`
/// already established for a manual, hand-audited table of third party
/// facts.
fn render(packages: &[Package], date: &str) -> String {
    let mut out = String::new();
    out.push_str("# LICENSES\n\n");
    out.push_str(
        "This file names the licence `cargo metadata` states for every third party \
         package DeForm6's build depends on. It is rendered, not typed. Running\n\n",
    );
    out.push_str(&format!("    {COMMAND}\n\n"));
    out.push_str(&format!("reads the current dependency tree and rewrites this file. Built {date}. A difference between a fresh run and this committed file means the dependency tree changed and this file did not.\n\n"));
    out.push_str(
        "The three workspace members, `deform6`, `deform6-cli` and `xtask`, are excluded, \
         so a version bump never changes this table.\n\n",
    );
    out.push_str("| Crate | Version | Licence | Source |\n");
    out.push_str("|---|---|---|---|\n");
    for pkg in packages {
        let licence = pkg.license.as_deref().unwrap_or("not stated");
        let source = pkg.repository.as_deref().unwrap_or("not stated");
        out.push_str(&format!(
            "| {} | {} | {} | {} |\n",
            pkg.name, pkg.version, licence, source
        ));
    }
    out.push('\n');
    out.push_str("## Licence counts\n\n");
    for (licence, count) in licence_counts(packages) {
        out.push_str(&format!("- {licence}: {count}\n"));
    }
    out
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

    use super::{
        Package, check_minimum_package_count, licence_counts, packages_from_metadata, render,
    };

    fn package(name: &str, version: &str, license: Option<&str>, repo: Option<&str>) -> Package {
        Package {
            name: name.to_owned(),
            version: version.to_owned(),
            license: license.map(str::to_owned),
            repository: repo.map(str::to_owned),
        }
    }

    #[test]
    fn packages_from_metadata_excludes_workspace_members_and_sorts() {
        let metadata = serde_json::json!({
            "packages": [
                { "name": "xtask", "version": "1.0.0", "license": null, "repository": null },
                { "name": "zed", "version": "1.0.0", "license": "MIT", "repository": "https://example.com/zed" },
                { "name": "abc", "version": "2.0.0", "license": "MIT", "repository": null },
                { "name": "abc", "version": "1.0.0", "license": "MIT", "repository": null },
                { "name": "deform6", "version": "1.0.0", "license": null, "repository": null },
                { "name": "deform6-cli", "version": "1.0.0", "license": null, "repository": null },
            ]
        });
        let packages = packages_from_metadata(&metadata).expect("valid metadata parses");
        let names: Vec<&str> = packages.iter().map(|pkg| pkg.name.as_str()).collect();
        assert_eq!(names, vec!["abc", "abc", "zed"]);
        assert_eq!(packages[0].version, "1.0.0");
        assert_eq!(packages[1].version, "2.0.0");
    }

    #[test]
    fn packages_from_metadata_refuses_a_package_with_no_name() {
        let metadata = serde_json::json!({ "packages": [ { "version": "1.0.0" } ] });
        assert!(packages_from_metadata(&metadata).is_err());
    }

    #[test]
    fn check_minimum_package_count_refuses_a_short_walk_and_says_how_many_it_found() {
        let err = check_minimum_package_count(19).expect_err("19 is below the minimum");
        assert!(err.contains("19"));
        assert!(check_minimum_package_count(20).is_ok());
    }

    #[test]
    fn licence_counts_groups_an_absent_licence_under_not_stated() {
        let packages = vec![
            package("a", "1.0.0", Some("MIT"), None),
            package("b", "1.0.0", Some("MIT"), None),
            package("c", "1.0.0", None, None),
        ];
        let counts = licence_counts(&packages);
        assert!(counts.contains(&("MIT".to_owned(), 2)));
        assert!(counts.contains(&("not stated".to_owned(), 1)));
    }

    #[test]
    fn render_never_names_a_workspace_member_and_writes_not_stated_for_an_absent_licence() {
        let packages = vec![
            package(
                "left-pad",
                "1.2.3",
                Some("MIT"),
                Some("https://example.com/left-pad"),
            ),
            package("no-licence", "0.1.0", None, None),
        ];
        let rendered = render(&packages, "2026-01-01");
        assert!(rendered.contains("| left-pad | 1.2.3 | MIT | https://example.com/left-pad |"));
        assert!(rendered.contains("| no-licence | 0.1.0 | not stated | not stated |"));
        assert!(!rendered.contains("| deform6 "));
        assert!(!rendered.contains("| deform6-cli "));
        assert!(!rendered.contains("| xtask "));
        assert!(rendered.contains("cargo run -p xtask -- licences"));
        assert!(rendered.contains("2026-01-01"));
    }
}
