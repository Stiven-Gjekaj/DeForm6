#!/bin/sh
#
# Prove that `Region`, `Off`, `Rva` and `Va` refuse the shapes a caller must
# not be able to write.
#
# The lint wall from `scripts/prove-lint-wall.sh` catches a bad shape written
# with primitives. It does not catch a caller that reaches into a `Region` or
# adds two offsets, because the type refuses those shapes, not a lint. The only
# way to show that a type refuses a shape is to write the shape and watch the
# compiler reject it.
#
# This script writes one probe at a time, compiles it, requires the compile to
# fail with the expected rustc error code on the probe file, and then removes
# the probe. A `trap` removes every probe on exit, so a failing run leaves no
# broken tree.
#
# The probe is an example target, for the same three reasons the lint wall
# script uses one. `cargo clippy --all-targets` and `cargo build --examples`
# both compile `crates/*/examples/*.rs`, an example is found by its path and
# needs no `mod` line, and the probe therefore never enters the library.
#
# `cargo build` appears below, and `AGENTS.md` forbids it as the gate. That
# rule holds. `build` is not the gate here. The question asked of each probe is
# "does this shape reach the compiler", and `build` answers it. The three gate
# commands run at the end of this script, after the probe is removed.
#
# Run it from anywhere:
#
#     sh scripts/prove-region-wall.sh
#
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$ROOT"

PROBE=crates/deform6/examples/region_probe.rs
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

# Probe one. `Region` holds `bytes` and `base` and exposes neither. A caller
# that reaches the slice directly would step around every bounds check.
cat >"$WORK/private-field.rs" <<'PROBE_EOF'
//! A probe that must not compile.
//!
//! `scripts/prove-region-wall.sh` writes this file, reads the error, and then
//! removes it. Do not commit it.

use deform6::read::region::{Off, Region};

fn main() {
    let data = [0_u8; 4];
    let region = Region::new(&data, Off::new(0));
    let _bytes = region.bytes;
}
PROBE_EOF

# Probe two. Adding two offsets is meaningless, and it is the shape that
# produces the silent overflow the project exists to refuse. `Off` implements
# no `Add`, so there is no `+` to write it with.
cat >"$WORK/add-two-offsets.rs" <<'PROBE_EOF'
//! A probe that must not compile.
//!
//! `scripts/prove-region-wall.sh` writes this file, reads the error, and then
//! removes it. Do not commit it.

use deform6::read::region::Off;

fn main() {
    let a = Off::new(0x10);
    let b = Off::new(0x20);
    let _sum = a + b;
}
PROBE_EOF

# Probe three. `Region` implements no `Index`, so a caller cannot index it and
# cannot panic on an out-of-range index. `take` is the only route out.
cat >"$WORK/index-a-region.rs" <<'PROBE_EOF'
//! A probe that must not compile.
//!
//! `scripts/prove-region-wall.sh` writes this file, reads the error, and then
//! removes it. Do not commit it.

use deform6::read::region::{Off, Region};

fn main() {
    let data = [0_u8; 4];
    let region = Region::new(&data, Off::new(0));
    let _byte = region[0];
}
PROBE_EOF

# One row for each shape: the probe file, the rustc error code that must
# refuse it, and what the shape is. The codes were read from this toolchain,
# not guessed. They are pinned so that a later change which turns a hard error
# into a warning is caught here.
EXPECTED='private-field|E0616|reaching the bytes of a Region directly
add-two-offsets|E0369|adding two Off values with the plus operator
index-a-region|E0608|indexing a Region with square brackets'

mkdir -p crates/deform6/examples

echo "The probe goes to $PROBE, one shape at a time."
echo

fail=0
checked=0

while IFS='|' read -r shape code description; do
	[ -n "$shape" ] || continue
	checked=$((checked + 1))

	cp "$WORK/$shape.rs" "$PROBE"

	# The compile is meant to fail, so keep its status away from `set -e`.
	cargo build --examples --message-format short \
		>"$WORK/$shape.out" 2>&1 || true

	rm -f "$PROBE"

	# The error must name the probe file, not merely appear somewhere in the
	# output. That proves the probe line produced it.
	if grep -q "region_probe\.rs.*error\[${code}\]" "$WORK/$shape.out"; then
		printf 'PASS  %-6s %s\n' "$code" "$description"
	else
		printf 'FAIL  %-6s %s\n' "$code" "$description"
		echo "      The compiler did not refuse this shape with $code."
		echo "      A shape that compiles is a hole in the type. Repair the"
		echo "      type in crates/deform6/src/read/region.rs. Do not relax"
		echo "      this script."
		echo "      What the compiler said:"
		sed 's/^/      /' "$WORK/$shape.out"
		fail=1
	fi
done <<EXPECTED_EOF
$EXPECTED
EXPECTED_EOF

echo
echo "Checked $checked shapes."

if [ "$fail" -ne 0 ]; then
	echo "At least one shape reached the compiler. The type is wrong."
	exit 1
fi

echo "Each one is refused by the type, not by a lint, so none of the three gate"
echo "commands can catch a change that lets one in. That is why this script"
echo "runs in the gate as a fourth step."

echo
echo "Removing the probe and running the gate."
cleanup
trap - EXIT

cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test --workspace

echo
echo "The type refuses every shape, and the tree it leaves behind is clean."
