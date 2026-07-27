#!/usr/bin/env bats

setup() {
  SCRIPT="${BATS_TEST_DIRNAME}/../release-guard.sh"
  # shellcheck disable=SC1090
  source "$SCRIPT"
}

@test "verify_versions accepts coherent versions" {
  run verify_versions "0.3.0" "0.3.0" "0.3.0" "^0.3.0"
  [ "$status" -eq 0 ]
}

@test "verify_versions rejects lib version mismatch" {
  run verify_versions "0.3.0" "0.2.1" "0.3.0" "^0.3.0"
  [ "$status" -eq 1 ]
  [[ "$output" == *"crates/reqrs/Cargo.toml"* ]]
  [[ "$output" == *"0.2.1"* ]]
}

@test "verify_versions rejects cli version mismatch" {
  run verify_versions "0.3.0" "0.3.0" "0.2.1" "^0.3.0"
  [ "$status" -eq 1 ]
  [[ "$output" == *"crates/reqrs-cli/Cargo.toml"* ]]
}

@test "verify_versions rejects stale dependency pin" {
  run verify_versions "0.3.0" "0.3.0" "0.3.0" "^0.2.1"
  [ "$status" -eq 1 ]
  [[ "$output" == *"pins reqrs"* ]]
}

@test "verify_versions reports every mismatch at once" {
  run verify_versions "0.3.0" "0.1.0" "0.2.0" "^0.1.0"
  [ "$status" -eq 1 ]
  [ "${#lines[@]}" -eq 3 ]
}

@test "verify_versions reproduces the v0.2.0 incident" {
  # The real failure: tag said 0.2.0, both manifests still said 0.1.0.
  run verify_versions "0.2.0" "0.1.0" "0.1.0" "^0.1.0"
  [ "$status" -eq 1 ]
}

@test "emit_publish_flags marks both crates for publishing when absent" {
  crate_published() { return 1; }
  run emit_publish_flags "0.3.0"
  [ "$status" -eq 0 ]
  [[ "$output" == *"publish_lib=true"* ]]
  [[ "$output" == *"publish_cli=true"* ]]
}

@test "emit_publish_flags skips both crates when already published" {
  crate_published() { return 0; }
  run emit_publish_flags "0.2.1"
  [ "$status" -eq 0 ]
  [[ "$output" == *"publish_lib=false"* ]]
  [[ "$output" == *"publish_cli=false"* ]]
}

@test "emit_publish_flags handles a half-published release" {
  # reqrs uploaded, reqrs-cli failed - re-running must finish the job.
  crate_published() { [[ "$1" == "reqrs" ]]; }
  run emit_publish_flags "0.3.0"
  [ "$status" -eq 0 ]
  [[ "$output" == *"publish_lib=false"* ]]
  [[ "$output" == *"publish_cli=true"* ]]
}
