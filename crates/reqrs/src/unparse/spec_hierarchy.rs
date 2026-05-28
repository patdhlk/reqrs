//! Unparser for `<SPEC-HIERARCHY>` elements.
//!
//! Indentation matches the strict-doc-reqif Python reference
//! (`spec_hierarchy_parser.py::ReqIFSpecHierarchy.calculate_base_level`):
//!
//! ```text
//! base_level = 12 + (level - 1) * 4
//! ```
//!
//! For a level-1 hierarchy, the outer `<SPEC-HIERARCHY>` opens at 12 spaces;
//! `<OBJECT>` / `<CHILDREN>` siblings sit at `base + 2`; `<SPEC-OBJECT-REF>` at
//! `base + 4`. Nested children recurse with `level + 1`, adding four spaces of
//! indent per depth — matching the canonical Polarion fixture output.
//!
//! Outer-tag attributes are alphabetically sorted (IDENTIFIER, IS-EDITABLE,
//! IS-TABLE-INTERNAL, LAST-CHANGE, LONG-NAME). Sibling order honors
//! `ref_then_children_order`. An empty `<CHILDREN>` is rendered self-closed
//! (`<CHILDREN/>`) only when `was_self_closing_children` was set during parse;
//! otherwise it round-trips as `<CHILDREN></CHILDREN>` (open + close on
//! separate lines, no inner body).

use crate::model::spec_hierarchy::SpecHierarchy;
use crate::unparse::writer::{write_close, write_open};

/// Compute the base indentation width for a hierarchy at `level`. Mirrors
/// `ReqIFSpecHierarchy.calculate_base_level` in the Python reference.
fn base_level(level: usize) -> usize {
    debug_assert!(level > 0, "SpecHierarchy.level must be 1-based");
    12 + (level - 1) * 4
}

pub fn unparse_spec_hierarchy(h: &SpecHierarchy) -> String {
    let mut out = String::new();
    let base = base_level(h.level);
    let base_indent = " ".repeat(base);
    let sibling_indent = " ".repeat(base + 2);
    let ref_indent = " ".repeat(base + 4);

    let mut attrs = collect_attrs(h);
    write_open(&mut out, &base_indent, "SPEC-HIERARCHY", &mut attrs)
        .expect("writing to String never fails");

    if h.ref_then_children_order {
        emit_object(&mut out, h, &sibling_indent, &ref_indent);
        if let Some(children) = &h.children {
            emit_children(&mut out, h, children, &sibling_indent);
        }
    } else {
        if let Some(children) = &h.children {
            emit_children(&mut out, h, children, &sibling_indent);
        }
        emit_object(&mut out, h, &sibling_indent, &ref_indent);
    }

    write_close(&mut out, &base_indent, "SPEC-HIERARCHY");
    out
}

fn collect_attrs(h: &SpecHierarchy) -> Vec<(&str, String)> {
    let mut attrs: Vec<(&str, String)> = Vec::with_capacity(5);
    attrs.push(("IDENTIFIER", h.identifier.clone()));
    if let Some(b) = h.editable {
        attrs.push(("IS-EDITABLE", if b { "true" } else { "false" }.to_owned()));
    }
    if let Some(b) = h.is_table_internal {
        attrs.push((
            "IS-TABLE-INTERNAL",
            if b { "true" } else { "false" }.to_owned(),
        ));
    }
    if let Some(s) = &h.last_change {
        attrs.push(("LAST-CHANGE", s.clone()));
    }
    if let Some(s) = &h.long_name {
        attrs.push(("LONG-NAME", s.clone()));
    }
    attrs
}

fn emit_object(out: &mut String, h: &SpecHierarchy, sibling_indent: &str, ref_indent: &str) {
    out.push_str(sibling_indent);
    out.push_str("<OBJECT>\n");
    out.push_str(ref_indent);
    out.push_str("<SPEC-OBJECT-REF>");
    out.push_str(h.spec_object_ref.as_str());
    out.push_str("</SPEC-OBJECT-REF>\n");
    out.push_str(sibling_indent);
    out.push_str("</OBJECT>\n");
}

fn emit_children(
    out: &mut String,
    h: &SpecHierarchy,
    children: &[SpecHierarchy],
    sibling_indent: &str,
) {
    if children.is_empty() && h.was_self_closing_children {
        out.push_str(sibling_indent);
        out.push_str("<CHILDREN/>\n");
        return;
    }
    out.push_str(sibling_indent);
    out.push_str("<CHILDREN>\n");
    for child in children {
        out.push_str(&unparse_spec_hierarchy(child));
    }
    out.push_str(sibling_indent);
    out.push_str("</CHILDREN>\n");
}
