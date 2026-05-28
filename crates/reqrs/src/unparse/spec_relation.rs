//! Unparser for `<SPEC-RELATION>` elements.
//!
//! Indentation matches the strict-doc-reqif Python reference
//! (`spec_relation_parser.py`):
//!
//! - `<SPEC-RELATION>` at 8 spaces
//! - `<TYPE>` / `<SOURCE>` / `<TARGET>` / `<VALUES>` at 10 spaces
//! - inner refs (`<SPEC-RELATION-TYPE-REF>`, `<SPEC-OBJECT-REF>`) at 12 spaces
//! - inner `<ATTRIBUTE-VALUE-*>` at 12 spaces — emitted by
//!   [`crate::unparse::attribute_value::unparse_attribute_value`]
//!
//! Outer-tag attributes are alphabetically sorted (DESC, IDENTIFIER,
//! LAST-CHANGE, LONG-NAME). Children are emitted in the canonical Python
//! order: TYPE → SOURCE → TARGET → VALUES.

use crate::model::spec_relation::SpecRelation;
use crate::unparse::attribute_value::unparse_attribute_value;
use crate::unparse::writer::{write_close, write_open};

const INDENT: &str = "        "; // 8 spaces
const CHILD_INDENT: &str = "          "; // 10 spaces
const REF_INDENT: &str = "            "; // 12 spaces

pub fn unparse_spec_relation(sr: &SpecRelation) -> String {
    let mut out = String::new();
    let mut attrs = collect_attrs(sr);
    write_open(&mut out, INDENT, "SPEC-RELATION", &mut attrs)
        .expect("writing to String never fails");

    emit_type(&mut out, sr);
    emit_source(&mut out, sr);
    emit_target(&mut out, sr);
    emit_values(&mut out, sr);

    write_close(&mut out, INDENT, "SPEC-RELATION");
    out
}

fn collect_attrs(sr: &SpecRelation) -> Vec<(&str, String)> {
    let mut attrs: Vec<(&str, String)> = Vec::with_capacity(4);
    if let Some(d) = &sr.description {
        attrs.push(("DESC", d.clone()));
    }
    attrs.push(("IDENTIFIER", sr.identifier.as_str().to_owned()));
    if let Some(d) = &sr.last_change {
        attrs.push(("LAST-CHANGE", d.clone()));
    }
    if let Some(d) = &sr.long_name {
        attrs.push(("LONG-NAME", d.clone()));
    }
    attrs
}

fn emit_type(out: &mut String, sr: &SpecRelation) {
    out.push_str(CHILD_INDENT);
    out.push_str("<TYPE>\n");
    out.push_str(REF_INDENT);
    out.push_str("<SPEC-RELATION-TYPE-REF>");
    out.push_str(sr.relation_type.as_str());
    out.push_str("</SPEC-RELATION-TYPE-REF>\n");
    out.push_str(CHILD_INDENT);
    out.push_str("</TYPE>\n");
}

fn emit_source(out: &mut String, sr: &SpecRelation) {
    out.push_str(CHILD_INDENT);
    out.push_str("<SOURCE>\n");
    out.push_str(REF_INDENT);
    out.push_str("<SPEC-OBJECT-REF>");
    out.push_str(sr.source.as_str());
    out.push_str("</SPEC-OBJECT-REF>\n");
    out.push_str(CHILD_INDENT);
    out.push_str("</SOURCE>\n");
}

fn emit_target(out: &mut String, sr: &SpecRelation) {
    out.push_str(CHILD_INDENT);
    out.push_str("<TARGET>\n");
    out.push_str(REF_INDENT);
    out.push_str("<SPEC-OBJECT-REF>");
    out.push_str(sr.target.as_str());
    out.push_str("</SPEC-OBJECT-REF>\n");
    out.push_str(CHILD_INDENT);
    out.push_str("</TARGET>\n");
}

fn emit_values(out: &mut String, sr: &SpecRelation) {
    let Some(values) = &sr.values else {
        return;
    };
    if values.is_empty() {
        out.push_str(CHILD_INDENT);
        out.push_str("<VALUES/>\n");
        return;
    }
    out.push_str(CHILD_INDENT);
    out.push_str("<VALUES>\n");
    for av in values {
        out.push_str(&unparse_attribute_value(av));
    }
    out.push_str(CHILD_INDENT);
    out.push_str("</VALUES>\n");
}
