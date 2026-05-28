//! Corpus round-trip harness for `.reqifz` files.
//!
//! Unlike the `.reqif` harness, byte-level zip equality is NOT the contract
//! (zip metadata varies). The contract is: same .reqif entries in the same
//! order, each round-tripping byte-identically; same attachments with
//! matching names and bytes.
//!
//! Files in `KNOWN_FAILURES` are skipped — add an entry with a one-line
//! reason when a fix would take more than ~30 minutes.

use std::path::{Path, PathBuf};

use reqrs::{FormatMode, ReqIfUnparser, ReqIfzBundle};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_owned()
}

/// Files where round-trip is known not to be byte-identical today. Each
/// entry is the path relative to `tests/corpus/`.
///
/// Baseline established 2026-05-28 alongside the .reqifz harness: 2/3 passed,
/// with the Polarion fixture listed for inline-comment loss. Comment
/// preservation landed shortly after and the list is now empty (3/3).
const KNOWN_FAILURES: &[&str] = &[];

#[test]
fn every_reqifz_in_corpus_round_trips() {
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
        if ext != "reqifz" {
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

        match check_one(path) {
            Ok(()) => passed += 1,
            Err(msg) => failures.push(format!("{rel}: {msg}")),
        }
    }

    let total = passed + failures.len() + skipped;
    println!(
        "reqifz corpus summary: {passed}/{total} passed, {} failed, {skipped} skipped",
        failures.len()
    );

    assert!(
        failures.is_empty(),
        "\n{} of {} reqifz corpus files failed:\n{}\n",
        failures.len(),
        total,
        failures.join("\n")
    );
}

fn check_one(path: &Path) -> Result<(), String> {
    // 1. Read the source bundle.
    let src_bundle = ReqIfzBundle::read(path).map_err(|e| format!("initial read failed: {e}"))?;

    // 2. Verify each inner .reqif round-trips byte-identically through unparse.
    for (name, bundle) in &src_bundle.bundles {
        let original_bytes =
            read_entry_from_zip(path, name).map_err(|e| format!("re-extracting {name}: {e}"))?;
        let original_text = String::from_utf8(original_bytes)
            .map_err(|e| format!("entry {name} not utf-8: {e}"))?;
        let unparsed = ReqIfUnparser::unparse(bundle, FormatMode::Passthrough)
            .map_err(|e| format!("unparsing {name}: {e}"))?;
        if unparsed != original_text {
            return Err(format!("inner entry {name} did not round-trip"));
        }
    }

    // 3. Write through ReqIfzBundle::write and re-read.
    let tmp = tempfile::tempdir().map_err(|e| format!("tempdir: {e}"))?;
    let out_path = tmp.path().join("out.reqifz");
    src_bundle
        .write(&out_path, FormatMode::Passthrough)
        .map_err(|e| format!("write: {e}"))?;
    let reread = ReqIfzBundle::read(&out_path).map_err(|e| format!("re-read: {e}"))?;

    // 4. Compare structure.
    if src_bundle.bundles.len() != reread.bundles.len() {
        return Err(format!(
            "bundle count drift: {} → {}",
            src_bundle.bundles.len(),
            reread.bundles.len()
        ));
    }
    for (i, ((sa, _), (sb, _))) in src_bundle
        .bundles
        .iter()
        .zip(reread.bundles.iter())
        .enumerate()
    {
        if sa != sb {
            return Err(format!("bundle name at index {i} drifted: {sa:?} → {sb:?}"));
        }
    }
    if src_bundle.attachments.len() != reread.attachments.len() {
        return Err(format!(
            "attachment count drift: {} → {}",
            src_bundle.attachments.len(),
            reread.attachments.len()
        ));
    }
    for (i, ((sn, sb), (rn, rb))) in src_bundle
        .attachments
        .iter()
        .zip(reread.attachments.iter())
        .enumerate()
    {
        if sn != rn {
            return Err(format!("attachment name at index {i}: {sn:?} → {rn:?}"));
        }
        if sb != rb {
            return Err(format!(
                "attachment bytes at {sn:?} differ ({} vs {} bytes)",
                sb.len(),
                rb.len()
            ));
        }
    }
    Ok(())
}

fn read_entry_from_zip(path: &Path, entry_name: &str) -> Result<Vec<u8>, String> {
    use std::io::Read;
    let f = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipArchive::new(f).map_err(|e| e.to_string())?;
    let mut entry = zip.by_name(entry_name).map_err(|e| e.to_string())?;
    let mut buf = Vec::with_capacity(entry.size() as usize);
    entry.read_to_end(&mut buf).map_err(|e| e.to_string())?;
    Ok(buf)
}
