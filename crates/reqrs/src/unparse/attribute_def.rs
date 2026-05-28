//! Unparser for `<ATTRIBUTE-DEFINITION-*>` elements.
//!
//! Mirrors the indentation observed in real ReqIF fixtures and matched by the
//! strict-doc-reqif Python reference: outer element at 12 spaces (inside
//! `<SPEC-ATTRIBUTES>` which sits at 10 inside `<SPEC-OBJECT-TYPE>` at 8),
//! `<TYPE>` at 14, the `*-REF` child at 16, and an optional `<DEFAULT-VALUE>`
//! block at 14 (its inner content emitted verbatim from the captured raw bytes).

use crate::model::attribute_def::*;
use crate::unparse::writer::{write_close, write_open, write_self_closing};

const INDENT: &str = "            ";
const TYPE_INDENT: &str = "              ";
const REF_INDENT: &str = "                ";
const DEFAULT_INDENT: &str = "              ";

pub fn unparse_attribute_definition(ad: &AttributeDefinition) -> String {
    match ad {
        AttributeDefinition::String(a) => unparse_one(
            "ATTRIBUTE-DEFINITION-STRING",
            "DATATYPE-DEFINITION-STRING-REF",
            a.identifier.as_str(),
            &a.common,
            a.type_ref.as_str(),
            &a.default_value,
            None,
        ),
        AttributeDefinition::Boolean(a) => unparse_one(
            "ATTRIBUTE-DEFINITION-BOOLEAN",
            "DATATYPE-DEFINITION-BOOLEAN-REF",
            a.identifier.as_str(),
            &a.common,
            a.type_ref.as_str(),
            &a.default_value,
            None,
        ),
        AttributeDefinition::Integer(a) => unparse_one(
            "ATTRIBUTE-DEFINITION-INTEGER",
            "DATATYPE-DEFINITION-INTEGER-REF",
            a.identifier.as_str(),
            &a.common,
            a.type_ref.as_str(),
            &a.default_value,
            None,
        ),
        AttributeDefinition::Real(a) => unparse_one(
            "ATTRIBUTE-DEFINITION-REAL",
            "DATATYPE-DEFINITION-REAL-REF",
            a.identifier.as_str(),
            &a.common,
            a.type_ref.as_str(),
            &a.default_value,
            None,
        ),
        AttributeDefinition::Date(a) => unparse_one(
            "ATTRIBUTE-DEFINITION-DATE",
            "DATATYPE-DEFINITION-DATE-REF",
            a.identifier.as_str(),
            &a.common,
            a.type_ref.as_str(),
            &a.default_value,
            None,
        ),
        AttributeDefinition::Xhtml(a) => unparse_one(
            "ATTRIBUTE-DEFINITION-XHTML",
            "DATATYPE-DEFINITION-XHTML-REF",
            a.identifier.as_str(),
            &a.common,
            a.type_ref.as_str(),
            &a.default_value,
            None,
        ),
        AttributeDefinition::Enumeration(a) => unparse_one(
            "ATTRIBUTE-DEFINITION-ENUMERATION",
            "DATATYPE-DEFINITION-ENUMERATION-REF",
            a.identifier.as_str(),
            &a.common,
            a.type_ref.as_str(),
            &a.default_value,
            a.multi_valued,
        ),
    }
}

fn unparse_one(
    tag: &str,
    ref_tag: &str,
    identifier: &str,
    common: &AttributeDefCommon,
    type_ref: &str,
    default_value: &DefaultValuePresence,
    multi_valued: Option<bool>,
) -> String {
    let mut out = String::new();
    let mut attrs = collect_attrs(identifier, common, multi_valued);

    // `was_self_closing` is preserved for symmetry with the DataType precedent
    // even though the schema requires a <TYPE> child (and the Python reference
    // unparser never emits the self-closing form). Treat presence of children
    // as the actual signal — if the model truly has no children we honour it.
    let has_default = !matches!(default_value, DefaultValuePresence::Absent);
    let truly_self_closed = common.was_self_closing && !has_default && type_ref.is_empty();

    if truly_self_closed {
        write_self_closing(&mut out, INDENT, tag, &mut attrs)
            .expect("writing to String never fails");
        return out;
    }

    write_open(&mut out, INDENT, tag, &mut attrs).expect("writing to String never fails");

    let emit_type = |out: &mut String| {
        out.push_str(TYPE_INDENT);
        out.push_str("<TYPE>\n");
        out.push_str(REF_INDENT);
        out.push('<');
        out.push_str(ref_tag);
        out.push('>');
        out.push_str(type_ref);
        out.push_str("</");
        out.push_str(ref_tag);
        out.push_str(">\n");
        out.push_str(TYPE_INDENT);
        out.push_str("</TYPE>\n");
    };

    let emit_self_closed_default = |out: &mut String| {
        out.push_str(DEFAULT_INDENT);
        out.push_str("<DEFAULT-VALUE/>\n");
    };
    let emit_open_default = |out: &mut String, raw: &DefaultValueRaw| {
        out.push_str(DEFAULT_INDENT);
        out.push_str("<DEFAULT-VALUE>");
        out.push_str(&raw.0);
        out.push_str("</DEFAULT-VALUE>\n");
    };

    // The child-order lives inside the presence variant — `Absent` cannot
    // carry an order, so it falls through to the type-only branch.
    match default_value {
        DefaultValuePresence::Absent => {
            emit_type(&mut out);
        }
        DefaultValuePresence::SelfClosed(ChildOrder::TypeFirst) => {
            emit_type(&mut out);
            emit_self_closed_default(&mut out);
        }
        DefaultValuePresence::SelfClosed(ChildOrder::DefaultFirst) => {
            emit_self_closed_default(&mut out);
            emit_type(&mut out);
        }
        DefaultValuePresence::Open(raw, ChildOrder::TypeFirst) => {
            emit_type(&mut out);
            emit_open_default(&mut out, raw);
        }
        DefaultValuePresence::Open(raw, ChildOrder::DefaultFirst) => {
            emit_open_default(&mut out, raw);
            emit_type(&mut out);
        }
    }

    write_close(&mut out, INDENT, tag);
    out
}

/// Build the attribute list for the outer element. Keys are passed unsorted —
/// `write_open` / `write_self_closing` apply the alphabetic sort that matches
/// the Python reference unparser's output order.
fn collect_attrs<'a>(
    identifier: &'a str,
    common: &'a AttributeDefCommon,
    multi_valued: Option<bool>,
) -> Vec<(&'a str, String)> {
    let mut attrs: Vec<(&str, String)> = Vec::with_capacity(6);
    if let Some(d) = &common.description {
        attrs.push(("DESC", d.clone()));
    }
    attrs.push(("IDENTIFIER", identifier.to_owned()));
    if let Some(b) = common.is_editable {
        attrs.push((
            "IS-EDITABLE",
            if b { "true".into() } else { "false".into() },
        ));
    }
    if let Some(d) = &common.last_change {
        attrs.push(("LAST-CHANGE", d.clone()));
    }
    if let Some(d) = &common.long_name {
        attrs.push(("LONG-NAME", d.clone()));
    }
    if let Some(b) = multi_valued {
        attrs.push((
            "MULTI-VALUED",
            if b { "true".into() } else { "false".into() },
        ));
    }
    attrs
}
