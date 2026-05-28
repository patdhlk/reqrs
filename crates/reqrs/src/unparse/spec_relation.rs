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
//! LAST-CHANGE, LONG-NAME). Children are emitted in the order captured by
//! [`SpecRelationChildTag`] during parse — the canonical order is
//! TYPE → SOURCE → TARGET → VALUES, but Polarion / ReqIF Studio emit VALUES
//! first and SparxSystems emits SOURCE first. When `children_order` is empty
//! (synthetic construction) the unparser falls back to the canonical order.

use crate::model::spec_relation::{SpecRelation, SpecRelationChildTag};
use crate::unparse::attribute_value::unparse_attribute_value;
use crate::unparse::writer::{FormatMode, write_close, write_open};

const INDENT: &str = "        "; // 8 spaces
const CHILD_INDENT: &str = "          "; // 10 spaces
const REF_INDENT: &str = "            "; // 12 spaces

pub fn unparse_spec_relation(sr: &SpecRelation, mode: FormatMode) -> String {
    let mut out = String::new();
    let mut attrs = collect_attrs(sr);
    write_open(&mut out, INDENT, "SPEC-RELATION", &mut attrs)
        .expect("writing to String never fails");

    // If no children_order was captured (synthetic SpecRelation), default to
    // the canonical TYPE → SOURCE → TARGET → VALUES order. Real parser output
    // always populates the sequence in source order.
    let default_order = [
        SpecRelationChildTag::Type,
        SpecRelationChildTag::Source,
        SpecRelationChildTag::Target,
        SpecRelationChildTag::Values,
    ];
    let order: &[SpecRelationChildTag] = if sr.children_order.is_empty() {
        &default_order
    } else {
        &sr.children_order
    };

    for tag in order {
        match tag {
            SpecRelationChildTag::Type => emit_type(&mut out, sr),
            SpecRelationChildTag::Source => emit_source(&mut out, sr),
            SpecRelationChildTag::Target => emit_target(&mut out, sr),
            SpecRelationChildTag::Values => emit_values(&mut out, sr, mode),
        }
    }

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

fn emit_values(out: &mut String, sr: &SpecRelation, mode: FormatMode) {
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
        out.push_str(&unparse_attribute_value(av, mode));
    }
    out.push_str(CHILD_INDENT);
    out.push_str("</VALUES>\n");
}
