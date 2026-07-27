# Releasing

Releases are cut by pushing a `v*` tag. `.github/workflows/release.yml` does
the rest.

## Steps

1. Bump the version in **both** manifests and the dependency pin:
   - `crates/reqrs/Cargo.toml` → `version`
   - `crates/reqrs-cli/Cargo.toml` → `version`
   - `crates/reqrs-cli/Cargo.toml` → `reqrs = { path = "../reqrs", version = "X.Y.Z" }`
2. Run `cargo update -w` to refresh `Cargo.lock`.
3. Check coherence locally before tagging:

   ```bash
   ./scripts/release-guard.sh X.Y.Z
   ```

   Exit 0 means the tag will pass CI's guard job.
4. Commit and push to `main`.
5. Tag and push:

   ```bash
   git tag vX.Y.Z && git push origin vX.Y.Z
   ```

## What the workflow does

`guard`, `test`, and `build` run concurrently. Only if all three pass does
`publish` push to crates.io — the one irreversible step. `release` then creates
the GitHub release with binaries and checksums.

## If a release half-fails

If `reqrs` publishes but `reqrs-cli` does not, just re-run the failed workflow.
The guard job detects which crates already exist on crates.io and skips them, so
the re-run completes the release rather than failing on "already uploaded".

## Known gap: binaries are compile-verified, not test-verified

The test suite runs on `ubuntu-latest` only, so the macOS and Windows binaries
are proven to compile but are never exercised by tests. The risk is low — the
code is pure Rust with no platform-specific logic beyond `tempfile` and path
handling — but a platform-specific regression could ship undetected.

Tests are Linux-only because `validate --use-reqif-schema` shells out to
`xmllint`, which has no clean Windows story. If this gap becomes a real problem,
the fix is to run `cargo test --workspace` on the matrix runners with the
`xmllint`-dependent tests gated off — not to drop the binaries.

## Why the Linux builds use musl

The Linux targets are `*-unknown-linux-musl`, not `-gnu`. A gnu binary built on
ubuntu-24.04 links glibc 2.39 and will not run on older distributions; musl links
statically and runs anywhere.

This works with no extra toolchain because the dependency graph contains no C
code — `zip` is configured `default-features = false, features = ["deflate"]`, so
compression resolves to `flate2` → `miniz_oxide` rather than a C zlib. Verified
in a `rust:latest` container with no `musl-gcc` present: the build succeeds and
produces a `static-pie linked` ELF. If a future dependency introduces C code, the
build will fail loudly at link time.

## Version history quirk

`v0.2.0` exists as a GitHub tag and release but was **never published to
crates.io** — it was tagged without bumping the manifests, which still declared
`0.1.0`. crates.io therefore goes `0.1.0` → `0.2.1` with no `0.2.0`. This is
intentional and is not a yank. Do not try to "restore" it.

The `guard` job exists specifically so this cannot happen again, and
`scripts/tests/release-guard.bats` carries a regression test replaying it.

## Token

The workflow authenticates with the `CARGO_REGISTRY_TOKEN` repository secret.
It must be created on crates.io with `publish-update` scope only, restricted to
the `reqrs` and `reqrs-cli` crates.
