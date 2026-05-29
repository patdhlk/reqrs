//! Round-trip tests for `<RELATION-GROUP>`.
//!
//! Indentation matches the strict-doc-reqif Python reference
//! (`relation_group_parser.py`): `<RELATION-GROUP>` at 8 spaces,
//! `<SPEC-RELATIONS>` / `<TYPE>` / `<SOURCE-SPECIFICATION>` /
//! `<TARGET-SPECIFICATION>` at 10, inner refs at 12.
//!
//! Note: the singular tag is `<RELATION-GROUP>` (the plural container is
//! `<SPEC-RELATION-GROUPS>`). The Python parser's assert key is
//! `"RELATION-GROUP" in tag` which matches both — we match `RELATION-GROUP`
//! exactly.

use pretty_assertions::assert_eq;
use reqrs::model::RelationGroup;
use reqrs::parse::relation_group::parse_relation_group;
use reqrs::unparse::relation_group::unparse_relation_group;
use reqrs::{RelationGroupId, SpecTypeId, SpecificationId};

fn round_trip(xml: &str) {
    let rg = parse_relation_group(xml).unwrap();
    let out = unparse_relation_group(&rg);
    assert_eq!(out, xml, "round-trip mismatch");
}

#[test]
fn relation_group_minimal() {
    // No <SPEC-RELATIONS> block. Children emitted in Python canonical order:
    // (SPEC-RELATIONS skipped because absent) → TYPE → SOURCE-SPECIFICATION →
    // TARGET-SPECIFICATION.
    let xml = r#"        <RELATION-GROUP IDENTIFIER="RG-1" LONG-NAME="Trace group">
          <TYPE>
            <RELATION-GROUP-TYPE-REF>RGT-1</RELATION-GROUP-TYPE-REF>
          </TYPE>
          <SOURCE-SPECIFICATION>
            <SPECIFICATION-REF>SPEC-A</SPECIFICATION-REF>
          </SOURCE-SPECIFICATION>
          <TARGET-SPECIFICATION>
            <SPECIFICATION-REF>SPEC-B</SPECIFICATION-REF>
          </TARGET-SPECIFICATION>
        </RELATION-GROUP>
"#;
    round_trip(xml);
}

#[test]
fn relation_group_with_comment_before_emits_above_element() {
    // The standalone `parse_relation_group` skips events before the first
    // start (so a leading `<!--` outside the element is not visible to it),
    // but a RelationGroup value constructed with a non-empty
    // `comments_before` must emit those comment lines at the 8-space
    // RELATION-GROUP indent immediately above the outer tag.
    let rg = RelationGroup {
        identifier: RelationGroupId::new("RG-1"),
        description: None,
        last_change: None,
        long_name: Some("Trace group".into()),
        group_type: SpecTypeId::new("RGT-1"),
        source_specification: SpecificationId::new("SPEC-A"),
        target_specification: SpecificationId::new("SPEC-B"),
        spec_relations: None,
        comments_before: vec![" relation group header ".into()],
    };
    let out = unparse_relation_group(&rg);
    let expected = r#"        <!-- relation group header -->
        <RELATION-GROUP IDENTIFIER="RG-1" LONG-NAME="Trace group">
          <TYPE>
            <RELATION-GROUP-TYPE-REF>RGT-1</RELATION-GROUP-TYPE-REF>
          </TYPE>
          <SOURCE-SPECIFICATION>
            <SPECIFICATION-REF>SPEC-A</SPECIFICATION-REF>
          </SOURCE-SPECIFICATION>
          <TARGET-SPECIFICATION>
            <SPECIFICATION-REF>SPEC-B</SPECIFICATION-REF>
          </TARGET-SPECIFICATION>
        </RELATION-GROUP>
"#;
    assert_eq!(out, expected);
}

#[test]
fn relation_group_with_spec_relations() {
    // Populated SPEC-RELATIONS list with two refs. Outer attributes
    // alphabetically sorted (DESC, IDENTIFIER, LAST-CHANGE, LONG-NAME).
    let xml = r#"        <RELATION-GROUP DESC="A bundled set of relations" IDENTIFIER="RG-2" LAST-CHANGE="2024-03-03T00:00:00Z" LONG-NAME="Group two">
          <SPEC-RELATIONS>
            <SPEC-RELATION-REF>SR-1</SPEC-RELATION-REF>
            <SPEC-RELATION-REF>SR-2</SPEC-RELATION-REF>
          </SPEC-RELATIONS>
          <TYPE>
            <RELATION-GROUP-TYPE-REF>RGT-2</RELATION-GROUP-TYPE-REF>
          </TYPE>
          <SOURCE-SPECIFICATION>
            <SPECIFICATION-REF>SPEC-C</SPECIFICATION-REF>
          </SOURCE-SPECIFICATION>
          <TARGET-SPECIFICATION>
            <SPECIFICATION-REF>SPEC-D</SPECIFICATION-REF>
          </TARGET-SPECIFICATION>
        </RELATION-GROUP>
"#;
    round_trip(xml);
}
