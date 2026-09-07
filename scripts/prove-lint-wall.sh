#!/bin/sh
#
# Prove that the lint wall rejects the bad shapes.
#
# The wall lives in the two [workspace.lints] tables in the root Cargo.toml.
# A wall that is configured is not the same thing as a wall that works. This
# script writes a probe that holds one bad shape on each line, runs clippy on
# it, and checks that each line is stopped by the lint that must stop it. Then
# it removes the probe and runs the three gate commands, so it proves both that
# the wall fires and that the tree it leaves behind is clean.
#
# The probe is an example target. `cargo clippy --all-targets` compiles
# `crates/*/examples/*.rs` under the full wall, an example is found by its path
# and needs no `mod` line, and the probe therefore never enters the library.
#
# Run it from anywhere:
#
#     sh scripts/prove-lint-wall.sh
#
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$ROOT"

PROBE=crates/deform6/examples/lint_wall_probe.rs
WORK=$(mktemp -d)

cleanup() {
	rm -f "$PROBE"
	rmdir crates/deform6/examples 2>/dev/null || true
	rm -rf "$WORK"
}
trap cleanup EXIT

if [ -e "$PROBE" ]; then
	echo "ERROR: $PROBE is already present. This script owns that path and"
	echo "       removes it. Move the file away and run the script again."
	exit 1
fi

if ! command -v jq >/dev/null 2>&1; then
	echo "ERROR: this script needs jq to read the clippy diagnostics."
	echo "       The short message format prints the message of a lint but"
	echo "       not its name, and the default format names a lint once only,"
	echo "       so neither can say which line each lint caught."
	exit 1
fi

# The eight bad lines. Each field is the shape, the lint that must stop it, and
# what the line holds. The script finds the line number of each shape in the
# probe by its marker comment, so a later edit of the probe does not break the
# table.
EXPECTED='index|clippy::indexing_slicing|an index of a slice with square brackets
slice|clippy::indexing_slicing|a range slice with square brackets
add|clippy::arithmetic_side_effects|an addition with the plus operator
unwrap|clippy::unwrap_used|a call to unwrap on an Option
truncate|clippy::cast_possible_truncation|a cast from u64 to u32
sign|clippy::cast_sign_loss|a cast from i32 to u32
divide|clippy::integer_division|an integer division
unsafe|unsafe_code|an unsafe block'

mkdir -p crates/deform6/examples
cat >"$PROBE" <<'PROBE_EOF'
//! A probe that must not compile.
//!
//! `scripts/prove-lint-wall.sh` writes this file, reads the errors, and then
//! removes it. Do not commit it.

fn probe(d: &[u8], a: u64, b: u64, c: i32) {
    let _index = d[0]; // shape: index
    let _slice = &d[4..8]; // shape: slice
    let _add = a + b; // shape: add
    let _unwrap = d.first().unwrap(); // shape: unwrap
    let _truncate = a as u32; // shape: truncate
    let _sign = c as u32; // shape: sign
    let _divide = a / b; // shape: divide
    let _unsafe = unsafe { core::ptr::read(&a) }; // shape: unsafe
}

fn main() {
    probe(&[], 0, 0, 0);
}
PROBE_EOF

echo "The probe is at $PROBE. Running clippy on it."
echo

# clippy exits non-zero here, and that is the point. Keep the status away from
# `set -e`. The json format goes to stdout and the progress lines go to stderr,
# so the two never mix.
cargo clippy --all-targets --message-format json \
	>"$WORK/raw.json" 2>"$WORK/cargo.err" || true

# One "line number|lint name" pair for each diagnostic that names a lint and
# points at a line. `is_primary` picks the span of the offending line and drops
# the spans of the help text.
jq -r 'select(.reason == "compiler-message")
       | .message as $m
       | ($m.code.code // empty) as $code
       | ($m.spans[] | select(.is_primary) | .line_start) as $line
       | "\($line)|\($code)"' \
	<"$WORK/raw.json" | sort -u >"$WORK/observed"

fail=0
checked=0

while IFS='|' read -r shape lint description; do
	[ -n "$shape" ] || continue
	checked=$((checked + 1))

	line=$(grep -n "// shape: ${shape}\$" "$PROBE" | cut -d: -f1 || true)
	if [ -z "$line" ]; then
		printf 'FAIL  %-38s the probe holds no line marked "// shape: %s"\n' \
			"$lint" "$shape"
		fail=1
		continue
	fi

	if grep -qx "${line}|${lint}" "$WORK/observed"; then
		printf 'PASS  line %-3s %-32s %s\n' "$line" "$lint" "$description"
	else
		printf 'FAIL  line %-3s %-32s %s reached the compiler and no lint stopped it\n' \
			"$line" "$lint" "$description"
		fail=1
	fi
done <<EXPECTED_EOF
$EXPECTED
EXPECTED_EOF

echo
echo "Checked $checked lines. Seven distinct lints cover them, because the index"
echo "and the slice both fire clippy::indexing_slicing. Five categories of bad"
echo "shape are covered: unchecked indexing, unchecked arithmetic, unwrapping, a"
echo "lossy cast, and unsafe code."

extra=$(grep -c . "$WORK/observed" || true)
echo "clippy and rustc reported $extra errors over the eight lines. The division"
echo "line fires two lints, clippy::integer_division and"
echo "clippy::arithmetic_side_effects, so the count is one more than the lines."

if [ "$fail" -ne 0 ]; then
	echo
	echo "The wall did not stop every bad shape. The wall is wrong, not the probe."
	echo "Repair the [workspace.lints] tables in Cargo.toml. Do not add an allow."
	echo
	echo "Every lint that clippy did report:"
	cat "$WORK/observed"
	exit 1
fi

echo
echo "Removing the probe and running the gate."
cleanup
trap - EXIT

cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test --workspace

echo
echo "The wall stops every bad shape, and the tree it leaves behind is clean."
