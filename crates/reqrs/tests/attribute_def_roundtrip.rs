//! Round-trip tests for `<ATTRIBUTE-DEFINITION-*>` variants.
//!
//! Each test feeds a minimal XML fixture through `parse_attribute_definition`
//! then `unparse_attribute_definition` and asserts byte-exact equality.
//! Indentation (12/14/16 spaces) is taken from the strict-doc-reqif fixtures
//! and the Python reference unparser; deviations would surface here.

use pretty_assertions::assert_eq;
use reqrs::parse::attribute_def::parse_attribute_definition;
use reqrs::unparse::attribute_def::unparse_attribute_definition;

fn round_trip(xml: &str) {
    let v = parse_attribute_definition(xml).unwrap();
    let out = unparse_attribute_definition(&v);
    assert_eq!(out, xml, "round-trip mismatch");
}

#[test]
fn string_attribute_definition() {
    round_trip(
        "            <ATTRIBUTE-DEFINITION-STRING IDENTIFIER=\"AD-1\" LONG-NAME=\"Title\">\n              <TYPE>\n                <DATATYPE-DEFINITION-STRING-REF>DT-STR</DATATYPE-DEFINITION-STRING-REF>\n              </TYPE>\n            </ATTRIBUTE-DEFINITION-STRING>\n",
    );
}

#[test]
fn boolean_attribute_definition() {
    round_trip(
        "            <ATTRIBUTE-DEFINITION-BOOLEAN IDENTIFIER=\"AD-B\" LONG-NAME=\"Atomic\">\n              <TYPE>\n                <DATATYPE-DEFINITION-BOOLEAN-REF>DT-BOOL</DATATYPE-DEFINITION-BOOLEAN-REF>\n              </TYPE>\n            </ATTRIBUTE-DEFINITION-BOOLEAN>\n",
    );
}

#[test]
fn integer_attribute_definition() {
    round_trip(
        "            <ATTRIBUTE-DEFINITION-INTEGER IDENTIFIER=\"AD-I\" LONG-NAME=\"Count\">\n              <TYPE>\n                <DATATYPE-DEFINITION-INTEGER-REF>DT-INT</DATATYPE-DEFINITION-INTEGER-REF>\n              </TYPE>\n            </ATTRIBUTE-DEFINITION-INTEGER>\n",
    );
}

#[test]
fn real_attribute_definition_with_is_editable() {
    // Mirrors the IS-EDITABLE shape seen in Polarion fixtures: the attribute
    // appears alphabetically between IDENTIFIER and LAST-CHANGE.
    round_trip(
        "            <ATTRIBUTE-DEFINITION-REAL IDENTIFIER=\"AD-R\" IS-EDITABLE=\"false\" LONG-NAME=\"Pi\">\n              <TYPE>\n                <DATATYPE-DEFINITION-REAL-REF>DT-REAL</DATATYPE-DEFINITION-REAL-REF>\n              </TYPE>\n            </ATTRIBUTE-DEFINITION-REAL>\n",
    );
}

#[test]
fn date_attribute_definition() {
    round_trip(
        "            <ATTRIBUTE-DEFINITION-DATE IDENTIFIER=\"AD-D\" LAST-CHANGE=\"2024-01-02T03:04:05+00:00\" LONG-NAME=\"When\">\n              <TYPE>\n                <DATATYPE-DEFINITION-DATE-REF>DT-DATE</DATATYPE-DEFINITION-DATE-REF>\n              </TYPE>\n            </ATTRIBUTE-DEFINITION-DATE>\n",
    );
}

#[test]
fn xhtml_attribute_definition() {
    round_trip(
        "            <ATTRIBUTE-DEFINITION-XHTML IDENTIFIER=\"AD-X\" LONG-NAME=\"Description\">\n              <TYPE>\n                <DATATYPE-DEFINITION-XHTML-REF>DT-XHTML</DATATYPE-DEFINITION-XHTML-REF>\n              </TYPE>\n            </ATTRIBUTE-DEFINITION-XHTML>\n",
    );
}

#[test]
fn enumeration_attribute_definition_multi_valued() {
    round_trip(
        "            <ATTRIBUTE-DEFINITION-ENUMERATION IDENTIFIER=\"AD-E\" MULTI-VALUED=\"true\">\n              <TYPE>\n                <DATATYPE-DEFINITION-ENUMERATION-REF>DT-E</DATATYPE-DEFINITION-ENUMERATION-REF>\n              </TYPE>\n            </ATTRIBUTE-DEFINITION-ENUMERATION>\n",
    );
}

#[test]
fn string_attribute_definition_with_self_closed_default_value() {
    // <DEFAULT-VALUE/> form — captured as `DefaultValuePresence::SelfClosed`,
    // re-emitted self-closed. The fixture also exercises the
    // `<TYPE>` -> `<DEFAULT-VALUE>` child ordering.
    round_trip(
        "            <ATTRIBUTE-DEFINITION-STRING IDENTIFIER=\"AD-1\" LONG-NAME=\"Title\">\n              <TYPE>\n                <DATATYPE-DEFINITION-STRING-REF>DT-STR</DATATYPE-DEFINITION-STRING-REF>\n              </TYPE>\n              <DEFAULT-VALUE/>\n            </ATTRIBUTE-DEFINITION-STRING>\n",
    );
}

#[test]
fn string_attribute_definition_with_self_closed_default_value_before_type() {
    // Exercises the `SelfClosed(DefaultFirst)` shape — the self-closed
    // `<DEFAULT-VALUE/>` appears before `<TYPE>`. The folded
    // `DefaultValuePresence` carries the order with the presence variant,
    // so this case is now a single irrefutable construction in the model.
    let xml = r#"            <ATTRIBUTE-DEFINITION-STRING IDENTIFIER="AD-2">
              <DEFAULT-VALUE/>
              <TYPE>
                <DATATYPE-DEFINITION-STRING-REF>DT-STR</DATATYPE-DEFINITION-STRING-REF>
              </TYPE>
            </ATTRIBUTE-DEFINITION-STRING>
"#;
    round_trip(xml);
}

#[test]
fn string_attribute_definition_with_verbatim_default_value() {
    // <DEFAULT-VALUE> with non-trivial inner content. The inner bytes
    // (including the leading newline + indentation and the trailing
    // newline + indentation) must round-trip exactly via `DefaultValueRaw`.
    round_trip(
        "            <ATTRIBUTE-DEFINITION-STRING DESC=\"Author of the requirement\" IDENTIFIER=\"AD-1\" LONG-NAME=\"Author\">\n              <DEFAULT-VALUE>\n                <ATTRIBUTE-VALUE-STRING THE-VALUE=\"TBD\"/>\n              </DEFAULT-VALUE>\n              <TYPE>\n                <DATATYPE-DEFINITION-STRING-REF>DT-STR</DATATYPE-DEFINITION-STRING-REF>\n              </TYPE>\n            </ATTRIBUTE-DEFINITION-STRING>\n",
    );
}
