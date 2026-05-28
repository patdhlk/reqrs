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

const SAMPLE_WITH_SPEC: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<REQ-IF xmlns="http://www.omg.org/spec/ReqIF/20110401/reqif.xsd">
  <THE-HEADER>
    <REQ-IF-HEADER IDENTIFIER="hdr-001">
      <TITLE>Doc Title</TITLE>
    </REQ-IF-HEADER>
  </THE-HEADER>
  <CORE-CONTENT>
    <REQ-IF-CONTENT>
      <DATATYPES/>
      <SPEC-TYPES/>
      <SPEC-OBJECTS/>
      <SPECIFICATIONS>
        <SPECIFICATION IDENTIFIER="spec-001" LONG-NAME="Spec Doc">
          <TYPE>
            <SPECIFICATION-TYPE-REF>stype-001</SPECIFICATION-TYPE-REF>
          </TYPE>
        </SPECIFICATION>
      </SPECIFICATIONS>
    </REQ-IF-CONTENT>
  </CORE-CONTENT>
</REQ-IF>
"#;

#[test]
fn anonymize_replaces_specification_long_name() {
    let dir = tempfile::tempdir().unwrap();
    let in_path = dir.path().join("in.reqif");
    let out_path = dir.path().join("out.reqif");
    std::fs::write(&in_path, SAMPLE_WITH_SPEC).unwrap();
    anonymize(AnonymizeOpts {
        input: in_path,
        output: out_path.clone(),
        seed: 42,
    })
    .unwrap();
    let out = std::fs::read_to_string(&out_path).unwrap();
    assert!(
        !out.contains("Spec Doc"),
        "specification LONG-NAME must be anonymized; got:\n{out}"
    );
    assert!(out.contains("Anonymized-"));
}

const SAMPLE_WITH_XHTML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<REQ-IF xmlns="http://www.omg.org/spec/ReqIF/20110401/reqif.xsd" xmlns:xhtml="http://www.w3.org/1999/xhtml">
  <THE-HEADER>
    <REQ-IF-HEADER IDENTIFIER="hdr-001">
      <TITLE>Doc Title</TITLE>
    </REQ-IF-HEADER>
  </THE-HEADER>
  <CORE-CONTENT>
    <REQ-IF-CONTENT>
      <DATATYPES>
        <DATATYPE-DEFINITION-XHTML IDENTIFIER="dt-xhtml" LONG-NAME="Rich Text"/>
      </DATATYPES>
      <SPEC-TYPES>
        <SPEC-OBJECT-TYPE IDENTIFIER="sot-001" LONG-NAME="Requirement">
          <SPEC-ATTRIBUTES>
            <ATTRIBUTE-DEFINITION-XHTML IDENTIFIER="ad-xhtml" LONG-NAME="ReqIF.Text">
              <TYPE>
                <DATATYPE-DEFINITION-XHTML-REF>dt-xhtml</DATATYPE-DEFINITION-XHTML-REF>
              </TYPE>
            </ATTRIBUTE-DEFINITION-XHTML>
          </SPEC-ATTRIBUTES>
        </SPEC-OBJECT-TYPE>
      </SPEC-TYPES>
      <SPEC-OBJECTS>
        <SPEC-OBJECT IDENTIFIER="so-001">
          <VALUES>
            <ATTRIBUTE-VALUE-XHTML>
              <DEFINITION>
                <ATTRIBUTE-DEFINITION-XHTML-REF>ad-xhtml</ATTRIBUTE-DEFINITION-XHTML-REF>
              </DEFINITION>
              <THE-VALUE>
                <xhtml:div>The Lorem Ipsum shall do something.</xhtml:div>
              </THE-VALUE>
            </ATTRIBUTE-VALUE-XHTML>
          </VALUES>
          <TYPE>
            <SPEC-OBJECT-TYPE-REF>sot-001</SPEC-OBJECT-TYPE-REF>
          </TYPE>
        </SPEC-OBJECT>
      </SPEC-OBJECTS>
    </REQ-IF-CONTENT>
  </CORE-CONTENT>
</REQ-IF>
"#;

#[test]
fn anonymize_wraps_xhtml_value_in_xhtml_div() {
    let dir = tempfile::tempdir().unwrap();
    let in_path = dir.path().join("in.reqif");
    let out_path = dir.path().join("out.reqif");
    std::fs::write(&in_path, SAMPLE_WITH_XHTML).unwrap();
    anonymize(AnonymizeOpts {
        input: in_path,
        output: out_path.clone(),
        seed: 7,
    })
    .unwrap();
    let out = std::fs::read_to_string(&out_path).unwrap();
    assert!(
        !out.contains("Lorem Ipsum"),
        "original XHTML content must be replaced; got:\n{out}"
    );
    assert!(
        out.contains("<xhtml:div>Anonymized-"),
        "anonymized XHTML must be wrapped in <xhtml:div>; got:\n{out}"
    );
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
