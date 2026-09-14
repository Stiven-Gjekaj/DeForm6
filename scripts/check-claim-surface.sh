#!/bin/sh
#
# Prove that the claim surface a reader meets before running the tool holds
# no claim the measurement does not support.
#
# Success criterion 2 names three sources: README.md, the --help output, and
# the report vocabulary. This script scans exactly those three for six
# forbidden shapes: a figure stated as a share of one hundred, a claim of
# statement recovery, a claim that the output compiles, and a claim of
# source recovery from native code. None of the three base gate commands can
# catch this. `cargo fmt` and `cargo clippy` read Rust syntax, never prose,
# and `cargo test` proves what the code does, never what a sentence beside
# it claims. A sentence is not a type, so nothing upstream of this script
# stops one.
#
# `tests/ratios.toml` is deliberately out of scope. Its decimal ratios are
# pinned test data a differential harness compares a build against, not a
# claim surface a reader meets before they run the tool. This is decision
# D-04, settled during planning. Do not "fix" this exclusion by adding
# tests/ratios.toml to the sources this script scans; that widens the scope
# success criterion 2 does not ask for, and breaks the pinned-ratio tests'
# own freedom to hold a plain decimal.
#
# A check that passes because it looked in the wrong place is worse than no
# check, because it makes something look guarded while it drifts. Stage 4
# below plants one violation of every shape into every source and requires
# each of the eighteen to fire before this script ever reports a pass on the
# real tree.
#
# Run it from anywhere:
#
#     sh scripts/check-claim-surface.sh
#
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$ROOT"

WORK=$(mktemp -d)
cleanup() {
	rm -rf "$WORK"
}
trap cleanup EXIT

# The six forbidden shapes. One row per shape: an identifier, an extended
# regular expression, and what it catches. Scanned case-insensitively.
SHAPES='pct-sign;[0-9]+(\.[0-9]+)?[[:space:]]*%;a figure stated as a share of one hundred
pct-word;[0-9]+[[:space:]]*(per cent|percent);the same figure written as a word
stmt-fwd;recover[a-z]*[^.]{0,40}statement;a claim of statement recovery
stmt-rev;statement[^.]{0,40}recover[a-z]*;the same claim, written the other way round
compilable;compilable|recompilable|compile.ready;a claim that the output compiles
decompile-src;decompil[a-z]*[^.]{0,40}(source|[Bb]asic);a claim of source recovery from native code'

# One violating line per shape, used only by the stage 4 probe below.
PROBES='pct-sign;This run recovers 92% of the corpus.
pct-word;This run recovers 92 percent of the corpus.
stmt-fwd;DeForm6 can recover every statement in the file.
stmt-rev;The statement is recovered fully.
compilable;The output is fully compilable Basic.
decompile-src;This binary decompiles cleanly into Basic source.'

# The audited allow list. Some sentences legitimately hold a forbidden shape
# because they are negations, not claims. A reviewer reads this list. Do not
# widen a regular expression above to make a real claim pass; if a new
# negation earns a place here, it goes on this list with its reason, on
# purpose.
#
# "It does not recover statements."
#   Reason: the README's own opening negation of the claim stmt-fwd hunts,
#   not a claim that the tool recovers statements.
#
# "DeForm6 does not recover statements. The forms, the control trees, the"
#   Reason: the opening line of the README's "What version 1.0 does not
#   return" section, the same negation restated as the section's own claim
#   about the tool.
ALLOWED='It does not recover statements.
DeForm6 does not recover statements. The forms, the control trees, the'

# The three facts success criterion 3 names. README.md must state all three,
# and removing any one of them from a copy must be detectable.
FACTS='lpNativeCode
frmHMM.frx
needs the Visual Basic 6 IDE on'

is_allowed() {
	candidate=$1
	while IFS= read -r entry; do
		[ -n "$entry" ] || continue
		if [ "$candidate" = "$entry" ]; then
			return 0
		fi
	done <<ALLOWED_EOF
$ALLOWED
ALLOWED_EOF
	return 1
}

probe_line_for() {
	want=$1
	while IFS=';' read -r id line; do
		[ -n "$id" ] || continue
		if [ "$id" = "$want" ]; then
			printf '%s' "$line"
			return 0
		fi
	done <<PROBES_EOF
$PROBES
PROBES_EOF
}

fact_check() {
	file=$1
	ok=1
	while IFS= read -r fact_entry; do
		[ -n "$fact_entry" ] || continue
		if ! grep -qF -- "$fact_entry" "$file"; then
			ok=0
		fi
	done <<FACTS_EOF
$FACTS
FACTS_EOF
	[ "$ok" -eq 1 ]
}

echo "Stage 1: collecting the three claim surfaces into $WORK."
echo

cp README.md "$WORK/src-readme.txt"

cargo run -q -p deform6-cli -- --help >"$WORK/src-help.txt"
cargo run -q -p deform6-cli -- inspect --help >>"$WORK/src-help.txt"
cargo run -q -p deform6-cli -- extract --help >>"$WORK/src-help.txt"

