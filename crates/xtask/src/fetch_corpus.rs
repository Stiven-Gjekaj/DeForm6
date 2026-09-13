//! `fetch-corpus`: fetches every entry `corpus/manifest.toml` pins,
//! verifies each one against its own pinned SHA-256, and writes it under
//! `corpus/fetched/`. `pin-corpus`: fetches one address once, hashes it,
//! and appends the entry to the manifest, so a human never computes a
//! SHA-256 by hand.
//!
//! Per `AGENTS.md`'s "What may enter this repository": the run time
//! robustness set carries no licence that permits redistribution, so its
//! bytes are fetched here rather than committed. `corpus/manifest.toml`
//! pins the address and the hash for each program, and is committed;
//! `.gitignore` already excludes `corpus/fetched/`, the directory this
//! module writes into.
//!
//! The named risk this module answers, stated in `05-RESEARCH.md` and
//! `05-07-PLAN.md` alike: a fetch loop that skips one failed entry and
//! keeps going turns the robustness set into a set of zero files that
//! passes every check. This module never skips an entry. Every failure,
//! a fetch that did not complete, a hash that does not match, or a write
//! that failed, propagates out of the loop with `?` and fails the whole
//! command, naming the entry key.

use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

/// One program's pinned entry: the address to fetch it from and the
/// SHA-256 the fetched bytes must match.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Entry {
    url: String,
    sha256: String,
}

/// The fewest entries `corpus/manifest.toml` may declare, checked before
/// the first fetch starts.
///
/// A manifest with zero entries passes every check in this repository
/// and proves nothing about hostile input handling. That silence is the
/// named risk this module exists to close: a fetch that skips a failure
/// turns the run time robustness set into a set of zero files that
/// passes. A human raises this number as the set grows.
pub const MINIMUM_MANIFEST_ENTRIES: usize = 1;

/// The workspace root, resolved from this crate's own manifest directory
/// rather than the process's current directory, matching the convention
/// `crates/deform6/tests/ratios.rs`'s own `corpus_root`/`ratios_toml_path`
/// already use.
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn manifest_path() -> PathBuf {
    workspace_root().join("corpus/manifest.toml")
}

fn fetched_dir() -> PathBuf {
    workspace_root().join("corpus/fetched")
}

/// Refuses a manifest key holding a path separator or a parent directory
/// component, before any fetch and before any write. Both `fetch-corpus`
/// (reading an entry) and `pin-corpus` (adding one) call this before
/// touching the network.
fn refuse_unsafe_key(key: &str) -> Result<(), String> {
    if key.is_empty() {
        return Err("a manifest key names no program".to_owned());
    }
    if key.contains('/') || key.contains('\\') {
        return Err(format!("{key}: a manifest key must hold no path separator"));
    }
    if key == "." || key == ".." {
        return Err(format!(
            "{key}: a manifest key must hold no parent directory component"
        ));
    }
    Ok(())
}

/// True when `value` is exactly 64 lower case hexadecimal characters, the
/// shape a SHA-256 digest renders to.
fn is_valid_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
}

/// Collapses `.` and `..` components lexically, without touching the
/// file system: the same shape `deform6-cli`'s own `plan_writes`
/// containment check uses for a path that may not exist yet.
fn lexically_normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// Builds the on disk destination for `key`, checked to be a direct
/// child of `resolved_fetched_dir`. Never takes a file name from the
/// network: the manifest key is the only input this function reads.
fn destination_path(key: &str, resolved_fetched_dir: &Path) -> Result<PathBuf, String> {
    refuse_unsafe_key(key)?;
    let candidate = lexically_normalize(&resolved_fetched_dir.join(key));
    let Some(parent) = candidate.parent() else {
        return Err(format!("{key}: names a path with no parent directory"));
    };
    if parent != resolved_fetched_dir {
        return Err(format!(
            "{key}: would write outside {}, at {}",
            resolved_fetched_dir.display(),
            candidate.display()
        ));
    }
    Ok(candidate)
}

