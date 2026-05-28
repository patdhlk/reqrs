//! Parser for `<ATTRIBUTE-DEFINITION-*>` elements.
//!
//! Mirrors `parse::data_type` in shape: the public `parse_attribute_definition`
//! entry locates the start event then defers to the `pub(crate)` inner routine
//! `parse_attribute_definition_inner`, which is the function the future
//! `<SPEC-ATTRIBUTES>` list driver (Task 11) will call directly.

use crate::error::ReqIfError;
use crate::ids::{AttributeDefId, DataTypeId};
use crate::model::attribute_def::*;
use crate::parse::reader::{ReqIfReader, optional_attr, required_attr};
use quick_xml::events::{BytesStart, Event};

/// Standalone entry point — typically used by integration tests and
/// list-driver code. Scans for the first `<ATTRIBUTE-DEFINITION-*>` start
/// event, then defers to `parse_attribute_definition_inner`.
pub fn parse_attribute_definition(xml: &str) -> Result<AttributeDefinition, ReqIfError> {
    let mut r = ReqIfReader::new(xml.as_bytes());
    loop {
        match r.read_event()? {
            Event::Start(s) => {
                let tag = s.name().as_ref().to_vec();
                let owned = s.into_owned();
                return parse_attribute_definition_inner(&mut r, &owned, &tag, false);
            }
            Event::Empty(s) => {
                let tag = s.name().as_ref().to_vec();
                let owned = s.into_owned();
                return parse_attribute_definition_inner(&mut r, &owned, &tag, true);
            }
            Event::Eof => {
                return Err(ReqIfError::MissingChild {
                    child: "ATTRIBUTE-DEFINITION-*".into(),
                    parent: "<root>".into(),
                });
            }
            _ => continue,
        }
    }
}

/// Inner parser called once the caller has identified the start event of an
/// `<ATTRIBUTE-DEFINITION-*>` element. The `tag` slice is the element name as
/// raw bytes; this routine dispatches off it for the variant + reads the
/// required `<TYPE>` child and an optional verbatim `<DEFAULT-VALUE>` block.
pub(crate) fn parse_attribute_definition_inner(
    r: &mut ReqIfReader<'_>,
    start: &BytesStart<'_>,
    tag: &[u8],
    was_self_closing: bool,
) -> Result<AttributeDefinition, ReqIfError> {
    let identifier = AttributeDefId(required_attr(start, "IDENTIFIER")?);
    let common = AttributeDefCommon {
        description: optional_attr(start, "DESC"),
        last_change: optional_attr(start, "LAST-CHANGE"),
        long_name: optional_attr(start, "LONG-NAME"),
        is_editable: optional_attr(start, "IS-EDITABLE").map(|s| s == "true"),
        was_self_closing,
    };

    // Self-closed form would have no <TYPE> child, which is a schema violation.
    if was_self_closing {
        return Err(ReqIfError::MissingChild {
            child: "TYPE".into(),
            parent: String::from_utf8_lossy(tag).into_owned(),
        });
    }

    // Identify variant + the matching <DATATYPE-DEFINITION-*-REF> child tag inside <TYPE>.
    let (variant, ref_child) = variant_for_tag(tag)?;

    // Pull the optional enumeration-only attribute up front so it's available
    // when constructing the variant after the children are read.
    let multi_valued = if variant == Variant::Enumeration {
        optional_attr(start, "MULTI-VALUED").map(|s| s == "true")
    } else {
        None
    };

    // Walk children in source order so we can preserve <TYPE> vs <DEFAULT-VALUE> ordering.
    // Variant of `<DEFAULT-VALUE>` (self-closed vs open with raw payload) is captured first;
    // the corresponding `ChildOrder` is decided from `seen_type_first` at the moment the
    // `<DEFAULT-VALUE>` event is observed and folded into the presence variant.
    enum RawPresence {
        Absent,
        SelfClosed,
        Open(String),
    }
    let mut type_ref: Option<String> = None;
    let mut raw_presence = RawPresence::Absent;
    let mut default_order: Option<ChildOrder> = None;
    let mut seen_type_first: Option<bool> = None;

    loop {
        match r.read_event()? {
            Event::Start(s) if s.name().as_ref() == b"TYPE" => {
                if seen_type_first.is_none() {
                    seen_type_first = Some(true);
                }
                type_ref = Some(parse_type_child(r, ref_child)?);
            }
            Event::Empty(s) if s.name().as_ref() == b"TYPE" => {
                // <TYPE/> with no inner ref — schema violation.
                let _ = s;
                return Err(ReqIfError::MissingChild {
                    child: String::from_utf8_lossy(ref_child).into_owned(),
                    parent: "TYPE".into(),
                });
            }
            Event::Start(s) if s.name().as_ref() == b"DEFAULT-VALUE" => {
                let owned = s.into_owned();
                if seen_type_first.is_none() {
                    seen_type_first = Some(false);
                }
                default_order = Some(child_order_from(seen_type_first));
                let raw = r.capture_inner_raw(&owned)?;
                raw_presence = RawPresence::Open(raw);
            }
            Event::Empty(s) if s.name().as_ref() == b"DEFAULT-VALUE" => {
                let _ = s;
                if seen_type_first.is_none() {
                    seen_type_first = Some(false);
                }
                default_order = Some(child_order_from(seen_type_first));
                raw_presence = RawPresence::SelfClosed;
            }
            Event::End(e) if e.name().as_ref() == tag => break,
            Event::Eof => {
                return Err(ReqIfError::Xml {
                    pos: r.buffer_position(),
                    msg: format!("EOF inside <{}>", String::from_utf8_lossy(tag)),
                });
            }
            _ => continue,
        }
    }

    let default_value = match raw_presence {
        RawPresence::Absent => DefaultValuePresence::Absent,
        RawPresence::SelfClosed => DefaultValuePresence::SelfClosed(
            default_order.expect("order must be set when default-value was observed"),
        ),
        RawPresence::Open(raw) => DefaultValuePresence::Open(
            DefaultValueRaw(raw),
            default_order.expect("order must be set when default-value was observed"),
        ),
    };

    let type_ref = DataTypeId(type_ref.ok_or(ReqIfError::MissingChild {
        child: "TYPE".into(),
        parent: String::from_utf8_lossy(tag).into_owned(),
    })?);

    Ok(build_variant(
        variant,
        identifier,
        common,
        type_ref,
        default_value,
        multi_valued,
    ))
}

