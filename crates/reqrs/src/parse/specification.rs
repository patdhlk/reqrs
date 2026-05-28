//! Parser for `<SPECIFICATION>` elements.
//!
//! Mirrors `strict-doc-reqif/reqif/parsers/specification_parser.py`. The
//! public [`parse_specification`] entry scans for the first start event and
//! defers to the `pub(crate)` inner routine [`parse_specification_inner`],
//! which is the function the future `<SPECIFICATIONS>` list driver will call
//! once it has discriminated `Start` vs `Empty` events.
//!
//! The body walk records the order of `<TYPE>`, `<CHILDREN>`, and `<VALUES>`
//! children into [`Specification::children_order`] so the unparser can re-emit
//! them in the original source order — this matches the Python `xml_node`
//! round-trip semantics without dragging a node graph through the model.

use crate::error::ReqIfError;
use crate::ids::{SpecTypeId, SpecificationId};
use crate::model::AttributeValue;
use crate::model::spec_hierarchy::SpecHierarchy;
use crate::model::specification::{Specification, SpecificationChildTag};
use crate::parse::attribute_value::parse_attribute_values_inner;
use crate::parse::reader::{ReqIfReader, optional_attr, required_attr};
use crate::parse::spec_hierarchy::parse_spec_hierarchy_inner;
use quick_xml::events::{BytesStart, Event};

/// Standalone entry point — typically used by integration tests and list-driver
/// code. Scans for the first `<SPECIFICATION>` start event then defers to
/// [`parse_specification_inner`].
pub fn parse_specification(xml: &str) -> Result<Specification, ReqIfError> {
    let mut r = ReqIfReader::new(xml.as_bytes());
    loop {
        match r.read_event()? {
            Event::Start(s) if s.name().as_ref() == b"SPECIFICATION" => {
                let owned = s.into_owned();
                return parse_specification_inner(&mut r, &owned, false);
            }
            Event::Empty(s) if s.name().as_ref() == b"SPECIFICATION" => {
                let owned = s.into_owned();
                return parse_specification_inner(&mut r, &owned, true);
            }
            Event::Eof => {
                return Err(ReqIfError::MissingChild {
                    child: "SPECIFICATION".into(),
                    parent: "<root>".into(),
                });
            }
            _ => continue,
        }
    }
}

/// Inner parser called once the caller has identified a `<SPECIFICATION>`
/// start event. A self-closed `<SPECIFICATION/>` is legal but unusual — the
/// Python reference treats every child as optional, so the only required
/// attribute is `IDENTIFIER`.
pub(crate) fn parse_specification_inner(
    r: &mut ReqIfReader<'_>,
    start: &BytesStart<'_>,
    was_self_closing: bool,
) -> Result<Specification, ReqIfError> {
    let identifier = SpecificationId(required_attr(start, "IDENTIFIER")?);
    let description = optional_attr(start, "DESC");
    let last_change = optional_attr(start, "LAST-CHANGE");
    let long_name = optional_attr(start, "LONG-NAME");

    if was_self_closing {
        return Ok(Specification {
            identifier,
            description,
            last_change,
            long_name,
            specification_type: None,
            values: None,
            children: None,
            children_order: Vec::new(),
        });
    }

    let mut specification_type: Option<SpecTypeId> = None;
    let mut values: Option<Vec<AttributeValue>> = None;
    let mut children: Option<Vec<SpecHierarchy>> = None;
    let mut children_order: Vec<SpecificationChildTag> = Vec::with_capacity(3);

    loop {
        match r.read_event()? {
            Event::Start(s) if s.name().as_ref() == b"TYPE" => {
                children_order.push(SpecificationChildTag::Type);
                specification_type = Some(read_specification_type_ref(r)?);
            }
            Event::Start(s) if s.name().as_ref() == b"CHILDREN" => {
                children_order.push(SpecificationChildTag::Children);
                children = Some(read_children(r)?);
            }
            Event::Empty(s) if s.name().as_ref() == b"CHILDREN" => {
                let _ = s;
                children_order.push(SpecificationChildTag::Children);
                children = Some(Vec::new());
            }
            Event::Start(s) if s.name().as_ref() == b"VALUES" => {
                children_order.push(SpecificationChildTag::Values);
                values = Some(parse_attribute_values_inner(r)?);
            }
            Event::Empty(s) if s.name().as_ref() == b"VALUES" => {
                let _ = s;
                children_order.push(SpecificationChildTag::Values);
                values = Some(Vec::new());
            }
            Event::End(e) if e.name().as_ref() == b"SPECIFICATION" => {
                return Ok(Specification {
                    identifier,
                    description,
                    last_change,
                    long_name,
                    specification_type,
                    values,
                    children,
                    children_order,
                });
            }
            Event::Eof => {
                return Err(ReqIfError::Xml {
                    pos: r.buffer_position(),
                    msg: "EOF inside <SPECIFICATION>".into(),
                });
            }
            _ => continue,
        }
    }
}

/// Read the body of a `<TYPE>` element, returning the text of its inner
/// `<SPECIFICATION-TYPE-REF>` child.
fn read_specification_type_ref(r: &mut ReqIfReader<'_>) -> Result<SpecTypeId, ReqIfError> {
    let mut value: Option<String> = None;
    loop {
        match r.read_event()? {
            Event::Start(s) if s.name().as_ref() == b"SPECIFICATION-TYPE-REF" => {
                let end = s.to_end().into_owned();
                value = Some(r.read_text_to_end(&end)?);
            }
            Event::Empty(s) if s.name().as_ref() == b"SPECIFICATION-TYPE-REF" => {
                let _ = s;
                value = Some(String::new());
            }
            Event::End(e) if e.name().as_ref() == b"TYPE" => {
                let v = value.ok_or(ReqIfError::MissingChild {
                    child: "SPECIFICATION-TYPE-REF".into(),
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

/// Drive a `<CHILDREN>` block under a `<SPECIFICATION>`: read every
/// `<SPEC-HIERARCHY>` (always at level=1 directly under SPECIFICATION) until
/// the closing `</CHILDREN>` is reached. Precondition: caller has just
/// consumed the `<CHILDREN>` Start event.
fn read_children(r: &mut ReqIfReader<'_>) -> Result<Vec<SpecHierarchy>, ReqIfError> {
    let mut out: Vec<SpecHierarchy> = Vec::new();
    loop {
        match r.read_event()? {
            Event::Start(s) if s.name().as_ref() == b"SPEC-HIERARCHY" => {
                let owned = s.into_owned();
                out.push(parse_spec_hierarchy_inner(r, &owned, false, 1)?);
            }
            Event::Empty(s) if s.name().as_ref() == b"SPEC-HIERARCHY" => {
                let owned = s.into_owned();
                out.push(parse_spec_hierarchy_inner(r, &owned, true, 1)?);
            }
            Event::End(e) if e.name().as_ref() == b"CHILDREN" => return Ok(out),
            Event::Eof => {
                return Err(ReqIfError::Xml {
                    pos: r.buffer_position(),
                    msg: "EOF inside <CHILDREN>".into(),
                });
            }
            _ => continue,
        }
    }
}
