use reqrs::commands::passthrough::{PassthroughOpts, passthrough};

const SAMPLE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<REQ-IF xmlns="http://www.omg.org/spec/ReqIF/20110401/reqif.xsd"/>
"#;

#[test]
fn passthrough_writes_byte_identical_output() {
    let dir = tempfile::tempdir().unwrap();
    let in_path = dir.path().join("in.reqif");
    let out_path = dir.path().join("out.reqif");
    std::fs::write(&in_path, SAMPLE).unwrap();
    passthrough(PassthroughOpts {
        input: in_path,
        output: out_path.clone(),
    })
    .unwrap();
    let written = std::fs::read_to_string(&out_path).unwrap();
    assert_eq!(written, SAMPLE);
}
