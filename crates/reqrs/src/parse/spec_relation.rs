//! Parser for `<SPEC-RELATION>` elements.
//!
//! Mirrors `strict-doc-reqif/reqif/parsers/spec_relation_parser.py`. The
//! public [`parse_spec_relation`] entry scans for the first start event and
//! defers to the `pub(crate)` inner routine [`parse_spec_relation_inner`],
//! which is the function the future `<SPEC-RELATIONS>` list driver will call
//! once it has discriminated `Start` vs `Empty` events.
//!
//! Required children: `<TYPE>`, `<SOURCE>`, `<TARGET>`. `<VALUES>` is optional.

use crate::error::ReqIfError;
use crate::ids::{SpecObjectId, SpecRelationId, SpecTypeId};
use crate::model::AttributeValue;
use crate::model::spec_relation::SpecRelation;
use crate::parse::attribute_value::parse_attribute_values_inner;
use crate::parse::reader::{ReqIfReader, optional_attr, required_attr};
use quick_xml::events::{BytesStart, Event};

/// Standalone entry point — typically used by integration tests and list-driver
/// code. Scans for the first `<SPEC-RELATION>` start event then defers to
/// [`parse_spec_relation_inner`].
pub fn parse_spec_relation(xml: &str) -> Result<SpecRelation, ReqIfError> {
    let mut r = ReqIfReader::new(xml.as_bytes());
    loop {
        match r.read_event()? {
            Event::Start(s) if s.name().as_ref() == b"SPEC-RELATION" => {
                let owned = s.into_owned();
                return parse_spec_relation_inner(&mut r, &owned, false);
            }
            Event::Empty(s) if s.name().as_ref() == b"SPEC-RELATION" => {
                let owned = s.into_owned();
                return parse_spec_relation_inner(&mut r, &owned, true);
            }
            Event::Eof => {
                return Err(ReqIfError::MissingChild {
                    child: "SPEC-RELATION".into(),
                    parent: "<root>".into(),
                });
            }
            _ => continue,
        }
    }
}

/// Inner parser called once the caller has identified a `<SPEC-RELATION>`
/// start event. A self-closed `<SPEC-RELATION/>` is rejected because the schema
/// requires `<TYPE>`, `<SOURCE>`, and `<TARGET>` children.
pub(crate) fn parse_spec_relation_inner(
    r: &mut ReqIfReader<'_>,
    start: &BytesStart<'_>,
    was_self_closing: bool,
) -> Result<SpecRelation, ReqIfError> {
    if was_self_closing {
        return Err(ReqIfError::MissingChild {
            child: "TYPE".into(),
            parent: "SPEC-RELATION".into(),
        });
    }

    let identifier = SpecRelationId(required_attr(start, "IDENTIFIER")?);
    let description = optional_attr(start, "DESC");
    let last_change = optional_attr(start, "LAST-CHANGE");
    let long_name = optional_attr(start, "LONG-NAME");

    let mut relation_type: Option<SpecTypeId> = None;
    let mut source: Option<SpecObjectId> = None;
    let mut target: Option<SpecObjectId> = None;
    let mut values: Option<Vec<AttributeValue>> = None;

    loop {
        match r.read_event()? {
            Event::Start(s) if s.name().as_ref() == b"TYPE" => {
                let text = read_inner_ref(r, b"TYPE", b"SPEC-RELATION-TYPE-REF")?;
                relation_type = Some(SpecTypeId(text));
            }
            Event::Start(s) if s.name().as_ref() == b"SOURCE" => {
                let text = read_inner_ref(r, b"SOURCE", b"SPEC-OBJECT-REF")?;
                source = Some(SpecObjectId(text));
            }
            Event::Start(s) if s.name().as_ref() == b"TARGET" => {
                let text = read_inner_ref(r, b"TARGET", b"SPEC-OBJECT-REF")?;
                target = Some(SpecObjectId(text));
            }
            Event::Start(s) if s.name().as_ref() == b"VALUES" => {
                values = Some(parse_attribute_values_inner(r)?);
            }
            Event::Empty(s) if s.name().as_ref() == b"VALUES" => {
                let _ = s;
                values = Some(Vec::new());
            }
            Event::End(e) if e.name().as_ref() == b"SPEC-RELATION" => {
                let relation_type = relation_type.ok_or(ReqIfError::MissingChild {
                    child: "TYPE".into(),
                    parent: "SPEC-RELATION".into(),
                })?;
                let source = source.ok_or(ReqIfError::MissingChild {
                    child: "SOURCE".into(),
                    parent: "SPEC-RELATION".into(),
                })?;
                let target = target.ok_or(ReqIfError::MissingChild {
                    child: "TARGET".into(),
                    parent: "SPEC-RELATION".into(),
                })?;
                return Ok(SpecRelation {
                    identifier,
                    description,
                    last_change,
                    long_name,
                    relation_type,
                    source,
                    target,
                    values,
                });
            }
            Event::Eof => {
                return Err(ReqIfError::Xml {
                    pos: r.buffer_position(),
                    msg: "EOF inside <SPEC-RELATION>".into(),
                });
            }
            _ => continue,
        }
    }
}

/// Read the body of an outer wrapper (`<TYPE>` / `<SOURCE>` / `<TARGET>`),
/// returning the text of the inner ref child (`<SPEC-RELATION-TYPE-REF>` /
/// `<SPEC-OBJECT-REF>`).
fn read_inner_ref(
    r: &mut ReqIfReader<'_>,
    outer: &[u8],
    inner: &[u8],
) -> Result<String, ReqIfError> {
    let mut value: Option<String> = None;
    loop {
        match r.read_event()? {
            Event::Start(s) if s.name().as_ref() == inner => {
                let end = s.to_end().into_owned();
                value = Some(r.read_text_to_end(&end)?);
            }
            Event::Empty(s) if s.name().as_ref() == inner => {
                let _ = s;
                value = Some(String::new());
            }
            Event::End(e) if e.name().as_ref() == outer => {
                return value.ok_or(ReqIfError::MissingChild {
                    child: String::from_utf8_lossy(inner).into_owned(),
                    parent: String::from_utf8_lossy(outer).into_owned(),
                });
            }
            Event::Eof => {
                return Err(ReqIfError::Xml {
                    pos: r.buffer_position(),
                    msg: format!("EOF inside <{}>", String::from_utf8_lossy(outer)),
                });
            }
            _ => continue,
        }
    }
}
