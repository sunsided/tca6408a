#!/usr/bin/env sh
# Prints the changelog sections of a version for the specified crates.
# Fails if a crate's changelog has no section for the version.
# Usage: release-notes.sh <version> <crate>...
set -eu
version="$1"
shift
for crate in "$@"; do
  echo "## $crate"
  echo
  awk -v heading="## [$version]" '
    index($0, heading) == 1 { found = 1; started = 0; next }
    /^## \[/ { found = 0 }
    found && !started && $0 == "" { next }
    found { started = 1; print }
    END { if (!started) exit 1 }
  ' "$crate/CHANGELOG.md" || {
    echo "release-notes.sh: no changelog section for $version in $crate" >&2
    exit 1
  }
  echo
done
