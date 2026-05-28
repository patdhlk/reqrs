use reqrs::commands::dump::{DumpOpts, dump};

const SAMPLE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<REQ-IF xmlns="http://www.omg.org/spec/ReqIF/20110401/reqif.xsd">
  <THE-HEADER>
    <REQ-IF-HEADER IDENTIFIER="hdr-001">
      <TITLE>Test</TITLE>
    </REQ-IF-HEADER>
  </THE-HEADER>
</REQ-IF>
"#;

#[test]
fn dump_emits_html_with_header_identifier() {
    let dir = tempfile::tempdir().unwrap();
    let in_path = dir.path().join("in.reqif");
    let out_path = dir.path().join("out.html");
    std::fs::write(&in_path, SAMPLE).unwrap();
    dump(DumpOpts {
        input: in_path,
        output: out_path.clone(),
    })
    .unwrap();
    let html = std::fs::read_to_string(&out_path).unwrap();
    assert!(html.starts_with("<!doctype html>"));
    assert!(html.contains("hdr-001"));
    assert!(html.contains("Test"));
}
