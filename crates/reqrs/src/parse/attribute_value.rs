//! Parser for `<ATTRIBUTE-VALUE-*>` elements.
//!
//! Mirrors `parse::attribute_def` in shape: the public
//! `parse_attribute_value` entry locates the first start event then defers to
//! the `pub(crate)` inner routine. A second `pub(crate)` helper —
//! `parse_attribute_values_inner` — is the function the future `<VALUES>` list
//! driver on `<SPEC-OBJECT>` (Task 11) will call directly.

use crate::error::ReqIfError;
use crate::ids::{AttributeDefId, EnumValueId};
use crate::model::attribute_value::*;
use crate::parse::reader::{ReqIfReader, required_attr};
use quick_xml::events::{BytesStart, Event};

/// Standalone entry point — typically used by integration tests and
/// list-driver code. Scans for the first `<ATTRIBUTE-VALUE-*>` start event,
/// then defers to `parse_attribute_value_inner`.
pub fn parse_attribute_value(xml: &str) -> Result<AttributeValue, ReqIfError> {
    let mut r = ReqIfReader::new(xml.as_bytes());
    loop {
        match r.read_event()? {
            Event::Start(s) => {
                let tag = s.name().as_ref().to_vec();
                let owned = s.into_owned();
                return parse_attribute_value_inner(&mut r, &owned, &tag, false);
            }
            Event::Empty(s) => {
                let tag = s.name().as_ref().to_vec();
                let owned = s.into_owned();
                return parse_attribute_value_inner(&mut r, &owned, &tag, true);
            }
            Event::Eof => {
                return Err(ReqIfError::MissingChild {
                    child: "ATTRIBUTE-VALUE-*".into(),
                    parent: "<root>".into(),
                });
            }
            _ => continue,
        }
    }
}

/// Drive a `<VALUES>` block: read every `<ATTRIBUTE-VALUE-*>` child until the
/// closing `</VALUES>` is reached, returning the collected typed values plus
/// any trailing inline `<!-- ... -->` comments after the last sibling.
///
/// Inter-sibling `Event::Comment` events are accumulated into
/// `pending_comments` and attached to the next element's `comments_before`
/// field. Comments that appear AFTER the last `<ATTRIBUTE-VALUE-*>` and before
/// the closing `</VALUES>` end up in the returned `trailing_comments` vec —
/// the caller (a `SpecObject` / `Specification` / `SpecRelation` parser)
/// stashes them in the matching `values_trailing_comments` slot on the
/// enclosing model type so they survive round-trip.
///
/// Precondition: the caller has just consumed the `<VALUES>` start event.
pub(crate) fn parse_attribute_values_inner(
    r: &mut ReqIfReader<'_>,
) -> Result<(Vec<AttributeValue>, Vec<String>), ReqIfError> {
    let mut out: Vec<AttributeValue> = Vec::new();
    let mut pending_comments: Vec<String> = Vec::new();
    loop {
        match r.read_event()? {
            Event::Comment(c) => {
                pending_comments.push(String::from_utf8_lossy(c.as_ref()).into_owned());
            }
            Event::Start(s) => {
                let tag = s.name().as_ref().to_vec();
                let owned = s.into_owned();
                let mut av = parse_attribute_value_inner(r, &owned, &tag, false)?;
                attach_comments(&mut av, std::mem::take(&mut pending_comments));
                out.push(av);
            }
            Event::Empty(s) => {
                let tag = s.name().as_ref().to_vec();
                let owned = s.into_owned();
                let mut av = parse_attribute_value_inner(r, &owned, &tag, true)?;
                attach_comments(&mut av, std::mem::take(&mut pending_comments));
                out.push(av);
            }
            Event::End(e) if e.name().as_ref() == b"VALUES" => return Ok((out, pending_comments)),
            Event::Eof => {
                return Err(ReqIfError::Xml {
                    pos: r.buffer_position(),
                    msg: "EOF inside <VALUES>".into(),
                });
            }
            _ => continue,
        }
    }
}