/// Parses `text` as `corpus/manifest.toml`'s own format and validates
/// every entry before returning any of them: a manifest that is half
/// valid is a manifest nobody has read. Refuses on the first bad entry,
/// naming its key.
fn parse_manifest(text: &str) -> Result<BTreeMap<String, Entry>, String> {
    let table: toml::Table = text
        .parse()
        .map_err(|err| format!("corpus/manifest.toml is not valid TOML: {err}"))?;
    let mut entries = BTreeMap::new();
    for (key, value) in &table {
        refuse_unsafe_key(key)?;
        let fields = value
            .as_table()
            .ok_or_else(|| format!("{key}: is not a table"))?;
        let url = fields
            .get("url")
            .and_then(toml::Value::as_str)
            .ok_or_else(|| format!("{key}: holds no url"))?;
        if !url.starts_with("https://") {
            return Err(format!("{key}: the address {url} is not an https address"));
        }
        let sha256 = fields
            .get("sha256")
            .and_then(toml::Value::as_str)
            .ok_or_else(|| format!("{key}: holds no sha256"))?;
        if !is_valid_sha256_hex(sha256) {
            return Err(format!(
                "{key}: the sha256 value must be 64 lower case hexadecimal characters"
            ));
        }
        entries.insert(
            key.clone(),
            Entry {
                url: url.to_owned(),
                sha256: sha256.to_owned(),
            },
        );
    }
    Ok(entries)
}

/// Refuses a manifest declaring fewer than [`MINIMUM_MANIFEST_ENTRIES`],
/// checked before the first fetch starts.
fn check_minimum_manifest_entries(found: usize) -> Result<(), String> {
    if found < MINIMUM_MANIFEST_ENTRIES {
        return Err(format!(
            "the manifest declares {found} entries, refusing to fetch fewer than {MINIMUM_MANIFEST_ENTRIES}"
        ));
    }
    Ok(())
}

/// Renders `bytes` as a lower case hexadecimal string, one `{:02x}` pair
/// per byte.
fn to_lower_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Computes the lower case hexadecimal SHA-256 digest of `bytes`. Never
/// the standard library's own hashing trait: it is documented as
/// unstable across compiler versions and it is not a cryptographic
/// hash, so it proves nothing about the bytes it hashes.
fn compute_sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    to_lower_hex(&hasher.finalize())
}

/// Compares `bytes`' own SHA-256 against `expected_hex`, giving the
/// computed digest back on a match. The one place the hash check lives,
/// reachable with no network at all: [`fetch_one`] calls this with the
/// bytes it received, and every test proving the check drives this
/// function directly.
fn verify_hash(bytes: &[u8], expected_hex: &str) -> Result<String, String> {
    let computed = compute_sha256_hex(bytes);
    if computed == expected_hex {
        Ok(computed)
    } else {
        Err(format!(
            "expected sha256 {expected_hex}, computed {computed}"
        ))
    }
}

/// Fetches `url` and gives its whole body. The one place this module
/// touches the network; every other function here takes bytes it was
/// already given, so a test can drive them without reaching the
/// network at all.
fn fetch_bytes(url: &str) -> Result<Vec<u8>, String> {
    let mut response = ureq::get(url).call().map_err(|err| err.to_string())?;
    response
        .body_mut()
        .read_to_vec()
        .map_err(|err| err.to_string())
}

/// Fetches, hashes and writes one entry. Every failure here propagates
/// out with `?`; the caller, [`fetch_all`], never catches one and keeps
/// going past it, because a set of zero files must never report as a
/// success.
fn fetch_one(key: &str, entry: &Entry, resolved_fetched_dir: &Path) -> Result<(), String> {
    let destination = destination_path(key, resolved_fetched_dir)?;
    let bytes =
        fetch_bytes(&entry.url).map_err(|err| format!("{key}: fetching {}: {err}", entry.url))?;
    verify_hash(&bytes, &entry.sha256).map_err(|err| format!("{key}: {err}"))?;
    std::fs::write(&destination, &bytes)
        .map_err(|err| format!("{key}: writing {}: {err}", destination.display()))?;
    Ok(())
}

/// Creates `dir` if it does not exist yet, then resolves it to an
/// absolute, canonicalized path: the shape `destination_path`'s own
/// containment check compares against.
fn resolve_fetched_dir(dir: &Path) -> Result<PathBuf, String> {
    std::fs::create_dir_all(dir).map_err(|err| format!("creating {}: {err}", dir.display()))?;
    std::fs::canonicalize(dir).map_err(|err| format!("resolving {}: {err}", dir.display()))
}

