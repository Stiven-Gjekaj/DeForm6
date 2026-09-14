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
# real tree. It also plants a violation split across two physical lines for
# every shape a line wrap can hide (see join_source below) into every
# source, and requires each of those to fire too.
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
#
# compilable's third alternative uses a character class for the separator,
# not a bare `.`. A bare `.` is an unescaped regex metacharacter that
# matches any single character, not only a space or a hyphen, so it used to
# also match a line such as "compileXready".
#
# compilable's first two alternatives now catch the verb directly
# ("compile", "compiles", "compiled") and not only the "-able" suffix, so
# the plainer, more natural phrasing "the recovered project compiles in
# the Visual Basic 6 IDE" is caught. Each requires a non-letter (or end of
# line) right after the word it matches, so it still catches those forms
# but not "compiler": that word names an input file's own compiler flag
# throughout this project's report vocabulary, never a claim that
# DeForm6's output compiles, and without this guard it fires on every one
# of the report's many "does not carry the X compiler flag" lines.
SHAPES='pct-sign;[0-9]+(\.[0-9]+)?[[:space:]]*%;a figure stated as a share of one hundred
pct-word;[0-9]+[[:space:]]*(per cent|percent);the same figure written as a word
stmt-fwd;recover[a-z]*[^.]{0,40}statement;a claim of statement recovery
stmt-rev;statement[^.]{0,40}recover[a-z]*;the same claim, written the other way round
compilable;compil(able|ed|es|e)([^a-zA-Z]|$)|recompil(able|ed|es|e)([^a-zA-Z]|$)|compile[- ]ready;a claim that the output compiles
decompile-src;decompil[a-z]*[^.]{0,40}(source|[Bb]asic);a claim of source recovery from native code'

# One violating line per shape, used only by the stage 4 probe below. The
# compilable line uses the plain "compiles" phrasing, not "compilable", so
# the probe actually exercises the widened shape above and not only the
# narrower case the pre-fix shape already caught.
PROBES='pct-sign;This run recovers 92% of the corpus.
pct-word;This run recovers 92 percent of the corpus.
stmt-fwd;DeForm6 can recover every statement in the file.
stmt-rev;The statement is recovered fully.
compilable;The recovered project compiles without changes.
decompile-src;This binary decompiles cleanly into Basic source.'

# One violation per shape, split across two physical lines with no blank
# line between them, the same way this repository's own hard-wrapped prose
# splits a sentence (see README.md:3-4, README.md:19-20). Neither line
# holds the shape on its own; only the two joined together do. This is the
# case join_source exists to catch: a line-based regex never assembles
# these two lines on its own.
#
# The "compilable" shape has no two-line probe here on purpose. Its trigger
# is a single word ("compiles", "compilable", ...) that a hard wrap never
# splits, because a hard wrap only ever breaks at white space, never inside
# a word. That shape is not exposed to the line-wrap gap the other five
# are, so a two-line probe for it would prove nothing a single-line probe
# does not already prove.
WRAPPED_PROBES='pct-sign;This run recovers 92;% of the corpus, measured against source.
stmt-fwd;DeForm6 can recover every;statement from the executable directly.
stmt-rev;Every statement in the file comes back fully;recovered, without exception.
decompile-src;This binary decompiles cleanly into;Basic source, without further work.'

