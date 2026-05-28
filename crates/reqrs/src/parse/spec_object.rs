//! Parser for `<SPEC-OBJECT>` elements.
//!
//! Mirrors `strict-doc-reqif/reqif/parsers/spec_object_parser.py`. The public
//! [`parse_spec_object`] entry scans for the first start event and defers to
//! the `pub(crate)` inner routine `parse_spec_object_inner`, which is the
//! function the future `<SPEC-OBJECTS>` list driver (Task 14) will call once
//! it has discriminated `Start` vs `Empty` events.
//!
//! The body walk records the order of `<TYPE>` and `<VALUES>` children into
//! [`SpecObject::children_order`] so the unparser can re-emit them in the
//! original source order — this matches the Python `xml_node` round-trip
//! semantics without dragging a node graph through the model.

use crate::error::ReqIfError;
use crate::ids::{SpecObjectId, SpecTypeId};
use crate::model::AttributeValue;
use crate::model::spec_object::{SpecObject, SpecObjectChildTag};
use crate::parse::attribute_value::parse_attribute_values_inner;
use crate::parse::reader::{ReqIfReader, optional_attr, required_attr};
use quick_xml::events::{BytesStart, Event};

/// Standalone entry point — typically used by integration tests and list-driver
/// code. Scans for the first `<SPEC-OBJECT>` start event then defers to
/// `parse_spec_object_inner`.
pub fn parse_spec_object(xml: &str) -> Result<SpecObject, ReqIfError> {
    let mut r = ReqIfReader::new(xml.as_bytes());
    loop {
        match r.read_event()? {
            Event::Start(s) if s.name().as_ref() == b"SPEC-OBJECT" => {
                let owned = s.into_owned();
                return parse_spec_object_inner(&mut r, &owned, false);
            }
            Event::Empty(s) if s.name().as_ref() == b"SPEC-OBJECT" => {
                let owned = s.into_owned();
                return parse_spec_object_inner(&mut r, &owned, true);
            }
            Event::Eof => {
                return Err(ReqIfError::MissingChild {
                    child: "SPEC-OBJECT".into(),
                    parent: "<root>".into(),
                });
            }
            _ => continue,
        }
    }
}

/// Inner parser called once the caller has identified a `<SPEC-OBJECT>` start
/// event. The schema requires both `<TYPE>` and `<VALUES>` children, so a
/// self-closed `<SPEC-OBJECT/>` is rejected.
pub(crate) fn parse_spec_object_inner(
    r: &mut ReqIfReader<'_>,
    start: &BytesStart<'_>,
    was_self_closing: bool,
) -> Result<SpecObject, ReqIfError> {
    if was_self_closing {
        return Err(ReqIfError::MissingChild {
            child: "TYPE".into(),
            parent: "SPEC-OBJECT".into(),
        });
    }

    let identifier = SpecObjectId(required_attr(start, "IDENTIFIER")?);
    let description = optional_attr(start, "DESC");
    let last_change = optional_attr(start, "LAST-CHANGE");
    let long_name = optional_attr(start, "LONG-NAME");

    let mut spec_object_type: Option<SpecTypeId> = None;
    let mut attributes: Vec<AttributeValue> = Vec::new();
    let mut children_order: Vec<SpecObjectChildTag> = Vec::with_capacity(2);

    loop {
        match r.read_event()? {
            Event::Start(s) if s.name().as_ref() == b"TYPE" => {
                children_order.push(SpecObjectChildTag::Type);
                spec_object_type = Some(read_spec_object_type_ref(r)?);
            }
            Event::Start(s) if s.name().as_ref() == b"VALUES" => {
                children_order.push(SpecObjectChildTag::Values);
                attributes = parse_attribute_values_inner(r)?;
            }
            Event::Empty(s) if s.name().as_ref() == b"VALUES" => {
                let _ = s;
                // Self-closed `<VALUES/>` — empty attribute list.
                children_order.push(SpecObjectChildTag::Values);
                attributes = Vec::new();
            }
            Event::End(e) if e.name().as_ref() == b"SPEC-OBJECT" => {
                let spec_object_type = spec_object_type.ok_or(ReqIfError::MissingChild {
                    child: "TYPE".into(),
                    parent: "SPEC-OBJECT".into(),
                })?;
                return Ok(SpecObject {
                    identifier,
                    description,
                    last_change,
                    long_name,
                    spec_object_type,
                    attributes,
                    children_order,
                    comments_before: Vec::new(),
                });
            }
            Event::Eof => {
                return Err(ReqIfError::Xml {
                    pos: r.buffer_position(),
                    msg: "EOF inside <SPEC-OBJECT>".into(),
                });
            }
            _ => continue,
        }
    }
}

/// Read the body of a `<TYPE>` element, returning the text of its inner
/// `<SPEC-OBJECT-TYPE-REF>` child.
fn read_spec_object_type_ref(r: &mut ReqIfReader<'_>) -> Result<SpecTypeId, ReqIfError> {
    let mut value: Option<String> = None;
    loop {
        match r.read_event()? {
            Event::Start(s) if s.name().as_ref() == b"SPEC-OBJECT-TYPE-REF" => {
                let end = s.to_end().into_owned();
                value = Some(r.read_text_to_end(&end)?);
            }
            Event::Empty(s) if s.name().as_ref() == b"SPEC-OBJECT-TYPE-REF" => {
                let _ = s;
                value = Some(String::new());
            }
            Event::End(e) if e.name().as_ref() == b"TYPE" => {
                let v = value.ok_or(ReqIfError::MissingChild {
                    child: "SPEC-OBJECT-TYPE-REF".into(),
                    parent: "TYPE".into(),
                })?;
                return Ok(SpecTypeId(v));
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