/// Fetches and verifies every entry `corpus/manifest.toml` declares,
/// giving back the count fetched. That count always equals the
/// manifest's own entry count on success: the loop never returns early
/// with some entries skipped, and it never counts whatever files happen
/// to already sit in `corpus/fetched/`.
fn fetch_all() -> Result<usize, String> {
    let path = manifest_path();
    let text = std::fs::read_to_string(&path)
        .map_err(|err| format!("reading {}: {err}", path.display()))?;
    let entries = parse_manifest(&text)?;
    check_minimum_manifest_entries(entries.len())?;

    let resolved_fetched_dir = resolve_fetched_dir(&fetched_dir())?;
    for (key, entry) in &entries {
        fetch_one(key, entry, &resolved_fetched_dir)?;
    }

    Ok(entries.len())
}

/// Refuses `url` when it is not an https address, before any fetch.
fn refuse_insecure_url(url: &str) -> Result<(), String> {
    if url.starts_with("https://") {
        Ok(())
    } else {
        Err(format!("{url} is not an https address"))
    }
}

/// Gives the lines of `text` that come before the first table header,
/// unchanged: `pin-corpus` preserves this text byte for byte across a
/// rewrite, rather than reformatting or dropping it.
fn split_header(text: &str) -> String {
    let mut header = String::new();
    for line in text.lines() {
        if line.trim_start().starts_with('[') {
            break;
        }
        header.push_str(line);
        header.push('\n');
    }
    header
}

/// Renders one manifest entry as a quoted `[key]` table header followed
/// by its two fields, using `toml::Value`'s own `Display` for correct
/// TOML string escaping.
fn render_entry(key: &str, entry: &Entry) -> String {
    format!(
        "[{}]\nurl = {}\nsha256 = {}\n",
        toml::Value::String(key.to_owned()),
        toml::Value::String(entry.url.clone()),
        toml::Value::String(entry.sha256.clone())
    )
}

/// Renders `header`, unchanged, followed by every entry in `entries`,
/// sorted by key: the whole manifest file, rebuilt from structured data
/// rather than grown by appending text to the end.
fn render_manifest(header: &str, entries: &BTreeMap<String, Entry>) -> String {
    let mut out = header.to_owned();
    for (key, entry) in entries {
        out.push('\n');
        out.push_str(&render_entry(key, entry));
    }
    out
}

/// Parses `text`, refuses an unsafe or duplicate `name`, adds `name`
/// pinned to `url`/`sha256`, and renders the whole file back, keeping
/// whatever text came before the first table header unchanged. Touches
/// neither the network nor the file system, so a test drives it
/// directly with a manifest built inside the test.
fn add_entry(text: &str, name: &str, url: &str, sha256: &str) -> Result<String, String> {
    refuse_unsafe_key(name)?;
    let mut entries = parse_manifest(text)?;
    if let Some(existing) = entries.get(name) {
        return Err(format!(
            "{name} is already pinned to {}, refusing to overwrite",
            existing.url
        ));
    }
    entries.insert(
        name.to_owned(),
        Entry {
            url: url.to_owned(),
            sha256: sha256.to_owned(),
        },
    );
    let header = split_header(text);
    let rendered = render_manifest(&header, &entries);
    rendered
        .parse::<toml::Table>()
        .map(|_| ())
        .map_err(|err| format!("the manifest this command rendered is not valid TOML: {err}"))?;
    Ok(rendered)
}

/// Fetches `url` once, hashes the bytes, and pins `name` to that hash in
/// `corpus/manifest.toml`. Never writes the fetched bytes anywhere:
/// pinning is not fetching, so a person can pin an entry from a machine
/// that has no room for the file.
fn pin_inner(name: &str, url: &str) -> Result<String, String> {
    refuse_unsafe_key(name)?;
    refuse_insecure_url(url)?;

    let path = manifest_path();
    let text = std::fs::read_to_string(&path)
        .map_err(|err| format!("reading {}: {err}", path.display()))?;

    let existing_entries = parse_manifest(&text)?;
    if let Some(existing) = existing_entries.get(name) {
        return Err(format!(
            "{name} is already pinned to {}, refusing to overwrite",
            existing.url
        ));
    }

    let bytes = fetch_bytes(url).map_err(|err| format!("{name}: fetching {url}: {err}"))?;
    let hash = compute_sha256_hex(&bytes);

    let rendered = add_entry(&text, name, url, &hash)?;
    std::fs::write(&path, &rendered).map_err(|err| format!("writing {}: {err}", path.display()))?;

    Ok(hash)
}

