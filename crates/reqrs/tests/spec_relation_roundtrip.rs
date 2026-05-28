//! Round-trip tests for `<SPEC-RELATION>`.
//!
//! Indentation matches the strict-doc-reqif Python reference
//! (`spec_relation_parser.py`): `<SPEC-RELATION>` at 8 spaces, `<TYPE>` /
//! `<SOURCE>` / `<TARGET>` / `<VALUES>` at 10, inner refs at 12.

use pretty_assertions::assert_eq;
use reqrs::parse::spec_relation::parse_spec_relation;
use reqrs::unparse::spec_relation::unparse_spec_relation;

fn round_trip(xml: &str) {
    let sr = parse_spec_relation(xml).unwrap();
    let out = unparse_spec_relation(&sr);
    assert_eq!(out, xml, "round-trip mismatch");
}

#[test]
fn spec_relation_minimal() {
    // Minimal: TYPE → SOURCE → TARGET, no VALUES.
    let xml = r#"        <SPEC-RELATION IDENTIFIER="SR-1">
          <TYPE>
            <SPEC-RELATION-TYPE-REF>SRT-1</SPEC-RELATION-TYPE-REF>
          </TYPE>
          <SOURCE>
            <SPEC-OBJECT-REF>SO-A</SPEC-OBJECT-REF>
          </SOURCE>
          <TARGET>
            <SPEC-OBJECT-REF>SO-B</SPEC-OBJECT-REF>
          </TARGET>
        </SPEC-RELATION>
"#;
    round_trip(xml);
}

#[test]
fn spec_relation_with_values() {
    // Full set of optional attributes + VALUES block with one string attribute.
    // Outer attributes are alphabetically sorted (DESC, IDENTIFIER, LAST-CHANGE,
    // LONG-NAME).
    let xml = r#"        <SPEC-RELATION DESC="Trace link" IDENTIFIER="SR-2" LAST-CHANGE="2024-02-02T12:00:00+00:00" LONG-NAME="Refines">
          <TYPE>
            <SPEC-RELATION-TYPE-REF>SRT-2</SPEC-RELATION-TYPE-REF>
          </TYPE>
          <SOURCE>
            <SPEC-OBJECT-REF>SO-C</SPEC-OBJECT-REF>
          </SOURCE>
          <TARGET>
            <SPEC-OBJECT-REF>SO-D</SPEC-OBJECT-REF>
          </TARGET>
          <VALUES>
            <ATTRIBUTE-VALUE-STRING THE-VALUE="rationale">
              <DEFINITION>
                <ATTRIBUTE-DEFINITION-STRING-REF>AD-REL</ATTRIBUTE-DEFINITION-STRING-REF>
              </DEFINITION>
            </ATTRIBUTE-VALUE-STRING>
          </VALUES>
        </SPEC-RELATION>
"#;
    round_trip(xml);
}
