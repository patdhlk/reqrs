//! Unparser for `<ATTRIBUTE-VALUE-*>` elements.
//!
//! Mirrors the layout templates from `strict-doc-reqif/reqif/parsers/
//! attribute_value_parser.py` line-for-line:
//! - `<ATTRIBUTE-VALUE-*>` at 12 spaces
//! - `<DEFINITION>` at 14 spaces
//! - `<ATTRIBUTE-DEFINITION-*-REF>` at 16 spaces
//! - `<VALUES>` (enumeration sibling) at 14 spaces
//! - `<ENUM-VALUE-REF>` at 16 spaces
//! - `<THE-VALUE>` (XHTML sibling) at 14 spaces
//!
//! Two templates exist for the heterogeneous-children variants (ENUMERATION
//! and XHTML) because real ReqIF tooling emits both child orderings —
//! `<DEFINITION>` first (the Python `_REVERSE` template) vs the sibling block
//! first (the canonical `_TEMPLATE` constant). Order is selected by the
//! per-variant `was_definition_first` flag captured during parse.

use crate::helpers::xhtml_indent::{indent_16, unindent_16};
use crate::model::attribute_value::*;
use crate::unparse::writer::{FormatMode, emit_comments_before, escape_attr};

const OUTER: &str = "            "; // 12 spaces
const INNER: &str = "              "; // 14 spaces
const REF: &str = "                "; // 16 spaces

/// Emit a single `<ATTRIBUTE-VALUE-*>` element.
///
/// `mode` only affects the XHTML branch today — under
/// [`FormatMode::Canonical`] the captured `the_value_raw` body is reflowed to
/// the Python reference's 16-space margin; under [`FormatMode::Passthrough`]
/// it is emitted verbatim so byte-exact round-trip is preserved. Scalar and
/// enumeration variants emit identical bytes in both modes.
pub fn unparse_attribute_value(av: &AttributeValue, mode: FormatMode) -> String {
    let mut out = String::new();
    emit_comments_before(&mut out, OUTER, av.comments_before());
    match av {
        AttributeValue::String(a) => out.push_str(&unparse_scalar(
            "ATTRIBUTE-VALUE-STRING",
            "ATTRIBUTE-DEFINITION-STRING-REF",
            a.definition_ref.as_str(),
            &a.value,
        )),
        AttributeValue::Boolean(a) => out.push_str(&unparse_scalar(
            "ATTRIBUTE-VALUE-BOOLEAN",
            "ATTRIBUTE-DEFINITION-BOOLEAN-REF",
            a.definition_ref.as_str(),
            if a.value { "true" } else { "false" },
        )),
        AttributeValue::Integer(a) => out.push_str(&unparse_scalar(
            "ATTRIBUTE-VALUE-INTEGER",
            "ATTRIBUTE-DEFINITION-INTEGER-REF",
            a.definition_ref.as_str(),
            &a.value,
        )),
        AttributeValue::Real(a) => out.push_str(&unparse_scalar(
            "ATTRIBUTE-VALUE-REAL",
            "ATTRIBUTE-DEFINITION-REAL-REF",
            a.definition_ref.as_str(),
            &a.value,
        )),
        AttributeValue::Date(a) => out.push_str(&unparse_scalar(
            "ATTRIBUTE-VALUE-DATE",
            "ATTRIBUTE-DEFINITION-DATE-REF",
            a.definition_ref.as_str(),
            &a.value,
        )),
        AttributeValue::Enumeration(a) => out.push_str(&unparse_enumeration(a)),
        AttributeValue::Xhtml(a) => out.push_str(&unparse_xhtml(a, mode)),
    }
    out
}

/// Emit a scalar `<ATTRIBUTE-VALUE-*>` whose value lives in the `THE-VALUE`
/// attribute and whose only child is `<DEFINITION>`. Mirrors the
/// `ATTRIBUTE_STRING_TEMPLATE` family in the Python reference.
fn unparse_scalar(outer_tag: &str, ref_tag: &str, definition_ref: &str, value: &str) -> String {
    let mut out = String::new();
    out.push_str(OUTER);
    out.push('<');
    out.push_str(outer_tag);
    out.push_str(" THE-VALUE=\"");
    escape_attr(&mut out, value);
    out.push_str("\">\n");

    emit_definition(&mut out, ref_tag, definition_ref);

    out.push_str(OUTER);
    out.push_str("</");
    out.push_str(outer_tag);
    out.push_str(">\n");
    out
}

