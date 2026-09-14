#!/bin/sh
#
# Prove that the git tag on HEAD matches the workspace version in Cargo.toml.
#
# `cargo fmt`, `cargo clippy` and `cargo test` never look at a git tag. A tag
# that names a version the manifest does not hold, or a release with no tag
# at all, passes those three gate commands silently. This script is the only
# guard on success criterion 5's tag clause.
#
# Run it with an explicit tag:
#
#     sh scripts/check-release-version.sh v1.0.0
#
# Or with no argument, to read the tag from the current commit:
#
#     sh scripts/check-release-version.sh
#
# Or in self test mode, which plants a wrong tag and proves the comparison
# below can report a mismatch, before the release trusts it to report a
# match:
#
#     sh scripts/check-release-version.sh --self-test
#
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$ROOT"

# The single line-anchored `version = ` key in the `[workspace.package]`
# table. Measured at plan time, `grep -c '^version = ' Cargo.toml` prints 1,
# so this line never matches a dependency's own version field.
read_version() {
	awk -F'"' '/^version = /{print $2; exit}' Cargo.toml
}

# Compares a tag against the manifest version, with the tag's leading `v`
# stripped. Prints a PASS or a FAIL line naming both strings, and returns 1
# on a mismatch.
check_tag_against_version() {
	tag=$1
	version=$2
	tag_version=${tag#v}
	if [ "$tag_version" != "$version" ]; then
		echo "FAIL  tag $tag names version $tag_version, Cargo.toml holds $version"
		return 1
	fi
	echo "PASS  tag $tag matches Cargo.toml version $version"
	return 0
}

VERSION=$(read_version)
if [ -z "$VERSION" ]; then
	echo "ERROR: Cargo.toml holds no line-anchored 'version = ' key. Nothing to check."
	exit 1
fi

if [ "${1:-}" = "--self-test" ]; then
	# The probe. It builds a tag from the real version with its last
	# component changed by one, and requires the comparison above to
	# report a mismatch. A script that passed this self test by
	# reporting a match for a wrong tag would never be trusted to report
	# a real mismatch at release time.
	last_component=$(echo "$VERSION" | awk -F. '{print $NF}')
	wrong_last_component=$((last_component + 1))
	wrong_version=$(echo "$VERSION" |
		awk -F. -v last="$wrong_last_component" 'BEGIN { OFS = "." } { $NF = last; print }')
	wrong_tag="v${wrong_version}"

	echo "Self test: planting the wrong tag $wrong_tag against version $VERSION."
	if check_tag_against_version "$wrong_tag" "$VERSION"; then
		echo "FAIL  self test: the comparison reported a match for a wrong tag"
		exit 1
	fi
	echo "PASS  self test: planted $wrong_tag, the comparison correctly reported a mismatch"
	exit 0
fi

if [ $# -ge 1 ]; then
	TAG=$1
else
	if ! TAG=$(git describe --tags --exact-match 2>/dev/null); then
		echo "ERROR: HEAD carries no tag. Give one explicitly, or tag the commit first."
		exit 1
	fi
fi

check_tag_against_version "$TAG" "$VERSION"
