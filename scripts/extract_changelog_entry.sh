#!/usr/bin/env bash
set -euo pipefail

if [ $# -lt 1 ] || [ $# -gt 2 ]; then
  echo "Usage: $0 <version> [changelog-path]" >&2
  exit 1
fi

VERSION="$1"
CHANGELOG_PATH="${2:-CHANGELOG.md}"

if [ ! -f "$CHANGELOG_PATH" ]; then
  echo "error: changelog not found: $CHANGELOG_PATH" >&2
  exit 1
fi

awk -v version="$VERSION" '
  BEGIN {
    in_section = 0
    found = 0
  }

  $0 ~ "^## \\[" version "\\]" {
    in_section = 1
    found = 1
    next
  }

  in_section && $0 ~ "^## \\[" {
    exit
  }

  in_section {
    print
  }

  END {
    if (!found) {
      exit 2
    }
  }
' "$CHANGELOG_PATH" | sed '/./,$!d'
