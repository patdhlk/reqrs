use reqrs::commands::validate::{ValidateOpts, validate};

#[test]
fn validate_passes_on_minimal_valid_file() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("ok.reqif");
    std::fs::write(
        &p,
        r#"<?xml version="1.0" encoding="UTF-8"?>
<REQ-IF xmlns="http://www.omg.org/spec/ReqIF/20110401/reqif.xsd"/>
"#,
    )
    .unwrap();
    let r = validate(ValidateOpts {
        input: p,
        use_reqif_schema: false,
    })
    .unwrap();
    assert!(r.errors.is_empty(), "{:?}", r.errors);
}

#[test]
fn validate_flags_dangling_spec_object_type_ref() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("bad.reqif");
    std::fs::write(&p, include_str!("fixtures/dangling_type_ref.reqif")).unwrap();
    let r = validate(ValidateOpts {
        input: p,
        use_reqif_schema: false,
    })
    .unwrap();
    assert!(
        r.errors
            .iter()
            .any(|e| e.contains("unknown SPEC-OBJECT-TYPE-REF")),
        "expected dangling ref error, got: {:?}",
        r.errors
    );
}

#[test]
fn validate_flags_duplicate_identifiers() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("dup.reqif");
    std::fs::write(&p, include_str!("fixtures/duplicate_identifiers.reqif")).unwrap();
    let r = validate(ValidateOpts {
        input: p,
        use_reqif_schema: false,
    })
    .unwrap();
    assert!(
        r.errors.iter().any(|e| e.contains("duplicate IDENTIFIER")),
        "expected duplicate IDENTIFIER error, got: {:?}",
        r.errors
    );
}

// Note: we deliberately do NOT include a test that clears `$PATH` to
// exercise the "xmllint missing" branch. Rust's test runner spawns
// integration tests in parallel threads, and `std::env::set_var` is a
// process-global, thread-hostile operation (marked `unsafe` since Rust
// 1.80). Mutating `PATH` here would race with any parallel test that
// spawns a subprocess (none today, but easily added later — silent
// breakage). The error message itself is a string literal, exercised
// only when `xmllint` is genuinely absent from the host. CI images that
// ship without `xmllint` will hit this path naturally; see the
// `--use-reqif-schema` integration test below for the happy-path cover.

#[test]
#[cfg(unix)]
fn schema_validation_runs_xmllint_when_available() {
    // Skip silently when xmllint isn't installed — this test asserts the
    // wiring is correct, not that every CI image has libxml2-utils.
    if std::process::Command::new("xmllint")
        .arg("--version")
        .output()
        .is_err()
    {
        eprintln!("skipping: xmllint not installed");
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("doc.reqif");
    // This minimal stub parses cleanly but is NOT schema-valid (it lacks
    // the required <THE-HEADER> / <CORE-CONTENT> children of <REQ-IF>).
    // We assert the call completes (returns Ok) and that xmllint's
    // complaints land in `report.errors` rather than propagating as an
    // error — that's the contract: schema errors are reportable, not
    // fatal.
    std::fs::write(
        &p,
        r#"<?xml version="1.0" encoding="UTF-8"?>
<REQ-IF xmlns="http://www.omg.org/spec/ReqIF/20110401/reqif.xsd"/>
"#,
    )
    .unwrap();
    let report = validate(ValidateOpts {
        input: p,
        use_reqif_schema: true,
    })
    .expect("schema validation should not propagate xmllint errors as Err");
    assert!(
        !report.errors.is_empty(),
        "minimal stub should fail XSD validation (missing required children); \
         got an empty error list — wiring is wrong"
    );
}
