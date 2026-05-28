//! Integration tests for `FormatMode::Canonical` XHTML whitespace reflow.
//!
//! These exercise the full parse → unparse pipeline (not just the helper)
//! so they double as a witness that `FormatMode` is correctly threaded all
//! the way down from `ReqIfUnparser::unparse` through `unparse_bundle`,
//! `unparse_spec_object`, `unparse_attribute_value`, and into the XHTML
//! body reflow logic.

use pretty_assertions::assert_eq;
use reqrs::{FormatMode, ReqIfParser, ReqIfUnparser};

const SAMPLE_WEIRD_INDENT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<REQ-IF xmlns="http://www.omg.org/spec/ReqIF/20110401/reqif.xsd" xmlns:xhtml="http://www.w3.org/1999/xhtml">
  <CORE-CONTENT>
    <REQ-IF-CONTENT>
      <SPEC-OBJECTS>
        <SPEC-OBJECT IDENTIFIER="SO-1" LONG-NAME="x">
          <VALUES>
            <ATTRIBUTE-VALUE-XHTML>
              <DEFINITION>
                <ATTRIBUTE-DEFINITION-XHTML-REF>AD-X</ATTRIBUTE-DEFINITION-XHTML-REF>
              </DEFINITION>
              <THE-VALUE>
   <xhtml:p>weird-indent</xhtml:p>
                </THE-VALUE>
            </ATTRIBUTE-VALUE-XHTML>
          </VALUES>
          <TYPE>
            <SPEC-OBJECT-TYPE-REF>SOT-1</SPEC-OBJECT-TYPE-REF>
          </TYPE>
        </SPEC-OBJECT>
      </SPEC-OBJECTS>
    </REQ-IF-CONTENT>
  </CORE-CONTENT>
</REQ-IF>
"#;

#[test]
fn canonical_mode_normalizes_xhtml_indentation() {
    let bundle = ReqIfParser::parse_str(SAMPLE_WEIRD_INDENT).unwrap();
    let passthrough = ReqIfUnparser::unparse(&bundle, FormatMode::Passthrough).unwrap();
    let canonical = ReqIfUnparser::unparse(&bundle, FormatMode::Canonical).unwrap();

    // Passthrough preserves the weird input verbatim.
    assert_eq!(passthrough, SAMPLE_WEIRD_INDENT);

    // Canonical mode canonicalizes the XHTML content's indentation.
    assert!(
        canonical != passthrough,
        "Canonical mode should differ from Passthrough"
    );
    assert!(
        canonical.contains("\n                <xhtml:p>weird-indent</xhtml:p>\n              "),
        "expected canonical 16-space indent, got:\n{canonical}"
    );
}

#[test]
fn passthrough_mode_is_byte_identical_for_canonical_xhtml() {
    let already_canonical: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<REQ-IF xmlns="http://www.omg.org/spec/ReqIF/20110401/reqif.xsd" xmlns:xhtml="http://www.w3.org/1999/xhtml">
  <CORE-CONTENT>
    <REQ-IF-CONTENT>
      <SPEC-OBJECTS>
        <SPEC-OBJECT IDENTIFIER="SO-1" LONG-NAME="x">
          <VALUES>
            <ATTRIBUTE-VALUE-XHTML>
              <DEFINITION>
                <ATTRIBUTE-DEFINITION-XHTML-REF>AD-X</ATTRIBUTE-DEFINITION-XHTML-REF>
              </DEFINITION>
              <THE-VALUE>
                <xhtml:p>aligned</xhtml:p>
              </THE-VALUE>
            </ATTRIBUTE-VALUE-XHTML>
          </VALUES>
          <TYPE>
            <SPEC-OBJECT-TYPE-REF>SOT-1</SPEC-OBJECT-TYPE-REF>
          </TYPE>
        </SPEC-OBJECT>
      </SPEC-OBJECTS>
    </REQ-IF-CONTENT>
  </CORE-CONTENT>
</REQ-IF>
"#;
    let bundle = ReqIfParser::parse_str(already_canonical).unwrap();
    let canonical = ReqIfUnparser::unparse(&bundle, FormatMode::Canonical).unwrap();
    // For an already-canonical input, canonical mode should produce the same output.
    assert_eq!(canonical, already_canonical);
}