# The audited allow list. Some sentences legitimately hold a forbidden shape
# because they are negations, not claims. A reviewer reads this list. Do not
# widen a regular expression above to make a real claim pass; if a new
# negation earns a place here, it goes on this list with its reason, on
# purpose.
#
# Every entry here is the text of one whole joined segment (see join_source
# below), not a raw physical line, because the scan below runs against the
# joined text. A segment is usually a whole hard-wrapped paragraph or list
# item; a markdown table row or a JSON report line is always its own
# segment, unchanged.
#
# "DeForm6 reads a compiled Visual Basic 6 executable and writes back a
# Visual Basic project. The first milestone recovers the metadata only: the
# forms, the control trees, the property values, the names, and the
# procedure signatures. It does not recover statements. Do not add a claim
# that it does."
#   Reason: the README's own opening paragraph, ending in the negation of
#   the claim stmt-fwd hunts, not a claim that the tool recovers
#   statements.
#
# "DeForm6 does not recover statements. The forms, the control trees, the
# property values, the names, and the procedure signatures come back. The
# code inside a procedure does not."
#   Reason: the opening paragraph of the README's "What version 1.0 does
#   not return" section, the same negation restated as the section's own
#   claim about the tool.
#
# "DeForm6 does not open the Visual Basic 6 IDE and it does not compile
# anything. No sentence in this file states a recovery figure as a share of
# one hundred. Every capability sentence in this file traces to a report
# field, a confidence word, or a limit the report states in its own words."
#   Reason: "it does not compile anything" is the compile claim's own
#   negation, stated plainly.
#
# "| S-10 | How a source level `Alias "#123"` ordinal import is encoded in
# the compiled file. | When the Visual Basic level name is not present in
# the file, the tool reports the export as an inferred ordinal number,
# rather than guessing a name. |"
#   Reason: a known-limits table row. "encoded in the compiled file"
#   describes the compiled input executable the row is about, not a claim
#   that DeForm6's own output compiles.
#
# "`deform6`: reads a compiled Visual Basic 6 executable and reports what
# it holds"
#   Reason: the CLI's own one-line `--help` summary. "a compiled ...
#   executable" describes the input file, the same way it does in
#   README.md's opening paragraph above.
ALLOWED='DeForm6 reads a compiled Visual Basic 6 executable and writes back a Visual Basic project. The first milestone recovers the metadata only: the forms, the control trees, the property values, the names, and the procedure signatures. It does not recover statements. Do not add a claim that it does.
DeForm6 does not recover statements. The forms, the control trees, the property values, the names, and the procedure signatures come back. The code inside a procedure does not.
DeForm6 does not open the Visual Basic 6 IDE and it does not compile anything. No sentence in this file states a recovery figure as a share of one hundred. Every capability sentence in this file traces to a report field, a confidence word, or a limit the report states in its own words.
| S-10 | How a source level `Alias "#123"` ordinal import is encoded in the compiled file. | When the Visual Basic level name is not present in the file, the tool reports the export as an inferred ordinal number, rather than guessing a name. |
`deform6`: reads a compiled Visual Basic 6 executable and reports what it holds'

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

# join_source reads $1 (a source file) and writes two parallel files: $2
# holds one joined segment of text per line, and $3 holds the original
# physical line range ("start-end") each segment came from, in the same
# order, so line N of $3 is where line N of $2's text lives in $1.
#
# A hard-wrapped sentence spans two or more physical lines with no blank
# line between them (README.md:3-4, README.md:19-20 are two examples). A
# single-line regex never assembles the two halves of such a sentence, so
# every ordinary run of non-blank lines is joined into one segment here
# before the shape regexes run in the caller.
#
# A markdown heading, a markdown table row, and a JSON structural line
# (a brace, a bracket, or a quoted key or string) are never wrapped by this
# repository's own style; each already holds one complete unit on its own
# physical line. Joining rows of a table, or lines of a JSON report,
# together would flatten a whole table or a whole report into one blob,
# lose the audited allow list's exact-string match, and prove nothing a
# per-line scan did not already prove. Each of those three kinds of line
# stays a segment of its own instead.
#
# A markdown list item starts a new segment at its bullet or number, and
# every indented continuation line that follows with no bullet of its own
# joins into that same segment, the same way a wrapped list item's second
# line does in this file today.
join_source() {
	: >"$2"
	: >"$3"
	awk -v textfile="$2" -v locfile="$3" '
		function flush(endline) {
			if (buf != "") {
				print buf > textfile
				print start "-" endline > locfile
				buf = ""
			}
		}
		/^[[:space:]]*$/ {
			flush(NR - 1)
			next
		}
		/^[[:space:]]*(\||#|\{|\}|\[|\]|")/ {
			flush(NR - 1)
			print $0 > textfile
			print NR "-" NR > locfile
			next
		}
		/^[[:space:]]*[-*][[:space:]]/ || /^[[:space:]]*[0-9]+\.[[:space:]]/ {
			flush(NR - 1)
			start = NR
			buf = $0
			next
		}
		{
			if (buf == "") { start = NR }
			buf = (buf == "" ? $0 : buf " " $0)
		}
		END { flush(NR) }
	' "$1"
}