/// Emit `<DEFINITION><ATTRIBUTE-DEFINITION-*-REF>id</…-REF></DEFINITION>`
/// (three lines) at the inner indentation level.
fn emit_definition(out: &mut String, ref_tag: &str, definition_ref: &str) {
    out.push_str(INNER);
    out.push_str("<DEFINITION>\n");
    out.push_str(REF);
    out.push('<');
    out.push_str(ref_tag);
    out.push('>');
    out.push_str(definition_ref);
    out.push_str("</");
    out.push_str(ref_tag);
    out.push_str(">\n");
    out.push_str(INNER);
    out.push_str("</DEFINITION>\n");
}

/// Emit a `<VALUES>` block carrying `<ENUM-VALUE-REF>` children. Empty
/// `values` is rendered as the canonical `<VALUES>...</VALUES>` pair with no
/// inner children (the Python template never emits the self-closed
/// `<VALUES/>` shape inside an enumeration value).
fn emit_values_block(out: &mut String, values: &[crate::ids::EnumValueId]) {
    out.push_str(INNER);
    out.push_str("<VALUES>\n");
    for v in values {
        out.push_str(REF);
        out.push_str("<ENUM-VALUE-REF>");
        out.push_str(v.as_str());
        out.push_str("</ENUM-VALUE-REF>\n");
    }
    out.push_str(INNER);
    out.push_str("</VALUES>\n");
}

fn unparse_enumeration(a: &AttributeValueEnumeration) -> String {
    let mut out = String::new();
    out.push_str(OUTER);
    out.push_str("<ATTRIBUTE-VALUE-ENUMERATION>\n");

    if a.was_definition_first {
        emit_definition(
            &mut out,
            "ATTRIBUTE-DEFINITION-ENUMERATION-REF",
            a.definition_ref.as_str(),
        );
        emit_values_block(&mut out, &a.values);
    } else {
        emit_values_block(&mut out, &a.values);
        emit_definition(
            &mut out,
            "ATTRIBUTE-DEFINITION-ENUMERATION-REF",
            a.definition_ref.as_str(),
        );
    }

    out.push_str(OUTER);
    out.push_str("</ATTRIBUTE-VALUE-ENUMERATION>\n");
    out
}

fn unparse_xhtml(a: &AttributeValueXhtml, mode: FormatMode) -> String {
    let mut out = String::new();
    out.push_str(OUTER);
    out.push_str("<ATTRIBUTE-VALUE-XHTML>\n");

    // Under `Passthrough` we emit the captured bytes verbatim so the corpus
    // round-trip stays byte-exact. Under `Canonical` we reflow through the
    // Python helpers so any source-specific indentation is normalized to the
    // canonical 16-space margin.
    let body = match mode {
        FormatMode::Passthrough => a.the_value_raw.clone(),
        FormatMode::Canonical => indent_16(&unindent_16(&a.the_value_raw)),
    };

    let emit_the_value = |out: &mut String, raw: &str| {
        out.push_str(INNER);
        out.push_str("<THE-VALUE>");
        out.push_str(raw);
        out.push_str("</THE-VALUE>\n");
    };

    if a.was_definition_first {
        emit_definition(
            &mut out,
            "ATTRIBUTE-DEFINITION-XHTML-REF",
            a.definition_ref.as_str(),
        );
        emit_the_value(&mut out, &body);
    } else {
        emit_the_value(&mut out, &body);
        emit_definition(
            &mut out,
            "ATTRIBUTE-DEFINITION-XHTML-REF",
            a.definition_ref.as_str(),
        );
    }

    out.push_str(OUTER);
    out.push_str("</ATTRIBUTE-VALUE-XHTML>\n");
    out
}
