#!/usr/bin/env bash
# Verify a release tag's version against the workspace manifests.
#
# Usage: scripts/release-guard.sh <version>     # bare version, e.g. 0.3.0
#
# Writes GITHUB_OUTPUT-style key=value lines to stdout and human-readable
# diagnostics to stderr. Exits non-zero if the tag and manifests disagree.
#
# Run this locally before tagging:  ./scripts/release-guard.sh 0.3.0

set -euo pipefail

# verify_versions TAG LIB CLI REQ -> 0 when coherent, 1 otherwise.
verify_versions() {
  local tag=$1 lib=$2 cli=$3 req=$4
  local rc=0

  if [[ "$lib" != "$tag" ]]; then
    echo "crates/reqrs/Cargo.toml declares version $lib, but the tag says $tag" >&2
    rc=1
  fi
  if [[ "$cli" != "$tag" ]]; then
    echo "crates/reqrs-cli/Cargo.toml declares version $cli, but the tag says $tag" >&2
    rc=1
  fi
  if [[ "$req" != "^$tag" ]]; then
    echo "crates/reqrs-cli/Cargo.toml pins reqrs at '$req', expected '^$tag'" >&2
    rc=1
  fi

  return "$rc"
}

# Only run main() when executed directly, so tests can source this file.
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
  echo "main() not implemented yet" >&2
  exit 1
fi