/// Attach the accumulated inter-sibling comments to the appropriate variant
/// field. Each variant carries its own `comments_before` slot; the per-variant
/// match keeps the field handling local and obvious.
fn attach_comments(av: &mut AttributeValue, comments: Vec<String>) {
    match av {
        AttributeValue::String(a) => a.comments_before = comments,
        AttributeValue::Boolean(a) => a.comments_before = comments,
        AttributeValue::Integer(a) => a.comments_before = comments,
        AttributeValue::Real(a) => a.comments_before = comments,
        AttributeValue::Date(a) => a.comments_before = comments,
        AttributeValue::Xhtml(a) => a.comments_before = comments,
        AttributeValue::Enumeration(a) => a.comments_before = comments,
    }
}

/// Inner parser called once the caller has identified the start event of an
/// `<ATTRIBUTE-VALUE-*>` element. Dispatches off `tag` for the variant and
/// reads the required children.
pub(crate) fn parse_attribute_value_inner(
    r: &mut ReqIfReader<'_>,
    start: &BytesStart<'_>,
    tag: &[u8],
    was_self_closing: bool,
) -> Result<AttributeValue, ReqIfError> {
    // Self-closed `<ATTRIBUTE-VALUE-*/>` violates the schema — every variant
    // requires at least the `<DEFINITION>` child carrying the type reference.
    if was_self_closing {
        return Err(ReqIfError::MissingChild {
            child: "DEFINITION".into(),
            parent: String::from_utf8_lossy(tag).into_owned(),
        });
    }

    match tag {
        b"ATTRIBUTE-VALUE-STRING" => {
            let value = required_attr(start, "THE-VALUE")?;
            let definition_ref =
                parse_definition_child(r, tag, b"ATTRIBUTE-DEFINITION-STRING-REF")?;
            Ok(AttributeValue::String(AttributeValueString {
                definition_ref,
                value,
                comments_before: Vec::new(),
            }))
        }
        b"ATTRIBUTE-VALUE-BOOLEAN" => {
            let raw = required_attr(start, "THE-VALUE")?;
            let value = match raw.as_str() {
                "true" => true,
                "false" => false,
                _ => {
                    return Err(ReqIfError::Schema(format!(
                        "ATTRIBUTE-VALUE-BOOLEAN THE-VALUE must be true|false, got {raw:?}"
                    )));
                }
            };
            let definition_ref =
                parse_definition_child(r, tag, b"ATTRIBUTE-DEFINITION-BOOLEAN-REF")?;
            Ok(AttributeValue::Boolean(AttributeValueBoolean {
                definition_ref,
                value,
                comments_before: Vec::new(),
            }))
        }
        b"ATTRIBUTE-VALUE-INTEGER" => {
            let value = required_attr(start, "THE-VALUE")?;
            let definition_ref =
                parse_definition_child(r, tag, b"ATTRIBUTE-DEFINITION-INTEGER-REF")?;
            Ok(AttributeValue::Integer(AttributeValueInteger {
                definition_ref,
                value,
                comments_before: Vec::new(),
            }))
        }
        b"ATTRIBUTE-VALUE-REAL" => {
            let value = required_attr(start, "THE-VALUE")?;
            let definition_ref = parse_definition_child(r, tag, b"ATTRIBUTE-DEFINITION-REAL-REF")?;
            Ok(AttributeValue::Real(AttributeValueReal {
                definition_ref,
                value,
                comments_before: Vec::new(),
            }))
        }
        b"ATTRIBUTE-VALUE-DATE" => {
            let value = required_attr(start, "THE-VALUE")?;
            let definition_ref = parse_definition_child(r, tag, b"ATTRIBUTE-DEFINITION-DATE-REF")?;
            Ok(AttributeValue::Date(AttributeValueDate {
                definition_ref,
                value,
                comments_before: Vec::new(),
            }))
        }
        b"ATTRIBUTE-VALUE-ENUMERATION" => parse_enumeration_body(r, tag),
        b"ATTRIBUTE-VALUE-XHTML" => parse_xhtml_body(r, tag),
        _ => Err(ReqIfError::UnexpectedTag {
            tag: String::from_utf8_lossy(tag).into_owned(),
            parent: "VALUES".into(),
        }),
    }
}

