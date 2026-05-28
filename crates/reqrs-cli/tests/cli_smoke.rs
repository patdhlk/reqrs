use assert_cmd::Command;

const SAMPLE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<REQ-IF xmlns="http://www.omg.org/spec/ReqIF/20110401/reqif.xsd"/>
"#;

#[test]
fn cli_version_prints_semver() {
    let assert = Command::cargo_bin("reqrs")
        .unwrap()
        .arg("version")
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let first = stdout.chars().next().unwrap();
    assert!(first.is_ascii_digit(), "stdout: {stdout:?}");
    assert!(stdout.contains('.'), "stdout: {stdout:?}");
}

#[test]
fn cli_passthrough_round_trips_minimal_file() {
    let dir = tempfile::tempdir().unwrap();
    let in_path = dir.path().join("in.reqif");
    let out_path = dir.path().join("out.reqif");
    std::fs::write(&in_path, SAMPLE).unwrap();
    Command::cargo_bin("reqrs")
        .unwrap()
        .arg("passthrough")
        .arg(&in_path)
        .arg(&out_path)
        .assert()
        .success();
    let written = std::fs::read_to_string(&out_path).unwrap();
    assert_eq!(written, SAMPLE);
}

#[test]
fn cli_validate_returns_zero_on_valid_minimal() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("ok.reqif");
    std::fs::write(&p, SAMPLE).unwrap();
    Command::cargo_bin("reqrs")
        .unwrap()
        .arg("validate")
        .arg(&p)
        .assert()
        .success();
}

#[test]
fn cli_anonymize_uses_seed_flag() {
    let dir = tempfile::tempdir().unwrap();
    let in_path = dir.path().join("in.reqif");
    let out_path = dir.path().join("out.reqif");
    std::fs::write(
        &in_path,
        r#"<?xml version="1.0" encoding="UTF-8"?>
<REQ-IF xmlns="http://www.omg.org/spec/ReqIF/20110401/reqif.xsd">
  <THE-HEADER><REQ-IF-HEADER IDENTIFIER="h"><TITLE>Secret</TITLE></REQ-IF-HEADER></THE-HEADER>
</REQ-IF>
"#,
    )
    .unwrap();
    Command::cargo_bin("reqrs")
        .unwrap()
        .arg("anonymize")
        .arg("--seed")
        .arg("42")
        .arg(&in_path)
        .arg(&out_path)
        .assert()
        .success();
    let written = std::fs::read_to_string(&out_path).unwrap();
    assert!(!written.contains("Secret"));
    assert!(written.contains("Anonymized-"));
}
