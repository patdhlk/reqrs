//! Round-trip tests for the four `<SPEC-TYPES>` variants.
//!
//! Each test feeds a hand-curated XML fixture through `parse_spec_type` then
//! `unparse_spec_type` and asserts byte-exact equality. Indentation (8 / 10 /
//! 12 / 14 / 16 spaces) matches the strict-doc-reqif Polarion fixture used by
//! the integration corpus.

use pretty_assertions::assert_eq;
use reqrs::SpecTypeId;
use reqrs::model::{SpecObjectType, SpecType, SpecTypeCommon};
use reqrs::parse::spec_type::parse_spec_type;
use reqrs::unparse::spec_type::unparse_spec_type;

fn round_trip(xml: &str) {
    let v = parse_spec_type(xml).unwrap();
    let out = unparse_spec_type(&v);
    assert_eq!(out, xml, "round-trip mismatch");
}

#[test]
fn spec_object_type_with_attributes() {
    // Mirrors the shape of the Polarion fixture (sample.reqif lines 23-51):
    // outer `<SPEC-OBJECT-TYPE>` at 8 spaces, `<SPEC-ATTRIBUTES>` at 10,
    // `<ATTRIBUTE-DEFINITION-*>` children at 12, `<TYPE>` at 14, `*-REF` at 16.
    let xml = r#"        <SPEC-OBJECT-TYPE IDENTIFIER="ST-1" LAST-CHANGE="2022-02-01T08:22:22.353-08:00" LONG-NAME="Applicable Standard">
          <SPEC-ATTRIBUTES>
            <ATTRIBUTE-DEFINITION-STRING IDENTIFIER="AD-1" IS-EDITABLE="true" LONG-NAME="ReqIF.ChapterName">
              <TYPE>
                <DATATYPE-DEFINITION-STRING-REF>DT-STR</DATATYPE-DEFINITION-STRING-REF>
              </TYPE>
            </ATTRIBUTE-DEFINITION-STRING>
            <ATTRIBUTE-DEFINITION-XHTML IDENTIFIER="AD-2" IS-EDITABLE="true" LONG-NAME="ReqIF.Text">
              <TYPE>
                <DATATYPE-DEFINITION-XHTML-REF>DT-XHTML</DATATYPE-DEFINITION-XHTML-REF>
              </TYPE>
            </ATTRIBUTE-DEFINITION-XHTML>
          </SPEC-ATTRIBUTES>
        </SPEC-OBJECT-TYPE>
"#;
    round_trip(xml);
}

#[test]
fn specification_type_minimal() {
    // The Polarion fixture (line 22) shows the self-closed form for a
    // `<SPECIFICATION-TYPE>` carrying no `<SPEC-ATTRIBUTES>` block — common
    // for tools that don't use specification-level attribute definitions.
    let xml = "        <SPECIFICATION-TYPE IDENTIFIER=\"ST-DOC\" LAST-CHANGE=\"2013-01-01T00:00:00Z\" LONG-NAME=\"Live Document\"/>\n";
    round_trip(xml);
}

#[test]
fn spec_relation_type_with_self_closed_spec_attributes() {
    // Exercises the `Some(vec![])` state: source had `<SPEC-ATTRIBUTES/>`,
    // so the outer element is open/close but the inner block is self-closed.
    // Distinguishes "no block" (None, outer can be self-closed) from "empty
    // block" (Some(vec![]), outer must be open/close).
    let xml = r#"        <SPEC-RELATION-TYPE IDENTIFIER="SRT-1" LONG-NAME="Satisfies">
          <SPEC-ATTRIBUTES/>
        </SPEC-RELATION-TYPE>
"#;
    round_trip(xml);
}

#[test]
fn relation_group_type_minimal() {
    // Mirrors the Python `RelationGroupTypeParser`'s self-closed branch
    // (relation_group_type_parser.py line 62) — IDENTIFIER + LONG-NAME only,
    // no attribute definitions, emitted self-closed.
    let xml = "        <RELATION-GROUP-TYPE IDENTIFIER=\"RGT-1\" LONG-NAME=\"Cluster\"/>\n";
    round_trip(xml);
}

#[test]
fn relation_group_type_with_desc_and_last_change() {
    // Verifies alphabetic attribute ordering when DESC + LAST-CHANGE are
    // present: DESC < IDENTIFIER < LAST-CHANGE < LONG-NAME.
    let xml = "        <RELATION-GROUP-TYPE DESC=\"Cluster of related artefacts\" IDENTIFIER=\"RGT-2\" LAST-CHANGE=\"2024-05-01T12:00:00+00:00\" LONG-NAME=\"Cluster\"/>\n";
    round_trip(xml);
}

#[test]
fn spec_type_with_comments_before_emits_above_element() {
    // A SpecType value with a non-empty `comments_before` must emit those
    // comment lines at the 8-space SPEC-TYPE indent immediately above the
    // outer tag. Mirrors the Polarion fixture's
    // `<!-- "Heading" spec type definition -->` line.
    let st = SpecType::SpecObject(SpecObjectType {
        common: SpecTypeCommon {
            identifier: SpecTypeId::new("ST-1"),
            description: None,
            last_change: None,
            long_name: Some("Heading".into()),
            was_self_closing: true,
            spec_attributes: None,
            comments_before: vec![" Heading spec type ".into()],
        },
    });
    let out = unparse_spec_type(&st);
    let expected = "        <!-- Heading spec type -->\n        <SPEC-OBJECT-TYPE IDENTIFIER=\"ST-1\" LONG-NAME=\"Heading\"/>\n";
    assert_eq!(out, expected);
}
