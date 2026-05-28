use reqrs::commands::anonymize::{AnonymizeOpts, anonymize};

const SAMPLE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<REQ-IF xmlns="http://www.omg.org/spec/ReqIF/20110401/reqif.xsd">
  <THE-HEADER>
    <REQ-IF-HEADER IDENTIFIER="hdr-001">
      <TITLE>Secret Title</TITLE>
    </REQ-IF-HEADER>
  </THE-HEADER>
</REQ-IF>
"#;

#[test]
fn anonymize_replaces_user_visible_strings_deterministically() {
    let dir = tempfile::tempdir().unwrap();
    let in_path = dir.path().join("in.reqif");
    let out1 = dir.path().join("out1.reqif");
    let out2 = dir.path().join("out2.reqif");
    std::fs::write(&in_path, SAMPLE).unwrap();
    anonymize(AnonymizeOpts {
        input: in_path.clone(),
        output: out1.clone(),
        seed: 0,
    })
    .unwrap();
    anonymize(AnonymizeOpts {
        input: in_path.clone(),
        output: out2.clone(),
        seed: 0,
    })
    .unwrap();
    let a = std::fs::read_to_string(&out1).unwrap();
    let b = std::fs::read_to_string(&out2).unwrap();
    assert_eq!(a, b, "same seed must produce identical output");
    assert!(!a.contains("Secret Title"));
    assert!(a.contains("Anonymized-"));
}

#[test]
fn anonymize_different_seed_produces_different_output() {
    let dir = tempfile::tempdir().unwrap();
    let in_path = dir.path().join("in.reqif");
    let out1 = dir.path().join("out1.reqif");
    let out2 = dir.path().join("out2.reqif");
    std::fs::write(&in_path, SAMPLE).unwrap();
    anonymize(AnonymizeOpts {
        input: in_path.clone(),
        output: out1.clone(),
        seed: 1,
    })
    .unwrap();
    anonymize(AnonymizeOpts {
        input: in_path.clone(),
        output: out2.clone(),
        seed: 2,
    })
    .unwrap();
    assert_ne!(
        std::fs::read_to_string(&out1).unwrap(),
        std::fs::read_to_string(&out2).unwrap()
    );
}