loc_for() {
	sed -n "${2}p" "$1"
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
	join_source "$sfile" "$WORK/joined-$slabel.txt" "$WORK/loc-$slabel.txt"
	while IFS=';' read -r shape_id shape_re shape_desc; do
		[ -n "$shape_id" ] || continue
		grep -inE "$shape_re" "$WORK/joined-$slabel.txt" >"$WORK/hits" 2>/dev/null || true
		while IFS=: read -r lineno rest; do
			[ -n "$lineno" ] || continue
			location=$(loc_for "$WORK/loc-$slabel.txt" "$lineno")
			if is_allowed "$rest"; then
				printf 'ALLOWED %-7s line %-9s %-14s %s\n' \
					"$slabel" "$location" "$shape_id" "$rest"
			else
				printf 'FAIL    %-7s line %-9s %-14s %s\n' \
					"$slabel" "$location" "$shape_id" "$rest"
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
echo "surface, and requiring the same scanner to catch each one. Then"
echo "planting one two-line, wrapped violation of every shape a line wrap"
echo "can hide into every surface, and requiring the same scanner, run"
echo "through the same line-join, to catch each of those too."
echo

PLANTED_FIRED=0
while IFS=';' read -r slabel sfile; do
	[ -n "$slabel" ] || continue
	while IFS=';' read -r shape_id shape_re shape_desc; do
		[ -n "$shape_id" ] || continue
		vline=$(probe_line_for "$shape_id")
		cp "$sfile" "$WORK/probe.txt"
		printf '%s\n' "$vline" >>"$WORK/probe.txt"
		join_source "$WORK/probe.txt" "$WORK/probe-joined.txt" "$WORK/probe-loc.txt"
		hits=$(grep -icE "$shape_re" "$WORK/probe-joined.txt" 2>/dev/null || true)
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

WRAPPED_FIRED=0
while IFS=';' read -r slabel sfile; do
	[ -n "$slabel" ] || continue
	while IFS=';' read -r shape_id vline1 vline2; do
		[ -n "$shape_id" ] || continue
		shape_re=$(printf '%s\n' "$SHAPES" | while IFS=';' read -r id re desc; do
			[ "$id" = "$shape_id" ] || continue
			printf '%s' "$re"
			break
		done)
		cp "$sfile" "$WORK/wprobe.txt"
		printf '%s\n%s\n' "$vline1" "$vline2" >>"$WORK/wprobe.txt"
		join_source "$WORK/wprobe.txt" "$WORK/wprobe-joined.txt" "$WORK/wprobe-loc.txt"
		hits=$(grep -icE "$shape_re" "$WORK/wprobe-joined.txt" 2>/dev/null || true)
		[ -n "$hits" ] || hits=0
		if [ "$hits" -ge 1 ]; then
			WRAPPED_FIRED=$((WRAPPED_FIRED + 1))
			printf 'PASS  %-7s %-14s wrapped: %s / %s\n' "$slabel" "$shape_id" "$vline1" "$vline2"
		else
			echo
			echo "FAIL  the scanner is wrong, not the probe."
			echo "      shape $shape_id did not fire on $slabel after"
			echo "      planting, across two lines: $vline1 / $vline2"
			exit 1
		fi
	done <<WRAPPED_EOF
$WRAPPED_PROBES
WRAPPED_EOF
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
echo "vocabulary) for 6 forbidden shapes. $PLANTED_FIRED of 18 single-line"
echo "planted violations fired, one per shape per surface, and $WRAPPED_FIRED"
echo "of 12 two-line wrapped violations fired, one per wrap-exposed shape per"
echo "surface. $FACT_PROBES_FIRED of 3 fact probes fired, one per literal"
echo "success criterion 3 names."
echo "tests/ratios.toml was not scanned; see the header comment for why"
echo "(decision D-04)."
echo
echo "The claim surface holds no claim the measurement does not support."
