//! Round-trip tests for `<SPEC-OBJECT>`.
//!
//! Each test feeds a hand-curated XML fixture through `parse_spec_object` then
//! `unparse_spec_object` and asserts byte-exact equality. Indentation matches
//! the strict-doc-reqif Python reference (`spec_object_parser.py`):
//! `<SPEC-OBJECT>` at 8 spaces, `<TYPE>` / `<VALUES>` at 10,
//! `<SPEC-OBJECT-TYPE-REF>` and `<ATTRIBUTE-VALUE-*>` at 12.

use pretty_assertions::assert_eq;
use reqrs::parse::spec_object::parse_spec_object;
use reqrs::unparse::spec_object::unparse_spec_object;

fn round_trip(xml: &str) {
    let so = parse_spec_object(xml).unwrap();
    let out = unparse_spec_object(&so);
    assert_eq!(out, xml, "round-trip mismatch");
}

#[test]
fn spec_object_type_first_then_values() {
    // Canonical Polarion order: <TYPE> child precedes <VALUES>. children_order
    // captures `[Type, Values]` and the unparser emits in that sequence.
    let xml = r#"        <SPEC-OBJECT IDENTIFIER="SO-1" LAST-CHANGE="2024-01-01T00:00:00Z" LONG-NAME="Req One">
          <TYPE>
            <SPEC-OBJECT-TYPE-REF>ST-1</SPEC-OBJECT-TYPE-REF>
          </TYPE>
          <VALUES>
            <ATTRIBUTE-VALUE-STRING THE-VALUE="Hello">
              <DEFINITION>
                <ATTRIBUTE-DEFINITION-STRING-REF>AD-1</ATTRIBUTE-DEFINITION-STRING-REF>
              </DEFINITION>
            </ATTRIBUTE-VALUE-STRING>
          </VALUES>
        </SPEC-OBJECT>
"#;
    round_trip(xml);
}

#[test]
fn spec_object_values_first_then_type() {
    // Some vendors emit <VALUES> before <TYPE>. children_order captures
    // `[Values, Type]` and the unparser emits in that exact sequence —
    // preserving the source-order signature byte-for-byte.
    let xml = r#"        <SPEC-OBJECT IDENTIFIER="SO-2">
          <VALUES>
            <ATTRIBUTE-VALUE-INTEGER THE-VALUE="42">
              <DEFINITION>
                <ATTRIBUTE-DEFINITION-INTEGER-REF>AD-INT</ATTRIBUTE-DEFINITION-INTEGER-REF>
              </DEFINITION>
            </ATTRIBUTE-VALUE-INTEGER>
          </VALUES>
          <TYPE>
            <SPEC-OBJECT-TYPE-REF>ST-2</SPEC-OBJECT-TYPE-REF>
          </TYPE>
        </SPEC-OBJECT>
"#;
    round_trip(xml);
}

#[test]
fn spec_object_with_multiple_attribute_values() {
    // Exercises the `<VALUES>` list driver with three heterogeneous children
    // (String, Boolean, Date), plus the full set of optional outer attributes
    // (DESC, LAST-CHANGE, LONG-NAME) to confirm alphabetic sorting in the
    // outer tag.
    let xml = r#"        <SPEC-OBJECT DESC="A multi-attribute requirement" IDENTIFIER="SO-3" LAST-CHANGE="2024-02-02T12:00:00+00:00" LONG-NAME="Req Three">
          <TYPE>
            <SPEC-OBJECT-TYPE-REF>ST-3</SPEC-OBJECT-TYPE-REF>
          </TYPE>
          <VALUES>
            <ATTRIBUTE-VALUE-STRING THE-VALUE="Chapter">
              <DEFINITION>
                <ATTRIBUTE-DEFINITION-STRING-REF>AD-CHAPTER</ATTRIBUTE-DEFINITION-STRING-REF>
              </DEFINITION>
            </ATTRIBUTE-VALUE-STRING>
            <ATTRIBUTE-VALUE-BOOLEAN THE-VALUE="true">
              <DEFINITION>
                <ATTRIBUTE-DEFINITION-BOOLEAN-REF>AD-FLAG</ATTRIBUTE-DEFINITION-BOOLEAN-REF>
              </DEFINITION>
            </ATTRIBUTE-VALUE-BOOLEAN>
            <ATTRIBUTE-VALUE-DATE THE-VALUE="2024-02-01">
              <DEFINITION>
                <ATTRIBUTE-DEFINITION-DATE-REF>AD-DUE</ATTRIBUTE-DEFINITION-DATE-REF>
              </DEFINITION>
            </ATTRIBUTE-VALUE-DATE>
          </VALUES>
        </SPEC-OBJECT>
"#;
    round_trip(xml);
}
