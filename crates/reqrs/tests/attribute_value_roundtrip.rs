//! Round-trip tests for `<ATTRIBUTE-VALUE-*>` variants.
//!
//! Indentation (12/14/16 spaces) is lifted directly from the Python reference
//! templates in `strict-doc-reqif/reqif/parsers/attribute_value_parser.py`.
//! Each test feeds a minimal XML fixture through `parse_attribute_value` then
//! `unparse_attribute_value` and asserts byte-exact equality.

use pretty_assertions::assert_eq;
use reqrs::parse::attribute_value::parse_attribute_value;
use reqrs::unparse::attribute_value::unparse_attribute_value;

fn round_trip(xml: &str) {
    let v = parse_attribute_value(xml).unwrap();
    let out = unparse_attribute_value(&v);
    assert_eq!(out, xml, "round-trip mismatch");
}

#[test]
fn string_value() {
    round_trip(
        "            <ATTRIBUTE-VALUE-STRING THE-VALUE=\"hello\">\n              <DEFINITION>\n                <ATTRIBUTE-DEFINITION-STRING-REF>AD-S</ATTRIBUTE-DEFINITION-STRING-REF>\n              </DEFINITION>\n            </ATTRIBUTE-VALUE-STRING>\n",
    );
}

#[test]
fn string_value_with_escaped_characters() {
    // & and < in the THE-VALUE attribute must round-trip via attribute escaping.
    round_trip(
        "            <ATTRIBUTE-VALUE-STRING THE-VALUE=\"a &amp; b &lt; c\">\n              <DEFINITION>\n                <ATTRIBUTE-DEFINITION-STRING-REF>AD-S</ATTRIBUTE-DEFINITION-STRING-REF>\n              </DEFINITION>\n            </ATTRIBUTE-VALUE-STRING>\n",
    );
}

#[test]
fn integer_value() {
    round_trip(
        "            <ATTRIBUTE-VALUE-INTEGER THE-VALUE=\"42\">\n              <DEFINITION>\n                <ATTRIBUTE-DEFINITION-INTEGER-REF>AD-I</ATTRIBUTE-DEFINITION-INTEGER-REF>\n              </DEFINITION>\n            </ATTRIBUTE-VALUE-INTEGER>\n",
    );
}

#[test]
fn real_value_preserves_trailing_zero() {
    // Real values are stored as text — "1234.50" must not collapse to "1234.5".
    round_trip(
        "            <ATTRIBUTE-VALUE-REAL THE-VALUE=\"1234.50\">\n              <DEFINITION>\n                <ATTRIBUTE-DEFINITION-REAL-REF>AD-R</ATTRIBUTE-DEFINITION-REAL-REF>\n              </DEFINITION>\n            </ATTRIBUTE-VALUE-REAL>\n",
    );
}

#[test]
fn date_value() {
    round_trip(
        "            <ATTRIBUTE-VALUE-DATE THE-VALUE=\"2024-01-02T03:04:05+00:00\">\n              <DEFINITION>\n                <ATTRIBUTE-DEFINITION-DATE-REF>AD-D</ATTRIBUTE-DEFINITION-DATE-REF>\n              </DEFINITION>\n            </ATTRIBUTE-VALUE-DATE>\n",
    );
}

#[test]
fn boolean_value_true() {
    round_trip(
        "            <ATTRIBUTE-VALUE-BOOLEAN THE-VALUE=\"true\">\n              <DEFINITION>\n                <ATTRIBUTE-DEFINITION-BOOLEAN-REF>AD-B</ATTRIBUTE-DEFINITION-BOOLEAN-REF>\n              </DEFINITION>\n            </ATTRIBUTE-VALUE-BOOLEAN>\n",
    );
}

#[test]
fn boolean_value_false() {
    round_trip(
        "            <ATTRIBUTE-VALUE-BOOLEAN THE-VALUE=\"false\">\n              <DEFINITION>\n                <ATTRIBUTE-DEFINITION-BOOLEAN-REF>AD-B</ATTRIBUTE-DEFINITION-BOOLEAN-REF>\n              </DEFINITION>\n            </ATTRIBUTE-VALUE-BOOLEAN>\n",
    );
}