/// Walk a scalar variant's body looking only for the single `<DEFINITION>`
/// child. Returns the inner `*-REF` text.
fn parse_definition_child(
    r: &mut ReqIfReader<'_>,
    outer_tag: &[u8],
    ref_child: &[u8],
) -> Result<AttributeDefId, ReqIfError> {
    let mut def_ref: Option<String> = None;
    loop {
        match r.read_event()? {
            Event::Start(s) if s.name().as_ref() == b"DEFINITION" => {
                def_ref = Some(read_definition_ref(r, ref_child)?);
            }
            Event::End(e) if e.name().as_ref() == outer_tag => {
                let id = def_ref.ok_or(ReqIfError::MissingChild {
                    child: "DEFINITION".into(),
                    parent: String::from_utf8_lossy(outer_tag).into_owned(),
                })?;
                return Ok(AttributeDefId(id));
            }
            Event::Eof => {
                return Err(ReqIfError::Xml {
                    pos: r.buffer_position(),
                    msg: format!("EOF inside <{}>", String::from_utf8_lossy(outer_tag)),
                });
            }
            _ => continue,
        }
    }
}

/// Read the body of a `<DEFINITION>` element, returning the text of its
/// `<ATTRIBUTE-DEFINITION-*-REF>` child.
fn read_definition_ref(r: &mut ReqIfReader<'_>, ref_child: &[u8]) -> Result<String, ReqIfError> {
    let mut value: Option<String> = None;
    loop {
        match r.read_event()? {
            Event::Start(s) if s.name().as_ref() == ref_child => {
                let end = s.to_end().into_owned();
                value = Some(r.read_text_to_end(&end)?);
            }
            Event::Empty(s) if s.name().as_ref() == ref_child => {
                let _ = s;
                value = Some(String::new());
            }
            Event::End(e) if e.name().as_ref() == b"DEFINITION" => {
                return value.ok_or(ReqIfError::MissingChild {
                    child: String::from_utf8_lossy(ref_child).into_owned(),
                    parent: "DEFINITION".into(),
                });
            }
            Event::Eof => {
                return Err(ReqIfError::Xml {
                    pos: r.buffer_position(),
                    msg: "EOF inside <DEFINITION>".into(),
                });
            }
            _ => continue,
        }
    }
}

/// Walk the body of an `<ATTRIBUTE-VALUE-ENUMERATION>`, capturing the
/// `<DEFINITION>` ref and the list of `<ENUM-VALUE-REF>`s inside `<VALUES>`
/// plus the order of the two children.
fn parse_enumeration_body(
    r: &mut ReqIfReader<'_>,
    outer_tag: &[u8],
) -> Result<AttributeValue, ReqIfError> {
    let mut def_ref: Option<String> = None;
    let mut values: Option<Vec<EnumValueId>> = None;
    let mut seen_def_first: Option<bool> = None;

    loop {
        match r.read_event()? {
            Event::Start(s) if s.name().as_ref() == b"DEFINITION" => {
                if seen_def_first.is_none() {
                    seen_def_first = Some(true);
                }
                def_ref = Some(read_definition_ref(
                    r,
                    b"ATTRIBUTE-DEFINITION-ENUMERATION-REF",
                )?);
            }
            Event::Start(s) if s.name().as_ref() == b"VALUES" => {
                if seen_def_first.is_none() {
                    seen_def_first = Some(false);
                }
                values = Some(read_enum_value_refs(r)?);
            }
            Event::Empty(s) if s.name().as_ref() == b"VALUES" => {
                let _ = s;
                if seen_def_first.is_none() {
                    seen_def_first = Some(false);
                }
                values = Some(Vec::new());
            }
            Event::End(e) if e.name().as_ref() == outer_tag => {
                let definition_ref = AttributeDefId(def_ref.ok_or(ReqIfError::MissingChild {
                    child: "DEFINITION".into(),
                    parent: String::from_utf8_lossy(outer_tag).into_owned(),
                })?);
                let values = values.ok_or(ReqIfError::MissingChild {
                    child: "VALUES".into(),
                    parent: String::from_utf8_lossy(outer_tag).into_owned(),
                })?;
                let was_definition_first = seen_def_first.unwrap_or(true);
                return Ok(AttributeValue::Enumeration(AttributeValueEnumeration {
                    definition_ref,
                    values,
                    was_definition_first,
                    comments_before: Vec::new(),
                }));
            }
            Event::Eof => {
                return Err(ReqIfError::Xml {
                    pos: r.buffer_position(),
                    msg: format!("EOF inside <{}>", String::from_utf8_lossy(outer_tag)),
                });
            }
            _ => continue,
        }
    }
}

