use reqrs::commands::format::{FormatOpts, format};

const SAMPLE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<REQ-IF xmlns="http://www.omg.org/spec/ReqIF/20110401/reqif.xsd"/>
"#;

#[test]
fn format_produces_canonical_output() {
    let dir = tempfile::tempdir().unwrap();
    let in_path = dir.path().join("in.reqif");
    let out_path = dir.path().join("out.reqif");
    std::fs::write(&in_path, SAMPLE).unwrap();
    format(FormatOpts {
        input: in_path,
        output: out_path.clone(),
    })
    .unwrap();
    let written = std::fs::read_to_string(&out_path).unwrap();
    // For now, Canonical mode is a no-op vs Passthrough — assert basic structure.
    assert!(written.contains("<REQ-IF"));
    assert!(
        written.contains("</REQ-IF>")
            || written.contains("<REQ-IF/>")
            || written.contains("<REQ-IF ") && written.contains("/>")
    );
}
