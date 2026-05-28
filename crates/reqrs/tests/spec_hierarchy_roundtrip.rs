//! Round-trip tests for `<SPEC-HIERARCHY>`.
//!
//! Indentation follows the Python reference `calculate_base_level`:
//! `base = 12 + (level - 1) * 4`. At level 1 the outer tag opens at 12 spaces;
//! at level 2 it opens at 16 spaces; OBJECT / CHILDREN siblings sit at
//! `base + 2`, and inner refs at `base + 4`.

use pretty_assertions::assert_eq;
use reqrs::parse::spec_hierarchy::parse_spec_hierarchy;
use reqrs::unparse::spec_hierarchy::unparse_spec_hierarchy;

fn round_trip(xml: &str) {
    let h = parse_spec_hierarchy(xml).unwrap();
    let out = unparse_spec_hierarchy(&h);
    assert_eq!(out, xml, "round-trip mismatch");
}

#[test]
fn spec_hierarchy_with_nested_children() {
    // Level 1 outer at 12 spaces; nested level-2 hierarchy at 16. OBJECT
    // before CHILDREN (`ref_then_children_order = true`).
    let xml = r#"            <SPEC-HIERARCHY IDENTIFIER="SH-1" LONG-NAME="Top">
              <OBJECT>
                <SPEC-OBJECT-REF>SO-1</SPEC-OBJECT-REF>
              </OBJECT>
              <CHILDREN>
                <SPEC-HIERARCHY IDENTIFIER="SH-1-1">
                  <OBJECT>
                    <SPEC-OBJECT-REF>SO-2</SPEC-OBJECT-REF>
                  </OBJECT>
                </SPEC-HIERARCHY>
              </CHILDREN>
            </SPEC-HIERARCHY>
"#;
    round_trip(xml);
}

#[test]
fn spec_hierarchy_with_self_closed_children() {
    // `<CHILDREN/>` self-closed — `was_self_closing_children = true` and
    // `children = Some(vec![])`. Round-trip preserves the self-closed form.
    let xml = r#"            <SPEC-HIERARCHY IDENTIFIER="SH-2">
              <OBJECT>
                <SPEC-OBJECT-REF>SO-3</SPEC-OBJECT-REF>
              </OBJECT>
              <CHILDREN/>
            </SPEC-HIERARCHY>
"#;
    round_trip(xml);
}

#[test]
fn spec_hierarchy_with_children_before_object() {
    // Some vendors emit <CHILDREN> before <OBJECT>. `ref_then_children_order`
    // is `false` and the unparser emits CHILDREN first.
    let xml = r#"            <SPEC-HIERARCHY IDENTIFIER="SH-3">
              <CHILDREN/>
              <OBJECT>
                <SPEC-OBJECT-REF>SO-4</SPEC-OBJECT-REF>
              </OBJECT>
            </SPEC-HIERARCHY>
"#;
    round_trip(xml);
}

#[test]
fn spec_hierarchy_with_editable_and_table_internal() {
    // Exercises both boolean attributes plus alphabetic sort order
    // (IDENTIFIER < IS-EDITABLE < IS-TABLE-INTERNAL < LAST-CHANGE < LONG-NAME).
    let xml = r#"            <SPEC-HIERARCHY IDENTIFIER="SH-4" IS-EDITABLE="true" IS-TABLE-INTERNAL="false" LAST-CHANGE="2024-03-03T00:00:00Z" LONG-NAME="Section">
              <OBJECT>
                <SPEC-OBJECT-REF>SO-5</SPEC-OBJECT-REF>
              </OBJECT>
            </SPEC-HIERARCHY>
"#;
    round_trip(xml);
}
