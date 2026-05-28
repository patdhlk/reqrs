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