/// Read the body of a `<VALUES>` block under an enumeration value, collecting
/// `<ENUM-VALUE-REF>` text children.
fn read_enum_value_refs(r: &mut ReqIfReader<'_>) -> Result<Vec<EnumValueId>, ReqIfError> {
    let mut out: Vec<EnumValueId> = Vec::new();
    loop {
        match r.read_event()? {
            Event::Start(s) if s.name().as_ref() == b"ENUM-VALUE-REF" => {
                let end = s.to_end().into_owned();
                let text = r.read_text_to_end(&end)?;
                out.push(EnumValueId(text));
            }
            Event::Empty(s) if s.name().as_ref() == b"ENUM-VALUE-REF" => {
                let _ = s;
                out.push(EnumValueId(String::new()));
            }
            Event::End(e) if e.name().as_ref() == b"VALUES" => return Ok(out),
            Event::Eof => {
                return Err(ReqIfError::Xml {
                    pos: r.buffer_position(),
                    msg: "EOF inside <VALUES>".into(),
                });
            }
            _ => continue,
        }
    }
}

/// Walk the body of an `<ATTRIBUTE-VALUE-XHTML>`, capturing the
/// `<DEFINITION>` ref and the verbatim inner bytes of `<THE-VALUE>` plus the
/// order of the two children.
fn parse_xhtml_body(
    r: &mut ReqIfReader<'_>,
    outer_tag: &[u8],
) -> Result<AttributeValue, ReqIfError> {
    let mut def_ref: Option<String> = None;
    let mut the_value_raw: Option<String> = None;
    let mut seen_def_first: Option<bool> = None;

    loop {
        match r.read_event()? {
            Event::Start(s) if s.name().as_ref() == b"DEFINITION" => {
                if seen_def_first.is_none() {
                    seen_def_first = Some(true);
                }
                def_ref = Some(read_definition_ref(r, b"ATTRIBUTE-DEFINITION-XHTML-REF")?);
            }
            Event::Start(s) if s.name().as_ref() == b"THE-VALUE" => {
                let owned = s.into_owned();
                if seen_def_first.is_none() {
                    seen_def_first = Some(false);
                }
                the_value_raw = Some(r.capture_inner_raw(&owned)?);
            }
            Event::Empty(s) if s.name().as_ref() == b"THE-VALUE" => {
                // Self-closed `<THE-VALUE/>` — empty inner content. We accept
                // this and round-trip as empty inner bytes, but the unparser
                // emits the open/close form (matching the Python template).
                let _ = s;
                if seen_def_first.is_none() {
                    seen_def_first = Some(false);
                }
                the_value_raw = Some(String::new());
            }
            Event::End(e) if e.name().as_ref() == outer_tag => {
                let definition_ref = AttributeDefId(def_ref.ok_or(ReqIfError::MissingChild {
                    child: "DEFINITION".into(),
                    parent: String::from_utf8_lossy(outer_tag).into_owned(),
                })?);
                let the_value_raw = the_value_raw.ok_or(ReqIfError::MissingChild {
                    child: "THE-VALUE".into(),
                    parent: String::from_utf8_lossy(outer_tag).into_owned(),
                })?;
                let was_definition_first = seen_def_first.unwrap_or(true);
                return Ok(AttributeValue::Xhtml(AttributeValueXhtml {
                    definition_ref,
                    the_value_raw,
                    was_definition_first,
                    comments_before: Vec::new(),
                }));
            }
            Event::Eof => {
                return Err(ReqIfError::Xml {
                    pos: r.buffer_position(),
                    msg: format!("EOF inside <{}>", String::from_utf8_lossy(outer_tag)),
                });
            }
            _ => continue,
        }
    }
}