/// Translate the `seen_type_first` tristate into a `ChildOrder` at the moment a
/// `<DEFAULT-VALUE>` child is observed.
fn child_order_from(seen_type_first: Option<bool>) -> ChildOrder {
    match seen_type_first {
        Some(true) => ChildOrder::TypeFirst,
        Some(false) | None => ChildOrder::DefaultFirst,
    }
}

/// Walk the body of a `<TYPE>` element and return the text content of its
/// `<DATATYPE-DEFINITION-*-REF>` child. The exact ref-child tag depends on the
/// outer variant and is supplied by the caller.
fn parse_type_child(r: &mut ReqIfReader<'_>, ref_child: &[u8]) -> Result<String, ReqIfError> {
    let mut value: Option<String> = None;
    loop {
        match r.read_event()? {
            Event::Start(s) if s.name().as_ref() == ref_child => {
                let end = s.to_end().into_owned();
                value = Some(r.read_text_to_end(&end)?);
            }
            Event::Empty(s) if s.name().as_ref() == ref_child => {
                // `<*-REF/>` — schema allows empty text content; surface as empty id.
                let _ = s;
                value = Some(String::new());
            }
            Event::End(e) if e.name().as_ref() == b"TYPE" => {
                return value.ok_or(ReqIfError::MissingChild {
                    child: String::from_utf8_lossy(ref_child).into_owned(),
                    parent: "TYPE".into(),
                });
            }
            Event::Eof => {
                return Err(ReqIfError::Xml {
                    pos: r.buffer_position(),
                    msg: "EOF inside <TYPE>".into(),
                });
            }
            _ => continue,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Variant {
    String,
    Boolean,
    Integer,
    Real,
    Date,
    Xhtml,
    Enumeration,
}

/// Map an outer element tag to its variant plus the matching ref-child tag inside `<TYPE>`.
fn variant_for_tag(tag: &[u8]) -> Result<(Variant, &'static [u8]), ReqIfError> {
    Ok(match tag {
        b"ATTRIBUTE-DEFINITION-STRING" => (Variant::String, b"DATATYPE-DEFINITION-STRING-REF"),
        b"ATTRIBUTE-DEFINITION-BOOLEAN" => (Variant::Boolean, b"DATATYPE-DEFINITION-BOOLEAN-REF"),
        b"ATTRIBUTE-DEFINITION-INTEGER" => (Variant::Integer, b"DATATYPE-DEFINITION-INTEGER-REF"),
        b"ATTRIBUTE-DEFINITION-REAL" => (Variant::Real, b"DATATYPE-DEFINITION-REAL-REF"),
        b"ATTRIBUTE-DEFINITION-DATE" => (Variant::Date, b"DATATYPE-DEFINITION-DATE-REF"),
        b"ATTRIBUTE-DEFINITION-XHTML" => (Variant::Xhtml, b"DATATYPE-DEFINITION-XHTML-REF"),
        b"ATTRIBUTE-DEFINITION-ENUMERATION" => {
            (Variant::Enumeration, b"DATATYPE-DEFINITION-ENUMERATION-REF")
        }
        _ => {
            return Err(ReqIfError::UnexpectedTag {
                tag: String::from_utf8_lossy(tag).into_owned(),
                parent: "SPEC-ATTRIBUTES".into(),
            });
        }
    })
}

fn build_variant(
    variant: Variant,
    identifier: AttributeDefId,
    common: AttributeDefCommon,
    type_ref: DataTypeId,
    default_value: DefaultValuePresence,
    multi_valued: Option<bool>,
) -> AttributeDefinition {
    match variant {
        Variant::String => AttributeDefinition::String(AttributeDefinitionString {
            identifier,
            common,
            type_ref,
            default_value,
        }),
        Variant::Boolean => AttributeDefinition::Boolean(AttributeDefinitionBoolean {
            identifier,
            common,
            type_ref,
            default_value,
        }),
        Variant::Integer => AttributeDefinition::Integer(AttributeDefinitionInteger {
            identifier,
            common,
            type_ref,
            default_value,
        }),
        Variant::Real => AttributeDefinition::Real(AttributeDefinitionReal {
            identifier,
            common,
            type_ref,
            default_value,
        }),
        Variant::Date => AttributeDefinition::Date(AttributeDefinitionDate {
            identifier,
            common,
            type_ref,
            default_value,
        }),
        Variant::Xhtml => AttributeDefinition::Xhtml(AttributeDefinitionXhtml {
            identifier,
            common,
            type_ref,
            default_value,
        }),
        Variant::Enumeration => AttributeDefinition::Enumeration(AttributeDefinitionEnumeration {
            identifier,
            common,
            type_ref,
            default_value,
            multi_valued,
        }),
    }
}