/// Runs `pin-corpus <name> <url>`.
pub fn run_pin(args: &[String]) -> i32 {
    match args {
        [name, url] => match pin_inner(name, url) {
            Ok(hash) => {
                println!("{name}");
                println!("{hash}");
                0
            }
            Err(message) => {
                eprintln!("xtask: {message}");
                1
            }
        },
        _ => {
            eprintln!("xtask: usage: cargo run -p xtask -- pin-corpus <name> <url>");
            1
        }
    }
}

/// Runs `fetch-corpus`. Takes no arguments.
pub fn run(_args: &[String]) -> i32 {
    match fetch_all() {
        Ok(count) => {
            println!("xtask: fetched and verified {count} entries");
            0
        }
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

    use super::{
        MINIMUM_MANIFEST_ENTRIES, add_entry, check_minimum_manifest_entries, compute_sha256_hex,
        destination_path, is_valid_sha256_hex, parse_manifest, refuse_insecure_url, verify_hash,
    };
    use std::path::Path;

    /// The known SHA-256 digest of the three bytes `b"abc"`, a standard
    /// NIST test vector, used across these tests so no test transcribes
    /// its own 64 character hash by hand.
    const KNOWN_SHA256_ABC: &str =
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

    #[test]
    fn known_sha256_abc_is_64_characters() {
        assert_eq!(KNOWN_SHA256_ABC.len(), 64);
    }

    #[test]
    fn compute_sha256_hex_matches_the_known_test_vector() {
        assert_eq!(compute_sha256_hex(b"abc"), KNOWN_SHA256_ABC);
    }

    #[test]
    fn verify_hash_accepts_a_match_and_refuses_a_mismatch() {
        assert!(verify_hash(b"abc", KNOWN_SHA256_ABC).is_ok());
        let wrong = "0".repeat(64);
        let err = verify_hash(b"abc", &wrong).expect_err("a wrong hash must refuse");
        assert!(err.contains(KNOWN_SHA256_ABC), "{err}");
        assert!(err.contains(&wrong), "{err}");
    }

    #[test]
    fn is_valid_sha256_hex_accepts_the_known_vector_and_refuses_upper_case() {
        assert!(is_valid_sha256_hex(KNOWN_SHA256_ABC));
        let upper = format!("A{}", &KNOWN_SHA256_ABC[1..]);
        assert!(!is_valid_sha256_hex(&upper));
    }

    #[test]
    fn a_well_formed_manifest_parses() {
        let text = format!(
            "[\"a-program\"]\nurl = \"https://example.invalid/a.exe\"\nsha256 = \"{KNOWN_SHA256_ABC}\"\n"
        );
        let entries = parse_manifest(&text).expect("a well formed manifest must parse");
        assert_eq!(entries.len(), 1);
        let entry = entries.get("a-program").expect("the entry must be present");
        assert_eq!(entry.url, "https://example.invalid/a.exe");
        assert_eq!(entry.sha256, KNOWN_SHA256_ABC);
    }

    #[test]
    fn a_missing_address_refuses() {
        let text = format!("[\"a-program\"]\nsha256 = \"{KNOWN_SHA256_ABC}\"\n");
        assert!(parse_manifest(&text).is_err());
    }

    #[test]
    fn a_missing_hash_refuses() {
        let text = "[\"a-program\"]\nurl = \"https://example.invalid/a.exe\"\n".to_owned();
        assert!(parse_manifest(&text).is_err());
    }

    #[test]
    fn a_hash_of_the_wrong_length_refuses() {
        let short_hash = "a".repeat(63);
        let text = format!(
            "[\"a-program\"]\nurl = \"https://example.invalid/a.exe\"\nsha256 = \"{short_hash}\"\n"
        );
        assert!(parse_manifest(&text).is_err());
    }

    #[test]
    fn a_hash_with_an_upper_case_letter_refuses() {
        let upper_hash = format!("A{}", &KNOWN_SHA256_ABC[1..]);
        let text = format!(
            "[\"a-program\"]\nurl = \"https://example.invalid/a.exe\"\nsha256 = \"{upper_hash}\"\n"
        );
        assert!(parse_manifest(&text).is_err());
    }

    #[test]
    fn an_insecure_address_refuses() {
        let text = format!(
            "[\"a-program\"]\nurl = \"http://example.invalid/a.exe\"\nsha256 = \"{KNOWN_SHA256_ABC}\"\n"
        );
        assert!(parse_manifest(&text).is_err());
    }

    #[test]
    fn a_key_holding_a_path_separator_refuses() {
        let text = format!(
            "[\"a/program\"]\nurl = \"https://example.invalid/a.exe\"\nsha256 = \"{KNOWN_SHA256_ABC}\"\n"
        );
        assert!(parse_manifest(&text).is_err());
    }

    #[test]
    fn a_key_holding_a_parent_directory_component_refuses() {
        let text = format!(
            "[\"..\"]\nurl = \"https://example.invalid/a.exe\"\nsha256 = \"{KNOWN_SHA256_ABC}\"\n"
        );
        assert!(parse_manifest(&text).is_err());
    }

    #[test]
    fn an_empty_manifest_refuses_with_the_minimum_entry_message() {
        let entries = parse_manifest("# just a header comment\n")
            .expect("comments alone still parse as zero entries");
        assert!(entries.is_empty());
        let err = check_minimum_manifest_entries(entries.len()).expect_err("zero must refuse");
        assert!(err.contains('0'), "{err}");
        assert!(err.contains(&MINIMUM_MANIFEST_ENTRIES.to_string()), "{err}");
    }

    #[test]
    fn destination_path_accepts_a_plain_key() {
        let resolved_dir = Path::new("/tmp/deform6-fetch-corpus-test");
        let result = destination_path("a-program", resolved_dir)
            .expect("a plain key must resolve inside the directory");
        assert_eq!(result, resolved_dir.join("a-program"));
    }

    #[test]
    fn destination_path_refuses_a_key_with_a_path_separator() {
        let resolved_dir = Path::new("/tmp/deform6-fetch-corpus-test");
        assert!(destination_path("a/program", resolved_dir).is_err());
    }

    #[test]
    fn destination_path_refuses_a_parent_directory_component() {
        let resolved_dir = Path::new("/tmp/deform6-fetch-corpus-test");
        assert!(destination_path("..", resolved_dir).is_err());
    }

    #[test]
    fn refuse_insecure_url_refuses_a_non_https_address() {
        assert!(refuse_insecure_url("http://example.invalid/a.exe").is_err());
        assert!(refuse_insecure_url("https://example.invalid/a.exe").is_ok());
    }

    #[test]
    fn add_entry_appends_to_an_empty_manifest_and_preserves_the_header() {
        let text = "# a header comment\n# a second header line\n";
        let rendered = add_entry(
            text,
            "a-program",
            "https://example.invalid/a.exe",
            KNOWN_SHA256_ABC,
        )
        .expect("adding to an empty manifest must succeed");
        assert!(rendered.starts_with(text), "{rendered}");
        let entries = parse_manifest(&rendered).expect("the rendered manifest must parse");
        assert_eq!(entries.len(), 1);
        let entry = entries
            .get("a-program")
            .expect("the new entry must be present");
        assert_eq!(entry.url, "https://example.invalid/a.exe");
        assert_eq!(entry.sha256, KNOWN_SHA256_ABC);
    }

    #[test]
    fn add_entry_adds_a_second_entry_and_keeps_the_first() {
        let text = format!(
            "# header\n\n[\"a-program\"]\nurl = \"https://example.invalid/a.exe\"\nsha256 = \"{KNOWN_SHA256_ABC}\"\n"
        );
        let other_hash = "b".repeat(64);
        let rendered = add_entry(
            &text,
            "b-program",
            "https://example.invalid/b.exe",
            &other_hash,
        )
        .expect("adding a second entry must succeed");
        let entries = parse_manifest(&rendered).expect("the rendered manifest must parse");
        assert_eq!(entries.len(), 2);
        assert!(entries.contains_key("a-program"));
        assert!(entries.contains_key("b-program"));
    }

    #[test]
    fn add_entry_refuses_a_duplicate_name() {
        let text = format!(
            "[\"a-program\"]\nurl = \"https://example.invalid/a.exe\"\nsha256 = \"{KNOWN_SHA256_ABC}\"\n"
        );
        let err = add_entry(
            &text,
            "a-program",
            "https://example.invalid/other.exe",
            KNOWN_SHA256_ABC,
        )
        .expect_err("a duplicate name must refuse");
        assert!(err.contains("a-program"), "{err}");
    }

    #[test]
    fn add_entry_refuses_a_name_with_a_path_separator() {
        assert!(
            add_entry(
                "",
                "a/program",
                "https://example.invalid/a.exe",
                KNOWN_SHA256_ABC
            )
            .is_err()
        );
    }

    #[test]
    fn add_entry_refuses_a_name_with_a_parent_directory_component() {
        assert!(add_entry("", "..", "https://example.invalid/a.exe", KNOWN_SHA256_ABC).is_err());
    }
}
