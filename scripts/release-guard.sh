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

# crate_published CRATE VERSION -> 0 when that version exists on crates.io.
crate_published() {
  local crate=$1 version=$2 code
  code=$(curl -sS -o /dev/null -w '%{http_code}' \
    -H 'User-Agent: reqrs-release-workflow (github.com/patdhlk/reqrs)' \
    "https://crates.io/api/v1/crates/${crate}/${version}")
  [[ "$code" == "200" ]]
}

# emit_publish_flags TAG -> writes publish_lib= / publish_cli= to stdout.
emit_publish_flags() {
  local tag=$1

  if crate_published "reqrs" "$tag"; then
    echo "reqrs $tag is already on crates.io - skipping" >&2
    echo "publish_lib=false"
  else
    echo "publish_lib=true"
  fi

  if crate_published "reqrs-cli" "$tag"; then
    echo "reqrs-cli $tag is already on crates.io - skipping" >&2
    echo "publish_cli=false"
  else
    echo "publish_cli=true"
  fi
}

main() {
  local tag=${1:?usage: release-guard.sh <version>}
  local meta lib cli req

  meta=$(cargo metadata --no-deps --format-version 1)
  lib=$(jq -r '.packages[] | select(.name=="reqrs") | .version' <<<"$meta")
  cli=$(jq -r '.packages[] | select(.name=="reqrs-cli") | .version' <<<"$meta")
  req=$(jq -r '.packages[]
                 | select(.name=="reqrs-cli")
                 | .dependencies[]
                 | select(.name=="reqrs")
                 | .req' <<<"$meta")

  verify_versions "$tag" "$lib" "$cli" "$req"
  echo "tag $tag matches both manifests and the dependency pin" >&2

  echo "version=$tag"
  emit_publish_flags "$tag"
}

# Only run main() when executed directly, so tests can source this file.
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
  main "$@"
fi
