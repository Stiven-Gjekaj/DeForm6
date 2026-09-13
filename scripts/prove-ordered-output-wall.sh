#!/bin/sh
#
# Prove that no hash keyed container can change what this tool writes.
#
# RPT-01 and RPT-05 ask for a byte identical report. A hash keyed container
# gives no order of its own, so a `HashMap` or a `HashSet` that something
# iterates puts the iteration order of a hash table into the output, and the
# output stops being byte identical. Nothing in the three gate commands
# catches that. The tests that prove determinism pass until the day the
# iteration order happens to differ.
#
# Phase 4 wrote the rule as a source assertion in two plan files: report.rs
# holds no HashMap, and write/model.rs holds no HashMap. A rule that lives in
# a plan file runs once, on the day that plan runs. Phase 5 proved the cost
# of that: plan 05-06 fixed a real quadratic search in PathIssuer and
# SafeNameIssuer, reached for HashSet and HashMap to do it, and both
# assertions were broken for four commits before a human read them again.
# This script is that rule, moved to where it runs on every gate.
#
# The wall has two halves.
#
# The ban. No hash keyed container at all, anywhere in production code, in
# report.rs or under write/. Those files are the ones that turn a recovered
# model into bytes on disk. There is no safe use of a hash container in them,
# because anything that reaches those files reaches the output.
#
# The audit. Everywhere else under crates/deform6/src/, a hash container is
# allowed when it is a keyed lookup table that nothing iterates. Reading a
# value out by key is order free. The audited list below names each such site
# and the reason it is safe. Both directions are checked: a site the source
# holds and the list does not is a new, un-audited container, and a row the
# list holds and the source does not is a stale claim about code that no
# longer exists. Either difference fails the script.
#
# A test module is dropped, from the first line that opens one to the end of
# the file, the same rule scripts/prove-capacity-wall.sh uses. A test may use
# a hash container freely. A test asserts a set of values, it does not write
# the output.
#
# Run it from anywhere:
#
#     sh scripts/prove-ordered-output-wall.sh
#
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$ROOT"

WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT

# The audited sites. One record per line, `file|identifier|reason`.
# `identifier` is the binding or field the container is held in, and the
# script proves that nothing iterates it. `reason` says why a keyed lookup
# is safe here.
AUDITED='crates/deform6/src/vb/controlinfo.rs|entries|the shipped control information table, read by a (name, index) key and never walked
crates/deform6/src/vb/mod.rs|control_info_by_name|an index built over the shipped table so compose_form can look a control up by name, never walked
crates/deform6/src/vb/opcodes.rs|entries|the opcode table, read by a (prefix, opcode) key and never walked'

# The files the ban covers. Anything that turns a model into bytes.
BANNED_FILES=$(find crates/deform6/src/write -name '*.rs' 2>/dev/null | sort)
BANNED_FILES="crates/deform6/src/report.rs
$BANNED_FILES"

