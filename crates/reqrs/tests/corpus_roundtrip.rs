//! Corpus round-trip regression net.
//!
//! Walks every `.reqif` file in `tests/corpus/` and asserts that
//! `parse → unparse` (in `Passthrough` mode) yields a byte-identical
//! output. Files where round-trip fidelity is not yet byte-identical
//! are listed in `KNOWN_FAILURES` along with a brief note on the
//! failure mode — the harness still asserts they are skipped, but
//! does not gate CI on them.
//!
//! The `.reqifz` (zipped) files in the corpus are NOT exercised here
//! — they need their own harness with bundle handling.

use std::path::{Path, PathBuf};

use reqrs::{FormatMode, ReqIfParser, ReqIfUnparser};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_owned()
}

/// Files where round-trip is known not to be byte-identical today.
///
/// Each entry is the path relative to `tests/corpus/`. Add entries
/// here when a fix would take more than ~30 minutes; remove them as
/// they get fixed. Keep the comment terse — it is the only breadcrumb
/// the next session has.
///
/// Baseline established 2026-05-28 (Task 23): 11/23 passed, 12 listed below.
/// Failures cluster into three modes:
///
/// 1. SPEC-RELATION child ordering.
///    FIXED 2026-05-28: SpecRelation.children_order now records the source
///    order of `<TYPE>`, `<SOURCE>`, `<TARGET>`, and `<VALUES>` during parse;
///    the unparser iterates it to preserve vendor-specific orderings
///    (Polarion / ReqIF Studio emit VALUES first; SparxSystems emits SOURCE
///    first). Falls back to canonical TYPE → SOURCE → TARGET → VALUES order
///    when `children_order` is empty (synthetic construction).
///
/// 2. Vendor-specific xmlns attributes on `<REQ-IF>`. The `NamespaceInfo`
///    model only knows the standard set (xmlns, xmlns:xsi, xmlns:configuration,
///    xmlns:id, xmlns:xhtml, xsi:schemaLocation, xml:lang). Doors files add
///    `xmlns:doors`, `xmlns:reqif-common`, etc. — these are dropped. Fix
///    needs an `extra_attributes: Vec<(String, String)>` on `NamespaceInfo`
///    plus parser capture and unparser emit in the original interleaved order.
///    Affects: Doors/01..06, examples/02_read_reqif/input.reqif.
///
/// 3. Empty paired-tag containers vs self-closed. When a file has
///    `<SPEC-RELATIONS>\n      </SPEC-RELATIONS>\n` (empty paired form)
///    we emit `<SPEC-RELATIONS/>\n`. Both are semantically identical.
///    Fix needs an "original form" flag on each `Option<Vec<_>>` container
///    in `ReqIfContent` (or model the empty-paired form explicitly).
///    Affects: Doors/10_capella, examples/04/sample2_sdoc.
const KNOWN_FAILURES: &[&str] = &[
    // Mode 1: SPEC-RELATION child ordering.
    // FIXED 2026-05-28: SpecRelation.children_order captures source order
    // (Type / Source / Target / Values); the unparser replays it.
    // Mode 2: vendor-specific xmlns attributes dropped.
    // FIXED 2026-05-28 (Task 24): NamespaceInfo.attributes_in_order now
    // captures the full attribute list in source order; the unparser walks
    // it to emit byte-exact. Doors/03 and Doors/05 now round-trip cleanly.
    // The remaining five Doors / examples/02 files still fail — but on Mode 3
    // (empty paired-tag SPEC-RELATIONS / SPEC-RELATION-GROUPS), not Mode 2 —
    // and are listed below under Mode 3.
    // Mode 3: empty paired-tag container collapsed to self-closed.
    // FIXED 2026-05-28 (Task 25): ReqIfContent.list_forms now tracks the
    // per-container empty-emission shape; the parser records `<X></X>` form
    // when it sees an Event::Start with no children, and the unparser
    // consults the flag to emit the matching shape (falling back to the
    // self-closed form for synthetic bundles built via Default).
];

#[test]
fn every_reqif_in_corpus_round_trips() {
    let corpus_root = workspace_root().join("tests/corpus");
    assert!(
        corpus_root.exists(),
        "corpus not found at {}",
        corpus_root.display()
    );

    let mut failures: Vec<String> = Vec::new();
    let mut passed = 0usize;
    let mut skipped = 0usize;

    for entry in walkdir::WalkDir::new(&corpus_root)
        .into_iter()
        .filter_map(Result::ok)
    {
        if entry.file_type().is_dir() {
            continue;
        }
        let path = entry.path();
        let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
            continue;
        };
        if ext != "reqif" {
            continue;
        }

        let rel = path
            .strip_prefix(&corpus_root)
            .unwrap()
            .to_string_lossy()
            .into_owned();
        if KNOWN_FAILURES.contains(&rel.as_str()) {
            skipped += 1;
            continue;
        }

        let input = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                failures.push(format!("{rel}: read error: {e}"));
                continue;
            }
        };
        let bundle = match ReqIfParser::parse_str(&input) {
            Ok(b) => b,
            Err(e) => {
                failures.push(format!("{rel}: parse error: {e}"));
                continue;
            }
        };
        let out = match ReqIfUnparser::unparse(&bundle, FormatMode::Passthrough) {
            Ok(o) => o,
            Err(e) => {
                failures.push(format!("{rel}: unparse error: {e}"));
                continue;
            }
        };
        if out != input {
            let preview_diff = first_diff_preview(&input, &out);
            failures.push(format!("{rel}: round-trip mismatch\n  {preview_diff}"));
        } else {
            passed += 1;
        }
    }

    let total = passed + failures.len() + skipped;
    println!(
        "corpus summary: {passed}/{total} passed, {} failed, {skipped} skipped",
        failures.len()
    );

    assert!(
        failures.is_empty(),
        "\n{} of {} corpus files failed round-trip:\n{}\n",
        failures.len(),
        total,
        failures.join("\n")
    );
}

fn first_diff_preview(a: &str, b: &str) -> String {
    for (i, (ca, cb)) in a.chars().zip(b.chars()).enumerate() {
        if ca != cb {
            let start = i.saturating_sub(20);
            let end_a = (i + 40).min(a.len());
            let end_b = (i + 40).min(b.len());
            return format!(
                "first diff at byte {i}: expected ...{:?}... got ...{:?}...",
                &a[start..end_a],
                &b[start..end_b]
            );
        }
    }
    if a.len() != b.len() {
        format!(
            "length differs: input {} bytes, output {} bytes",
            a.len(),
            b.len()
        )
    } else {
        "identical (test bug?)".to_string()
    }
}
