#!/bin/sh
#
# Prove that every capacity sized allocation under crates/deform6/src/ is one
# an audit has traced, and that the trace is still true.
#
# AGENTS.md bars an allocation sized from a length field in the file before
# that length is checked against the real size of the file. A rule that is
# written down is not the same thing as a rule that holds. This script holds
# the audited list plan 05-02's task 2 built and compares it against the
# real tree two ways: a site the source holds and the list does not is a
# new, un-audited allocation, and a row the list holds and the source does
# not is a stale claim about code that no longer exists. Either difference
# fails the script.
#
# The audited list is read from a here document below, one record per line,
# `file|verdict|check|expression`. `verdict` is one of `internal` (the value
# is the length of a collection this crate already built, or a compile time
# constant, so the file cannot drive it) or `bounded` (the value came from
# the file, and `check` names the file and the reason a comparison against a
# real region length runs before the allocation). Plan 05-02's task 2 found
# no `unbounded` site: every capacity argument under crates/deform6/src/
# already traces to a check, or to a value the file cannot drive. The full
# audit, row by row, is in 05-02-SUMMARY.md.
#
# Run it from anywhere:
#
#     sh scripts/prove-capacity-wall.sh
#
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$ROOT"

WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT

AUDITED='crates/deform6/src/write/values.rs|internal|text.len() is the length of a &str already resident in memory; no file field drives it|let mut out = String::with_capacity(text.len().saturating_add(2));
crates/deform6/src/report.rs|internal|model_items is a Vec this crate already built and passed in by value|let mut items = Vec::with_capacity(model_items.len());
crates/deform6/src/write/frm.rs|internal|order is built locally, from pending.len(), a Vec this function already built|let mut resolved: Vec<(String, ResolvedLine)> = Vec::with_capacity(order.len());
crates/deform6/src/write/comment.rs|internal|lines is built locally in this function, from items this crate already recovered|let mut block = Vec::with_capacity(lines.len().saturating_add(1));
crates/deform6/src/read/pe.rs|bounded|table is the object crate own SectionTable, already parsed and length checked against the real file bytes by SectionTable::parse (a fallible read_slice_at) before this line runs|let mut sections = Vec::with_capacity(table.len());
crates/deform6/src/write/model.rs|internal|text is a &str parameter already resident in memory|let mut bytes = Vec::with_capacity(text.len());
crates/deform6/src/write/model.rs|internal|raw is a &str parameter already resident in memory|let mut sanitized = String::with_capacity(raw.len());
crates/deform6/src/write/model.rs|internal|controls is a slice this crate already built during inspect(), not a raw file field|let mut models = Vec::with_capacity(controls.len());
crates/deform6/src/vb/privateobj.rs|bounded|object.rs bound_proc_count clamps object.proc_count upstream against lpProcNamesArray own region, and this function own preceding array.subregion(0, window_size) proves the bytes exist before this line runs|let mut procs = Vec::with_capacity(usize::try_from(object.proc_count).unwrap_or(0));
crates/deform6/src/vb/privateobj.rs|bounded|this function own preceding array.subregion(0, window_size) proves the bytes exist before this line runs; no upstream clamp exists for cnt_events|let mut addresses = Vec::with_capacity(usize::from(*cnt_events));
crates/deform6/src/vb/project.rs|internal|a compile time constant, reached only after entry.take(guid_offset, 72) already proved 72 bytes present|let mut units = Vec::with_capacity(36);
crates/deform6/src/vb/functyp.rs|bounded|object.rs bound_proc_count clamps object.proc_count upstream, and this function own preceding array.subregion(0, window_size) against lpFuncTypeInfo own region proves the bytes exist before this line runs|let mut signatures = Vec::with_capacity(usize::try_from(object.proc_count).unwrap_or(0));
crates/deform6/src/vb/functyp.rs|bounded|arg_size is a u8, wanted is arg_size shr 2, domain bounded to a maximum of 63 entries whatever the file declares|let mut entries: Vec<RawEntry> = Vec::with_capacity(wanted);
crates/deform6/src/vb/functyp.rs|internal|arg_count is entries.len(), the length of a Vec walk_type_buffer already built and returned|let mut arguments = Vec::with_capacity(arg_count);'

# The real sites. Each file is read from its own line 1, and everything from
# the first line that opens a test module (`#[cfg(test)]`, at the start of a
# line) to the end of the file is dropped, the same rule
# `vb/controlinfo.rs::production_code_only` uses for its own, single file
# check. A whole-line comment is dropped too, so a comment that names the
# constructor is never counted as a site.
find crates/deform6/src -name '*.rs' | sort >"$WORK/files"

: >"$WORK/found"
while IFS= read -r file; do
	awk -v file="$file" '
		/^#\[cfg\(test\)\]/ { exit }
		{
			trimmed = $0
			sub(/^[ \t]+/, "", trimmed)
			if (trimmed ~ /^\/\//) next
			if (index($0, "with_capacity(") > 0) {
				print file "|" trimmed
			}
		}
	' "$file" >>"$WORK/found"
done <"$WORK/files"

# One `file|expression` key per audited row, with the verdict and the check
# dropped, so both directions of the comparison below key on the same shape
# `$WORK/found` already holds.
awk -F'|' '{print $1 "|" $4}' <<AUDITED_EOF >"$WORK/audited_keys"
$AUDITED
AUDITED_EOF

fail=0
checked=0

while IFS='|' read -r a_file a_verdict a_check a_expr; do
	[ -n "$a_file" ] || continue
	checked=$((checked + 1))
	if grep -qxF "${a_file}|${a_expr}" "$WORK/found"; then
		printf 'PASS  %-8s %s: %s\n' "$a_verdict" "$a_file" "$a_expr"
	else
		printf 'FAIL  stale audit row, no longer in the source: %s: %s\n' "$a_file" "$a_expr"
		echo "      Remove this row from scripts/prove-capacity-wall.sh."
		fail=1
	fi
done <<AUDITED_EOF
$AUDITED
AUDITED_EOF

while IFS='|' read -r f_file f_expr; do
	[ -n "$f_file" ] || continue
	if ! grep -qxF "${f_file}|${f_expr}" "$WORK/audited_keys"; then
		line=$(grep -nF "$f_expr" "$f_file" | head -1 | cut -d: -f1)
		printf 'FAIL  un-audited capacity site: %s:%s\n' "$f_file" "$line"
		echo "      ${f_expr}"
		echo "      Trace the capacity argument to its origin. If it comes from"
		echo "      the file, add the bound check next to the field it bounds,"
		echo "      following object.rs::bound_proc_count's own four steps."
		echo "      Then add this row to scripts/prove-capacity-wall.sh."
		fail=1
	fi
done <"$WORK/found"

echo
echo "Checked $checked audited capacity sites under crates/deform6/src/."

if [ "$fail" -ne 0 ]; then
	echo
	echo "The capacity wall did not hold. See the FAIL lines above."
	exit 1
fi

echo "Every capacity sized allocation under crates/deform6/src/ is one this"
echo "audit has traced, and the audit still matches the tree."

echo
echo "Running the gate."
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test --workspace

echo
echo "The capacity wall holds, and the tree it leaves behind is clean."
