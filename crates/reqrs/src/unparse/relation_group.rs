//! Unparser for `<RELATION-GROUP>` elements.
//!
//! Indentation matches the strict-doc-reqif Python reference
//! (`relation_group_parser.py`):
//!
//! - `<RELATION-GROUP>` at 8 spaces
//! - `<SPEC-RELATIONS>` / `<TYPE>` / `<SOURCE-SPECIFICATION>` /
//!   `<TARGET-SPECIFICATION>` at 10 spaces
//! - inner refs (`<SPEC-RELATION-REF>`, `<RELATION-GROUP-TYPE-REF>`,
//!   `<SPECIFICATION-REF>`) at 12 spaces
//!
//! Outer-tag attributes are alphabetically sorted (DESC, IDENTIFIER,
//! LAST-CHANGE, LONG-NAME). Children are emitted in the Python canonical
//! order: SPEC-RELATIONS → TYPE → SOURCE-SPECIFICATION → TARGET-SPECIFICATION.

use crate::model::relation_group::RelationGroup;
use crate::unparse::writer::{write_close, write_open};

const INDENT: &str = "        "; // 8 spaces
const CHILD_INDENT: &str = "          "; // 10 spaces
const REF_INDENT: &str = "            "; // 12 spaces

pub fn unparse_relation_group(rg: &RelationGroup) -> String {
    let mut out = String::new();
    let mut attrs = collect_attrs(rg);
    write_open(&mut out, INDENT, "RELATION-GROUP", &mut attrs)
        .expect("writing to String never fails");

    emit_spec_relations(&mut out, rg);
    emit_type(&mut out, rg);
    emit_source_specification(&mut out, rg);
    emit_target_specification(&mut out, rg);

    write_close(&mut out, INDENT, "RELATION-GROUP");
    out
}

fn collect_attrs(rg: &RelationGroup) -> Vec<(&str, String)> {
    let mut attrs: Vec<(&str, String)> = Vec::with_capacity(4);
    if let Some(d) = &rg.description {
        attrs.push(("DESC", d.clone()));
    }
    attrs.push(("IDENTIFIER", rg.identifier.as_str().to_owned()));
    if let Some(d) = &rg.last_change {
        attrs.push(("LAST-CHANGE", d.clone()));
    }
    if let Some(d) = &rg.long_name {
        attrs.push(("LONG-NAME", d.clone()));
    }
    attrs
}

fn emit_spec_relations(out: &mut String, rg: &RelationGroup) {
    let Some(list) = &rg.spec_relations else {
        return;
    };
    out.push_str(CHILD_INDENT);
    out.push_str("<SPEC-RELATIONS>\n");
    for id in list {
        out.push_str(REF_INDENT);
        out.push_str("<SPEC-RELATION-REF>");
        out.push_str(id.as_str());
        out.push_str("</SPEC-RELATION-REF>\n");
    }
    out.push_str(CHILD_INDENT);
    out.push_str("</SPEC-RELATIONS>\n");
}

fn emit_type(out: &mut String, rg: &RelationGroup) {
    out.push_str(CHILD_INDENT);
    out.push_str("<TYPE>\n");
    out.push_str(REF_INDENT);
    out.push_str("<RELATION-GROUP-TYPE-REF>");
    out.push_str(rg.group_type.as_str());
    out.push_str("</RELATION-GROUP-TYPE-REF>\n");
    out.push_str(CHILD_INDENT);
    out.push_str("</TYPE>\n");
}

fn emit_source_specification(out: &mut String, rg: &RelationGroup) {
    out.push_str(CHILD_INDENT);
    out.push_str("<SOURCE-SPECIFICATION>\n");
    out.push_str(REF_INDENT);
    out.push_str("<SPECIFICATION-REF>");
    out.push_str(rg.source_specification.as_str());
    out.push_str("</SPECIFICATION-REF>\n");
    out.push_str(CHILD_INDENT);
    out.push_str("</SOURCE-SPECIFICATION>\n");
}

fn emit_target_specification(out: &mut String, rg: &RelationGroup) {
    out.push_str(CHILD_INDENT);
    out.push_str("<TARGET-SPECIFICATION>\n");
    out.push_str(REF_INDENT);
    out.push_str("<SPECIFICATION-REF>");
    out.push_str(rg.target_specification.as_str());
    out.push_str("</SPECIFICATION-REF>\n");
    out.push_str(CHILD_INDENT);
    out.push_str("</TARGET-SPECIFICATION>\n");
}
