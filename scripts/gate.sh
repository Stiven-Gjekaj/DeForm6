#!/bin/sh
# Runs locally what `.github/workflows/gate.yml` runs in CI, in the same
# order, so a push is not the first thing that discovers a failure.
#
# `gate.yml` is the source of truth. This script exists because remembering
# nine steps by hand is the kind of fix that has to be repeated, and a fix
# that has to be repeated is not one. A `cargo doc` step was missed exactly
# that way, and CI caught it after the push.
#
# The drift this most likely suffers is a new `scripts/*.sh` step added to
# `gate.yml` and not added here. Stage 0 below reads the workflow and refuses
# to run when it names a script this file does not.

set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$ROOT"

WORKFLOW=.github/workflows/gate.yml

# The wall scripts this file runs, in the order gate.yml runs them.
WALLS='prove-lint-wall
prove-region-wall
prove-capacity-wall
prove-ordered-output-wall
check-claim-surface'

say() {
	printf '\n=== %s ===\n' "$1"
}

# --- Stage 0: refuse to run if this script has drifted from the workflow ---

say "0. this script still matches $WORKFLOW"
missing=''
for named in $(grep -oE 'sh scripts/[a-z-]+\.sh' "$WORKFLOW" | sed 's|sh scripts/||; s|\.sh$||'); do
	if ! printf '%s\n' "$WALLS" | grep -qx -- "$named"; then
		missing="$missing $named"
	fi
done
if [ -n "$missing" ]; then
	echo "ERROR: $WORKFLOW runs script(s) this file does not:$missing" >&2
	echo "Add them to WALLS in scripts/gate.sh, in the order the workflow runs them." >&2
	exit 1
fi
echo "every scripts/*.sh step in the workflow is run below"

# --- The nine steps, in the workflow's own order ------------------------

say "1. cargo fmt"
cargo fmt --all --check

say "2. cargo clippy"
cargo clippy --all-targets -- -D warnings

say "3. cargo test"
cargo test --workspace

# The step that was missed. Broken and private intra-doc links are denied
# here and nowhere else: `cargo test` and `cargo clippy` both pass with a
# link that points at a private item.
say "4. cargo doc"
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --workspace

step=5
for wall in $WALLS; do
	say "$step. $wall"
	sh "scripts/$wall.sh"
	step=$((step + 1))
done

printf '\n=== the gate passes ===\n'