# Every production line naming a hash container, whole file list.
find crates/deform6/src -name '*.rs' | sort >"$WORK/files"
: >"$WORK/found"
while IFS= read -r file; do
	awk -v file="$file" '
		/^#\[cfg\(test\)\]/ { exit }
		{
			trimmed = $0
			sub(/^[ \t]+/, "", trimmed)
			if (trimmed ~ /^\/\//) next
			if ($0 ~ /HashMap|HashSet/) {
				print file "|" trimmed
			}
		}
	' "$file" >>"$WORK/found"
done <"$WORK/files"

fail=0

echo "The ban: no hash keyed container in the files that write the output."
echo

banned_checked=0
while IFS= read -r file; do
	[ -n "$file" ] || continue
	[ -f "$file" ] || continue
	banned_checked=$((banned_checked + 1))
	hits=$(grep -cF "${file}|" "$WORK/found" || true)
	if [ "$hits" -eq 0 ]; then
		printf 'PASS  ban    %s\n' "$file"
	else
		printf 'FAIL  ban    %s holds %s hash container line(s)\n' "$file" "$hits"
		grep -F "${file}|" "$WORK/found" | sed 's/^/      /'
		echo "      This file turns a recovered model into bytes. A hash keyed"
		echo "      container here can reach the output and RPT-01 and RPT-05"
		echo "      ask for that output to be byte identical. Use BTreeMap or"
		echo "      BTreeSet. They give the same API and an order of their own."
		fail=1
	fi
done <<BANNED_EOF
$BANNED_FILES
BANNED_EOF

echo
echo "The audit: every other hash container is a keyed lookup nothing walks."
echo

audited_checked=0
: >"$WORK/audited_files"
while IFS='|' read -r a_file a_ident a_reason; do
	[ -n "$a_file" ] || continue
	audited_checked=$((audited_checked + 1))
	echo "$a_file" >>"$WORK/audited_files"

	if ! grep -qF "${a_file}|" "$WORK/found"; then
		printf 'FAIL  stale  %s no longer holds a hash container\n' "$a_file"
		echo "      Remove this row from scripts/prove-ordered-output-wall.sh."
		fail=1
		continue
	fi

	# Nothing may walk the audited container. An iteration is what puts a
	# hash table's own order into anything downstream.
	walked=$(awk -v ident="$a_ident" '
		/^#\[cfg\(test\)\]/ { exit }
		{
			trimmed = $0
			sub(/^[ \t]+/, "", trimmed)
			if (trimmed ~ /^\/\//) next
			if ($0 ~ ident "\\.(iter|iter_mut|into_iter|keys|values|values_mut|drain)\\(") { print NR ": " trimmed }
			if ($0 ~ "for .* in .*" ident "[^A-Za-z0-9_]") { print NR ": " trimmed }
		}
	' "$a_file")
	if [ -n "$walked" ]; then
		printf 'FAIL  walked %s: %s is iterated\n' "$a_file" "$a_ident"
		printf '%s\n' "$walked" | sed 's/^/      /'
		echo "      A keyed lookup is order free. Walking one is not. Either stop"
		echo "      walking it, or make it a BTreeMap and move this row to the"
		echo "      reason column that says so."
		fail=1
	else
		printf 'PASS  audit  %s: %s, %s\n' "$a_file" "$a_ident" "$a_reason"
	fi
done <<AUDITED_EOF
$AUDITED
AUDITED_EOF

# A production hash container in a file the audit does not name at all.
sort -u "$WORK/audited_files" >"$WORK/audited_files_u" 2>/dev/null || true
while IFS='|' read -r f_file f_expr; do
	[ -n "$f_file" ] || continue
	case "$f_file" in
		crates/deform6/src/report.rs|crates/deform6/src/write/*) continue ;;
	esac
	if ! grep -qxF "$f_file" "$WORK/audited_files_u"; then
		line=$(grep -nF "$f_expr" "$f_file" | head -1 | cut -d: -f1)
		printf 'FAIL  new    un-audited hash container: %s:%s\n' "$f_file" "$line"
		echo "      ${f_expr}"
		echo "      Trace it. If anything walks it, or if what it holds can reach"
		echo "      the written output, use BTreeMap or BTreeSet. If it is a keyed"
		echo "      lookup nothing walks, add a row to"
		echo "      scripts/prove-ordered-output-wall.sh saying so."
		fail=1
	fi
done <"$WORK/found"

echo
echo "Checked $banned_checked files under the ban and $audited_checked audited sites."

if [ "$fail" -ne 0 ]; then
	echo
	echo "The ordered output wall did not hold. See the FAIL lines above."
	exit 1
fi

echo "No hash keyed container can reach what this tool writes, and every one"
echo "that remains is a keyed lookup nothing walks."

echo
echo "Running the gate."
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test --workspace

echo
echo "The ordered output wall holds, and the tree it leaves behind is clean."
