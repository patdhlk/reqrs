//! Unparser for `<SPECIFICATION>` elements.
//!
//! Indentation matches the strict-doc-reqif Python reference
//! (`specification_parser.py`):
//!
//! - `<SPECIFICATION>` at 8 spaces
//! - `<TYPE>` / `<CHILDREN>` / `<VALUES>` at 10 spaces
//! - `<SPECIFICATION-TYPE-REF>` at 12 spaces
//! - inner `<SPEC-HIERARCHY>` nodes start at level 1 (12 spaces) and recurse
//!   via [`crate::unparse::spec_hierarchy::unparse_spec_hierarchy`]
//! - inner `<ATTRIBUTE-VALUE-*>` at 12 spaces — emitted by
//!   [`crate::unparse::attribute_value::unparse_attribute_value`]
//!
//! Outer-tag attributes are alphabetically sorted (DESC, IDENTIFIER,
//! LAST-CHANGE, LONG-NAME). The three children (`<TYPE>`, `<CHILDREN>`,
//! `<VALUES>`) are emitted in the order captured by [`SpecificationChildTag`]
//! during parse — vendors differ on which orderings they emit and round-trip
//! preserves whichever the source used.

use crate::model::specification::{Specification, SpecificationChildTag};
use crate::unparse::attribute_value::unparse_attribute_value;
use crate::unparse::spec_hierarchy::unparse_spec_hierarchy;
use crate::unparse::writer::{write_close, write_open, write_self_closing};

const INDENT: &str = "        "; // 8 spaces
const CHILD_INDENT: &str = "          "; // 10 spaces
const REF_INDENT: &str = "            "; // 12 spaces

pub fn unparse_specification(s: &Specification) -> String {
    let mut out = String::new();
    let mut attrs = collect_attrs(s);

    // Self-closed shape: no children of any kind. Honor it when the source had
    // nothing inside and `children_order` is empty.
    let has_any_child =
        s.specification_type.is_some() || s.children.is_some() || s.values.is_some();
    if !has_any_child && s.children_order.is_empty() {
        write_self_closing(&mut out, INDENT, "SPECIFICATION", &mut attrs)
            .expect("writing to String never fails");
        return out;
    }

    write_open(&mut out, INDENT, "SPECIFICATION", &mut attrs)
        .expect("writing to String never fails");

    // If no children_order was captured (synthetic Specification), default to
    // the canonical TYPE → CHILDREN → VALUES ordering (matching the Python
    // default in `specification_parser.py`).
    let default_order = [
        SpecificationChildTag::Type,
        SpecificationChildTag::Children,
        SpecificationChildTag::Values,
    ];
    let order: &[SpecificationChildTag] = if s.children_order.is_empty() {
        &default_order
    } else {
        &s.children_order
    };

    for tag in order {
        match tag {
            SpecificationChildTag::Type => emit_type(&mut out, s),
            SpecificationChildTag::Children => emit_children(&mut out, s),
            SpecificationChildTag::Values => emit_values(&mut out, s),
        }
    }

    write_close(&mut out, INDENT, "SPECIFICATION");
    out
}

fn collect_attrs(s: &Specification) -> Vec<(&str, String)> {
    let mut attrs: Vec<(&str, String)> = Vec::with_capacity(4);
    if let Some(d) = &s.description {
        attrs.push(("DESC", d.clone()));
    }
    attrs.push(("IDENTIFIER", s.identifier.as_str().to_owned()));
    if let Some(d) = &s.last_change {
        attrs.push(("LAST-CHANGE", d.clone()));
    }
    if let Some(d) = &s.long_name {
        attrs.push(("LONG-NAME", d.clone()));
    }
    attrs
}

fn emit_type(out: &mut String, s: &Specification) {
    let Some(ref_id) = &s.specification_type else {
        return;
    };
    out.push_str(CHILD_INDENT);
    out.push_str("<TYPE>\n");
    out.push_str(REF_INDENT);
    out.push_str("<SPECIFICATION-TYPE-REF>");
    out.push_str(ref_id.as_str());
    out.push_str("</SPECIFICATION-TYPE-REF>\n");
    out.push_str(CHILD_INDENT);
    out.push_str("</TYPE>\n");
}

fn emit_children(out: &mut String, s: &Specification) {
    let Some(children) = &s.children else {
        return;
    };
    if children.is_empty() {
        out.push_str(CHILD_INDENT);
        out.push_str("<CHILDREN/>\n");
        return;
    }
    out.push_str(CHILD_INDENT);
    out.push_str("<CHILDREN>\n");
    for child in children {
        out.push_str(&unparse_spec_hierarchy(child));
    }
    out.push_str(CHILD_INDENT);
    out.push_str("</CHILDREN>\n");
}

fn emit_values(out: &mut String, s: &Specification) {
    let Some(values) = &s.values else {
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
