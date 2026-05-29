//! Round-trip tests for `<SPECIFICATION>`.
//!
//! Indentation matches the strict-doc-reqif Python reference
//! (`specification_parser.py`): `<SPECIFICATION>` at 8 spaces, `<TYPE>` /
//! `<CHILDREN>` / `<VALUES>` at 10, inner refs at 12. Spec-hierarchy children
//! enter at `level = 1` so their `<SPEC-HIERARCHY>` opens at 12 spaces.

use pretty_assertions::assert_eq;
use reqrs::SpecificationId;
use reqrs::model::Specification;
use reqrs::parse::specification::parse_specification;
use reqrs::unparse::specification::unparse_specification;
use reqrs::{FormatMode, SpecTypeId};

fn round_trip(xml: &str) {
    let s = parse_specification(xml).unwrap();
    let out = unparse_specification(&s, FormatMode::Passthrough);
    assert_eq!(out, xml, "round-trip mismatch");
}

#[test]
fn specification_canonical_type_then_children() {
    // Canonical order: <TYPE> followed by <CHILDREN>, no <VALUES>. The
    // nested <SPEC-HIERARCHY> is at level 1 (12 spaces).
    let xml = r#"        <SPECIFICATION IDENTIFIER="SPEC-1" LONG-NAME="Doc">
          <TYPE>
            <SPECIFICATION-TYPE-REF>ST-1</SPECIFICATION-TYPE-REF>
          </TYPE>
          <CHILDREN>
            <SPEC-HIERARCHY IDENTIFIER="SH-1">
              <OBJECT>
                <SPEC-OBJECT-REF>SO-1</SPEC-OBJECT-REF>
              </OBJECT>
            </SPEC-HIERARCHY>
          </CHILDREN>
        </SPECIFICATION>
"#;
    round_trip(xml);
}

#[test]
fn specification_with_comment_before_emits_above_element() {
    // The standalone `parse_specification` skips events before the first start
    // (so a leading `<!--` outside the element is not visible to it), but a
    // Specification value constructed with a non-empty `comments_before` must
    // emit those comment lines at the 8-space SPECIFICATION indent
    // immediately above the outer tag.
    let spec = Specification {
        identifier: SpecificationId::new("SPEC-1"),
        description: None,
        last_change: None,
        long_name: Some("Doc".into()),
        specification_type: Some(SpecTypeId::new("ST-1")),
        values: None,
        children: None,
        children_order: vec![],
        children_empty_open_close: false,
        values_empty_open_close: false,
        comments_before: vec![" section header ".into()],
        values_trailing_comments: vec![],
    };
    let out = unparse_specification(&spec, FormatMode::Passthrough);
    let expected = r#"        <!-- section header -->
        <SPECIFICATION IDENTIFIER="SPEC-1" LONG-NAME="Doc">
          <TYPE>
            <SPECIFICATION-TYPE-REF>ST-1</SPECIFICATION-TYPE-REF>
          </TYPE>
        </SPECIFICATION>
"#;
    assert_eq!(out, expected);
}

#[test]
fn specification_with_comment_between_type_and_children() {
    // An inline `<!-- ... -->` between top-level `<SPECIFICATION>` children
    // is captured in `children_order` as a `Comment` entry and re-emitted at
    // the 10-space child indent on the unparse side, preserving the source
    // order verbatim.
    let xml = r#"        <SPECIFICATION IDENTIFIER="SPEC-1" LONG-NAME="Doc">
          <TYPE>
            <SPECIFICATION-TYPE-REF>ST-1</SPECIFICATION-TYPE-REF>
          </TYPE>
          <!-- comment between TYPE and CHILDREN -->
          <CHILDREN>
            <SPEC-HIERARCHY IDENTIFIER="SH-1">
              <OBJECT>
                <SPEC-OBJECT-REF>SO-1</SPEC-OBJECT-REF>
              </OBJECT>
            </SPEC-HIERARCHY>
          </CHILDREN>
        </SPECIFICATION>
"#;
    let s = parse_specification(xml).unwrap();
    let out = unparse_specification(&s, FormatMode::Passthrough);
    assert_eq!(out, xml, "round-trip mismatch");
}

#[test]
fn specification_with_values_first() {
    // Vendor variation: VALUES → TYPE → CHILDREN ordering. children_order
    // captures `[Values, Type, Children]` and the unparser emits in that
    // exact sequence.
    let xml = r#"        <SPECIFICATION DESC="Doc with values" IDENTIFIER="SPEC-2" LAST-CHANGE="2024-01-01T00:00:00Z">
          <VALUES>
            <ATTRIBUTE-VALUE-STRING THE-VALUE="Chapter">
              <DEFINITION>
                <ATTRIBUTE-DEFINITION-STRING-REF>AD-CHAPTER</ATTRIBUTE-DEFINITION-STRING-REF>
              </DEFINITION>
            </ATTRIBUTE-VALUE-STRING>
          </VALUES>
          <TYPE>
            <SPECIFICATION-TYPE-REF>ST-2</SPECIFICATION-TYPE-REF>
          </TYPE>
          <CHILDREN>
            <SPEC-HIERARCHY IDENTIFIER="SH-2">
              <OBJECT>
                <SPEC-OBJECT-REF>SO-2</SPEC-OBJECT-REF>
              </OBJECT>
            </SPEC-HIERARCHY>
          </CHILDREN>
        </SPECIFICATION>
"#;
    round_trip(xml);
}
