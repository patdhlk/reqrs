//! Unparser for `<SPEC-OBJECT-TYPE>` / `<SPECIFICATION-TYPE>` /
//! `<SPEC-RELATION-TYPE>` / `<RELATION-GROUP-TYPE>`.
//!
//! Indentation matches the strict-doc-reqif Python reference (and the Polarion
//! fixtures used by the integration corpus):
//!
//! - outer element at 8 spaces
//! - `<SPEC-ATTRIBUTES>` at 10 spaces
//! - each `<ATTRIBUTE-DEFINITION-*>` child at 12 spaces — produced directly
//!   by [`crate::unparse::attribute_def::unparse_attribute_definition`] so the
//!   child indentation is consumed verbatim from Task 8.
//!
//! `was_self_closing` is honored only when `spec_attributes` is also `None`
//! (a self-closed element cannot wrap a child block). When the parsed source
//! had `<SPEC-ATTRIBUTES/>` (i.e. `Some(vec![])`), the unparser emits the same
//! self-closed form for byte-exact round-trip.

use crate::model::spec_type::*;
use crate::unparse::attribute_def::unparse_attribute_definition;
use crate::unparse::writer::{write_close, write_open, write_self_closing};

const INDENT: &str = "        ";
const ATTRS_INDENT: &str = "          ";

pub fn unparse_spec_type(st: &SpecType) -> String {
    match st {
        SpecType::SpecObject(t) => unparse_one("SPEC-OBJECT-TYPE", &t.common),
        SpecType::Specification(t) => unparse_one("SPECIFICATION-TYPE", &t.common),
        SpecType::SpecRelation(t) => unparse_one("SPEC-RELATION-TYPE", &t.common),
        SpecType::RelationGroup(t) => unparse_one("RELATION-GROUP-TYPE", &t.common),
    }
}

fn unparse_one(tag: &str, common: &SpecTypeCommon) -> String {
    let mut out = String::new();
    let mut attrs = collect_attrs(common);

    // Self-closed shape is only legal when the source had no <SPEC-ATTRIBUTES>
    // block at all — `Some(vec![])` indicates the source had `<SPEC-ATTRIBUTES/>`,
    // which means the outer element MUST be open/close to wrap the child.
    if common.was_self_closing && common.spec_attributes.is_none() {
        write_self_closing(&mut out, INDENT, tag, &mut attrs)
            .expect("writing to String never fails");
        return out;
    }

    write_open(&mut out, INDENT, tag, &mut attrs).expect("writing to String never fails");

    match &common.spec_attributes {
        None => {}
        Some(ads) if ads.is_empty() => {
            // Empty <SPEC-ATTRIBUTES/> from the source.
            out.push_str(ATTRS_INDENT);
            out.push_str("<SPEC-ATTRIBUTES/>\n");
        }
        Some(ads) => {
            out.push_str(ATTRS_INDENT);
            out.push_str("<SPEC-ATTRIBUTES>\n");
            for ad in ads {
                out.push_str(&unparse_attribute_definition(ad));
            }
            out.push_str(ATTRS_INDENT);
            out.push_str("</SPEC-ATTRIBUTES>\n");
        }
    }

    write_close(&mut out, INDENT, tag);
    out
}

/// Build the outer-element attribute list. Keys are passed unsorted —
/// `write_open` / `write_self_closing` apply the alphabetic sort that matches
/// the Python reference unparser's output order.
fn collect_attrs(common: &SpecTypeCommon) -> Vec<(&str, String)> {
    let mut attrs: Vec<(&str, String)> = Vec::with_capacity(4);
    if let Some(d) = &common.description {
        attrs.push(("DESC", d.clone()));
    }
    attrs.push(("IDENTIFIER", common.identifier.as_str().to_owned()));
    if let Some(d) = &common.last_change {
        attrs.push(("LAST-CHANGE", d.clone()));
    }
    if let Some(d) = &common.long_name {
        attrs.push(("LONG-NAME", d.clone()));
    }
    attrs
}