cargo run -q -p deform6-cli -- extract \
	corpus/public-domain/PassGen/PassGen.exe -o "$WORK/out"
REPORT_FILE=$(find "$WORK/out" -maxdepth 1 -name '*.report.json' | head -n 1)
if [ -z "$REPORT_FILE" ]; then
	echo "FAIL  the extract run wrote no *.report.json file under $WORK/out."
	exit 1
fi
cp "$REPORT_FILE" "$WORK/src-report.txt"
printf 'proven\ninferred\nunrecoverable\n' >>"$WORK/src-report.txt"

SOURCES="readme;$WORK/src-readme.txt
help;$WORK/src-help.txt
report;$WORK/src-report.txt"

echo "Stage 2 and 3: scanning the three surfaces for six forbidden shapes,"
echo "classifying every hit against the audited allow list."
echo

FAIL_COUNT=0
while IFS=';' read -r slabel sfile; do
	[ -n "$slabel" ] || continue
	while IFS=';' read -r shape_id shape_re shape_desc; do
		[ -n "$shape_id" ] || continue
		grep -inE "$shape_re" "$sfile" >"$WORK/hits" 2>/dev/null || true
		while IFS=: read -r lineno rest; do
			[ -n "$lineno" ] || continue
			if is_allowed "$rest"; then
				printf 'ALLOWED %-7s line %-4s %-14s %s\n' \
					"$slabel" "$lineno" "$shape_id" "$rest"
			else
				printf 'FAIL    %-7s line %-4s %-14s %s\n' \
					"$slabel" "$lineno" "$shape_id" "$rest"
				FAIL_COUNT=$((FAIL_COUNT + 1))
			fi
		done <"$WORK/hits"
	done <<SHAPES_EOF
$SHAPES
SHAPES_EOF
done <<SOURCES_EOF
$SOURCES
SOURCES_EOF

if [ "$FAIL_COUNT" -ne 0 ]; then
	echo
	echo "The claim surface holds $FAIL_COUNT line(s) not on the audited"
	echo "allow list. Rewrite the line so it no longer holds the shape, or"
	echo "trace it and add it to the ALLOWED list in this script on purpose."
	exit 1
fi

echo
echo "Stage 4: the probe. Planting one violation of every shape into every"
echo "surface, and requiring the same scanner to catch each one."
echo

PLANTED_FIRED=0
while IFS=';' read -r slabel sfile; do
	[ -n "$slabel" ] || continue
	while IFS=';' read -r shape_id shape_re shape_desc; do
		[ -n "$shape_id" ] || continue
		vline=$(probe_line_for "$shape_id")
		cp "$sfile" "$WORK/probe.txt"
		printf '%s\n' "$vline" >>"$WORK/probe.txt"
		hits=$(grep -icE "$shape_re" "$WORK/probe.txt" 2>/dev/null || true)
		[ -n "$hits" ] || hits=0
		if [ "$hits" -ge 1 ]; then
			PLANTED_FIRED=$((PLANTED_FIRED + 1))
			printf 'PASS  %-7s %-14s %s\n' "$slabel" "$shape_id" "$vline"
		else
			echo
			echo "FAIL  the scanner is wrong, not the probe."
			echo "      shape $shape_id did not fire on $slabel after"
			echo "      planting: $vline"
			exit 1
		fi
	done <<SHAPES_EOF
$SHAPES
SHAPES_EOF
done <<SOURCES_EOF
$SOURCES
SOURCES_EOF

echo
echo "Checking success criterion 3's three facts in README.md, and probing"
echo "that removing any one of them is detected."
echo

if ! fact_check "$WORK/src-readme.txt"; then
	echo "FAIL  README.md no longer holds one of the three facts success"
	echo "      criterion 3 names: lpNativeCode, frmHMM.frx, or the"
	echo "      recompilation sentence."
	exit 1
fi

FACT_PROBES_FIRED=0
while IFS= read -r literal; do
	[ -n "$literal" ] || continue
	grep -vF -- "$literal" "$WORK/src-readme.txt" >"$WORK/probe-fact.txt"
	if fact_check "$WORK/probe-fact.txt"; then
		echo
		echo "FAIL  the fact check is wrong, not the probe."
		echo "      removing \"$literal\" from README.md left it passing."
		exit 1
	else
		FACT_PROBES_FIRED=$((FACT_PROBES_FIRED + 1))
		printf 'FACTPASS %s\n' "$literal"
	fi
done <<FACTS_EOF
$FACTS
FACTS_EOF

echo
echo "Scanned 3 claim surfaces (README.md, the --help output, and the report"
echo "vocabulary) for 6 forbidden shapes. $PLANTED_FIRED of 18 planted"
echo "violations fired, one per shape per surface. $FACT_PROBES_FIRED of 3"
echo "fact probes fired, one per literal success criterion 3 names."
echo "tests/ratios.toml was not scanned; see the header comment for why"
echo "(decision D-04)."
echo
echo "The claim surface holds no claim the measurement does not support."
