//! Unparser for `<SPEC-OBJECT>` elements.
//!
//! Indentation matches the strict-doc-reqif Python reference
//! (`spec_object_parser.py`):
//!
//! - `<SPEC-OBJECT>` at 8 spaces
//! - `<TYPE>` at 10 spaces, `<SPEC-OBJECT-TYPE-REF>` at 12 spaces
//! - `<VALUES>` at 10 spaces (rendered self-closed when the attribute list is
//!   empty), inner `<ATTRIBUTE-VALUE-*>` at 12 spaces — emitted by
//!   [`crate::unparse::attribute_value::unparse_attribute_value`].
//!
//! Outer-tag attributes are alphabetically sorted (DESC, IDENTIFIER,
//! LAST-CHANGE, LONG-NAME). The two children (`<TYPE>` / `<VALUES>`) are
//! emitted in the order captured by [`SpecObjectChildTag`] during parse — most
//! tools emit TYPE first, but some emit VALUES first and round-trip preserves
//! whichever the source used.

use crate::model::spec_object::{SpecObject, SpecObjectChildTag};
use crate::unparse::attribute_value::unparse_attribute_value;
use crate::unparse::writer::{FormatMode, emit_comments_before, write_close, write_open};

const INDENT: &str = "        "; // 8 spaces
const CHILD_INDENT: &str = "          "; // 10 spaces
const REF_INDENT: &str = "            "; // 12 spaces

pub fn unparse_spec_object(so: &SpecObject, mode: FormatMode) -> String {
    let mut out = String::new();
    emit_comments_before(&mut out, INDENT, &so.comments_before);
    let mut attrs = collect_attrs(so);
    write_open(&mut out, INDENT, "SPEC-OBJECT", &mut attrs).expect("writing to String never fails");

    // If no children_order was captured (synthetic SpecObject), default to the
    // Polarion canonical TYPE-then-VALUES ordering. Real parser output always
    // populates both entries.
    let default_order = [SpecObjectChildTag::Type, SpecObjectChildTag::Values];
    let order: &[SpecObjectChildTag] = if so.children_order.is_empty() {
        &default_order
    } else {
        &so.children_order
    };

    for tag in order {
        match tag {
            SpecObjectChildTag::Type => emit_type(&mut out, so),
            SpecObjectChildTag::Values => emit_values(&mut out, so, mode),
        }
    }

    write_close(&mut out, INDENT, "SPEC-OBJECT");
    out
}

fn collect_attrs(so: &SpecObject) -> Vec<(&str, String)> {
    let mut attrs: Vec<(&str, String)> = Vec::with_capacity(4);
    if let Some(d) = &so.description {
        attrs.push(("DESC", d.clone()));
    }
    attrs.push(("IDENTIFIER", so.identifier.as_str().to_owned()));
    if let Some(d) = &so.last_change {
        attrs.push(("LAST-CHANGE", d.clone()));
    }
    if let Some(d) = &so.long_name {
        attrs.push(("LONG-NAME", d.clone()));
    }
    attrs
}

fn emit_type(out: &mut String, so: &SpecObject) {
    out.push_str(CHILD_INDENT);
    out.push_str("<TYPE>\n");
    out.push_str(REF_INDENT);
    out.push_str("<SPEC-OBJECT-TYPE-REF>");
    out.push_str(so.spec_object_type.as_str());
    out.push_str("</SPEC-OBJECT-TYPE-REF>\n");
    out.push_str(CHILD_INDENT);
    out.push_str("</TYPE>\n");
}

fn emit_values(out: &mut String, so: &SpecObject, mode: FormatMode) {
    if so.attributes.is_empty() {
        out.push_str(CHILD_INDENT);
        out.push_str("<VALUES/>\n");
        return;
    }
    out.push_str(CHILD_INDENT);
    out.push_str("<VALUES>\n");
    for av in &so.attributes {
        out.push_str(&unparse_attribute_value(av, mode));
    }
    out.push_str(CHILD_INDENT);
    out.push_str("</VALUES>\n");
}
