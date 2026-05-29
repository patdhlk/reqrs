//! Round-trip tests for `<SPEC-RELATION>`.
//!
//! Indentation matches the strict-doc-reqif Python reference
//! (`spec_relation_parser.py`): `<SPEC-RELATION>` at 8 spaces, `<TYPE>` /
//! `<SOURCE>` / `<TARGET>` / `<VALUES>` at 10, inner refs at 12.

use pretty_assertions::assert_eq;
use reqrs::model::SpecRelation;
use reqrs::parse::spec_relation::parse_spec_relation;
use reqrs::unparse::spec_relation::unparse_spec_relation;
use reqrs::{FormatMode, SpecObjectId, SpecRelationId, SpecTypeId};

fn round_trip(xml: &str) {
    let sr = parse_spec_relation(xml).unwrap();
    let out = unparse_spec_relation(&sr, FormatMode::Passthrough);
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
fn spec_relation_values_first_then_target_source_type() {
    // Polarion / ReqIF Studio style: VALUES → TARGET → SOURCE → TYPE.
    // children_order captures `[Values, Target, Source, Type]` and the
    // unparser emits in that exact sequence — preserving the source-order
    // signature byte-for-byte.
    let xml = r#"        <SPEC-RELATION IDENTIFIER="SR-3">
          <VALUES>
            <ATTRIBUTE-VALUE-STRING THE-VALUE="LNK-2">
              <DEFINITION>
                <ATTRIBUTE-DEFINITION-STRING-REF>AD-REL</ATTRIBUTE-DEFINITION-STRING-REF>
              </DEFINITION>
            </ATTRIBUTE-VALUE-STRING>
          </VALUES>
          <TARGET>
            <SPEC-OBJECT-REF>SO-T</SPEC-OBJECT-REF>
          </TARGET>
          <SOURCE>
            <SPEC-OBJECT-REF>SO-S</SPEC-OBJECT-REF>
          </SOURCE>
          <TYPE>
            <SPEC-RELATION-TYPE-REF>SRT-3</SPEC-RELATION-TYPE-REF>
          </TYPE>
        </SPEC-RELATION>
"#;
    round_trip(xml);
}

#[test]
fn spec_relation_source_first_no_values() {
    // SparxSystems style: SOURCE → TARGET → TYPE, no VALUES block.
    let xml = r#"        <SPEC-RELATION IDENTIFIER="SR-4">
          <SOURCE>
            <SPEC-OBJECT-REF>SO-S</SPEC-OBJECT-REF>
          </SOURCE>
          <TARGET>
            <SPEC-OBJECT-REF>SO-T</SPEC-OBJECT-REF>
          </TARGET>
          <TYPE>
            <SPEC-RELATION-TYPE-REF>SRT-4</SPEC-RELATION-TYPE-REF>
          </TYPE>
        </SPEC-RELATION>
"#;
    round_trip(xml);
}

#[test]
fn spec_relation_with_comment_before_emits_above_element() {
    // The standalone `parse_spec_relation` skips events before the first start
    // (so a leading `<!--` outside the element is not visible to it), but a
    // SpecRelation value constructed with a non-empty `comments_before` must
    // emit those comment lines at the 8-space SPEC-RELATION indent
    // immediately above the outer tag.
    let sr = SpecRelation {
        identifier: SpecRelationId::new("SR-1"),
        description: None,
        last_change: None,
        long_name: None,
        relation_type: SpecTypeId::new("SRT-1"),
        source: SpecObjectId::new("SO-A"),
        target: SpecObjectId::new("SO-B"),
        values: None,
        children_order: vec![],
        comments_before: vec![" relation header ".into()],
    };
    let out = unparse_spec_relation(&sr, FormatMode::Passthrough);
    let expected = r#"        <!-- relation header -->
        <SPEC-RELATION IDENTIFIER="SR-1">
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
    assert_eq!(out, expected);
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
