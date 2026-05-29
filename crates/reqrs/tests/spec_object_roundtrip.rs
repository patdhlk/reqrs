//! Round-trip tests for `<SPEC-OBJECT>`.
//!
//! Each test feeds a hand-curated XML fixture through `parse_spec_object` then
//! `unparse_spec_object` and asserts byte-exact equality. Indentation matches
//! the strict-doc-reqif Python reference (`spec_object_parser.py`):
//! `<SPEC-OBJECT>` at 8 spaces, `<TYPE>` / `<VALUES>` at 10,
//! `<SPEC-OBJECT-TYPE-REF>` and `<ATTRIBUTE-VALUE-*>` at 12.

use pretty_assertions::assert_eq;
use reqrs::FormatMode;
use reqrs::model::{AttributeValue, AttributeValueString, SpecObject, SpecObjectChildTag};
use reqrs::parse::spec_object::parse_spec_object;
use reqrs::unparse::spec_object::unparse_spec_object;
use reqrs::{AttributeDefId, SpecObjectId, SpecTypeId};

fn round_trip(xml: &str) {
    let so = parse_spec_object(xml).unwrap();
    let out = unparse_spec_object(&so, FormatMode::Passthrough);
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

#[test]
fn spec_object_with_comments_before_emits_above_element() {
    // The standalone `parse_spec_object` skips events before the first start
    // (so a leading `<!--` outside the element is not visible to it), but a
    // SpecObject value constructed with a non-empty `comments_before` must
    // emit those comments at the SPEC-OBJECT indent level when unparsed.
    let so = SpecObject {
        identifier: SpecObjectId::new("SO-1"),
        description: None,
        last_change: None,
        long_name: None,
        spec_object_type: SpecTypeId::new("ST-1"),
        attributes: vec![],
        children_order: vec![SpecObjectChildTag::Type, SpecObjectChildTag::Values],
        comments_before: vec![" header for SO-1 ".into()],
        values_trailing_comments: vec![],
    };
    let out = unparse_spec_object(&so, FormatMode::Passthrough);
    let expected = r#"        <!-- header for SO-1 -->
        <SPEC-OBJECT IDENTIFIER="SO-1">
          <TYPE>
            <SPEC-OBJECT-TYPE-REF>ST-1</SPEC-OBJECT-TYPE-REF>
          </TYPE>
          <VALUES/>
        </SPEC-OBJECT>
"#;
    assert_eq!(out, expected);
}

#[test]
fn spec_object_with_comment_before_inner_attribute_value_emits_at_12_space_indent() {
    // A comment on an AttributeValue inside `<VALUES>` is emitted at the
    // 12-space indent that matches the element it precedes — mirrors the
    // Polarion fixture's `<!-- Section title (ReqIF.ChapterName) -->` line.
    let av = AttributeValue::String(AttributeValueString {
        definition_ref: AttributeDefId::new("AD-1"),
        value: "Section 1".into(),
        comments_before: vec![" section title ".into()],
    });
    let so = SpecObject {
        identifier: SpecObjectId::new("SO-1"),
        description: None,
        last_change: None,
        long_name: None,
        spec_object_type: SpecTypeId::new("ST-1"),
        attributes: vec![av],
        children_order: vec![SpecObjectChildTag::Type, SpecObjectChildTag::Values],
        comments_before: vec![],
        values_trailing_comments: vec![],
    };
    let out = unparse_spec_object(&so, FormatMode::Passthrough);
    let expected = r#"        <SPEC-OBJECT IDENTIFIER="SO-1">
          <TYPE>
            <SPEC-OBJECT-TYPE-REF>ST-1</SPEC-OBJECT-TYPE-REF>
          </TYPE>
          <VALUES>
            <!-- section title -->
            <ATTRIBUTE-VALUE-STRING THE-VALUE="Section 1">
              <DEFINITION>
                <ATTRIBUTE-DEFINITION-STRING-REF>AD-1</ATTRIBUTE-DEFINITION-STRING-REF>
              </DEFINITION>
            </ATTRIBUTE-VALUE-STRING>
          </VALUES>
        </SPEC-OBJECT>
"#;
    assert_eq!(out, expected);
}

#[test]
fn spec_object_with_comment_between_type_and_values() {
    // An inline `<!-- ... -->` between top-level `<SPEC-OBJECT>` children is
    // captured in `children_order` as a `Comment` entry and re-emitted at the
    // 10-space child indent on the unparse side, preserving the source order
    // verbatim.
    let xml = r#"        <SPEC-OBJECT IDENTIFIER="SO-1" LONG-NAME="x">
          <TYPE>
            <SPEC-OBJECT-TYPE-REF>SOT-1</SPEC-OBJECT-TYPE-REF>
          </TYPE>
          <!-- comment between TYPE and VALUES -->
          <VALUES>
            <ATTRIBUTE-VALUE-STRING THE-VALUE="hello">
              <DEFINITION>
                <ATTRIBUTE-DEFINITION-STRING-REF>AD-1</ATTRIBUTE-DEFINITION-STRING-REF>
              </DEFINITION>
            </ATTRIBUTE-VALUE-STRING>
          </VALUES>
        </SPEC-OBJECT>
"#;
    round_trip(xml);
}