#[test]
fn enumeration_value_with_definition_first() {
    // Matches the Python `ATTRIBUTE_ENUMERATION_TEMPLATE_REVERSE` layout:
    // `<DEFINITION>` before `<VALUES>`. Captured via `was_definition_first=true`.
    let xml = "            <ATTRIBUTE-VALUE-ENUMERATION>\n              <DEFINITION>\n                <ATTRIBUTE-DEFINITION-ENUMERATION-REF>AD-E</ATTRIBUTE-DEFINITION-ENUMERATION-REF>\n              </DEFINITION>\n              <VALUES>\n                <ENUM-VALUE-REF>EV-1</ENUM-VALUE-REF>\n              </VALUES>\n            </ATTRIBUTE-VALUE-ENUMERATION>\n";
    round_trip(xml);
}

#[test]
fn enumeration_value_with_values_first() {
    // Matches the Python `ATTRIBUTE_ENUMERATION_TEMPLATE` layout:
    // `<VALUES>` before `<DEFINITION>`. Captured via `was_definition_first=false`.
    let xml = "            <ATTRIBUTE-VALUE-ENUMERATION>\n              <VALUES>\n                <ENUM-VALUE-REF>EV-1</ENUM-VALUE-REF>\n                <ENUM-VALUE-REF>EV-2</ENUM-VALUE-REF>\n              </VALUES>\n              <DEFINITION>\n                <ATTRIBUTE-DEFINITION-ENUMERATION-REF>AD-E</ATTRIBUTE-DEFINITION-ENUMERATION-REF>\n              </DEFINITION>\n            </ATTRIBUTE-VALUE-ENUMERATION>\n";
    round_trip(xml);
}

#[test]
fn xhtml_value_with_inline_markup() {
    // `<THE-VALUE>` carries inline markup which must round-trip byte-exact
    // via `capture_inner_raw`. Matches the Python `ATTRIBUTE_XHTML_TEMPLATE`
    // layout: `<DEFINITION>` before `<THE-VALUE>`.
    let xml = "            <ATTRIBUTE-VALUE-XHTML>\n              <DEFINITION>\n                <ATTRIBUTE-DEFINITION-XHTML-REF>AD-X</ATTRIBUTE-DEFINITION-XHTML-REF>\n              </DEFINITION>\n              <THE-VALUE>hello <b>bold</b> world</THE-VALUE>\n            </ATTRIBUTE-VALUE-XHTML>\n";
    round_trip(xml);
}

#[test]
fn xhtml_value_with_value_first() {
    // Reversed child order — `<THE-VALUE>` before `<DEFINITION>`. Tracked
    // via `was_definition_first=false`.
    let xml = "            <ATTRIBUTE-VALUE-XHTML>\n              <THE-VALUE>plain text</THE-VALUE>\n              <DEFINITION>\n                <ATTRIBUTE-DEFINITION-XHTML-REF>AD-X</ATTRIBUTE-DEFINITION-XHTML-REF>\n              </DEFINITION>\n            </ATTRIBUTE-VALUE-XHTML>\n";
    round_trip(xml);
}

#[test]
fn xhtml_value_with_namespaced_inline_markup() {
    // Verifies that arbitrary namespaced inline elements survive verbatim
    // — `capture_inner_raw` does not parse the inner content.
    let xml = "            <ATTRIBUTE-VALUE-XHTML>\n              <DEFINITION>\n                <ATTRIBUTE-DEFINITION-XHTML-REF>AD-X</ATTRIBUTE-DEFINITION-XHTML-REF>\n              </DEFINITION>\n              <THE-VALUE><reqif-xhtml:p>para</reqif-xhtml:p></THE-VALUE>\n            </ATTRIBUTE-VALUE-XHTML>\n";
    round_trip(xml);
}
