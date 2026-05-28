//! Parser for `<RELATION-GROUP>` elements.
//!
//! Mirrors `strict-doc-reqif/reqif/parsers/relation_group_parser.py`. The
//! singular tag is `<RELATION-GROUP>`; the plural list container in the schema
//! is `<SPEC-RELATION-GROUPS>`. The public [`parse_relation_group`] entry
//! scans for the first start event and defers to the `pub(crate)` inner
//! routine `parse_relation_group_inner`, which is the function the future
//! `<SPEC-RELATION-GROUPS>` list driver will call once it has discriminated
//! `Start` vs `Empty` events.
//!
//! Required children per ReqIF schema: `<TYPE>`, `<SOURCE-SPECIFICATION>`,
//! `<TARGET-SPECIFICATION>`. `<SPEC-RELATIONS>` is optional.

use crate::error::ReqIfError;
use crate::ids::{RelationGroupId, SpecRelationId, SpecTypeId, SpecificationId};
use crate::model::relation_group::RelationGroup;
use crate::parse::reader::{ReqIfReader, optional_attr, required_attr};
use quick_xml::events::{BytesStart, Event};

/// Standalone entry point — typically used by integration tests and list-driver
/// code. Scans for the first `<RELATION-GROUP>` start event then defers to
/// `parse_relation_group_inner`.
pub fn parse_relation_group(xml: &str) -> Result<RelationGroup, ReqIfError> {
    let mut r = ReqIfReader::new(xml.as_bytes());
    loop {
        match r.read_event()? {
            Event::Start(s) if s.name().as_ref() == b"RELATION-GROUP" => {
                let owned = s.into_owned();
                return parse_relation_group_inner(&mut r, &owned, false);
            }
            Event::Empty(s) if s.name().as_ref() == b"RELATION-GROUP" => {
                let owned = s.into_owned();
                return parse_relation_group_inner(&mut r, &owned, true);
            }
            Event::Eof => {
                return Err(ReqIfError::MissingChild {
                    child: "RELATION-GROUP".into(),
                    parent: "<root>".into(),
                });
            }
            _ => continue,
        }
    }
}

/// Inner parser called once the caller has identified a `<RELATION-GROUP>`
/// start event. A self-closed `<RELATION-GROUP/>` is rejected because the
/// schema requires `<TYPE>`, `<SOURCE-SPECIFICATION>`, and
/// `<TARGET-SPECIFICATION>` children.
pub(crate) fn parse_relation_group_inner(
    r: &mut ReqIfReader<'_>,
    start: &BytesStart<'_>,
    was_self_closing: bool,
) -> Result<RelationGroup, ReqIfError> {
    if was_self_closing {
        return Err(ReqIfError::MissingChild {
            child: "TYPE".into(),
            parent: "RELATION-GROUP".into(),
        });
    }

    let identifier = RelationGroupId(required_attr(start, "IDENTIFIER")?);
    let description = optional_attr(start, "DESC");
    let last_change = optional_attr(start, "LAST-CHANGE");
    let long_name = optional_attr(start, "LONG-NAME");

    let mut group_type: Option<SpecTypeId> = None;
    let mut source_specification: Option<SpecificationId> = None;
    let mut target_specification: Option<SpecificationId> = None;
    let mut spec_relations: Option<Vec<SpecRelationId>> = None;

    loop {
        match r.read_event()? {
            Event::Start(s) if s.name().as_ref() == b"TYPE" => {
                let text = read_inner_ref(r, b"TYPE", b"RELATION-GROUP-TYPE-REF")?;
                group_type = Some(SpecTypeId(text));
            }
            Event::Start(s) if s.name().as_ref() == b"SOURCE-SPECIFICATION" => {
                let text = read_inner_ref(r, b"SOURCE-SPECIFICATION", b"SPECIFICATION-REF")?;
                source_specification = Some(SpecificationId(text));
            }
            Event::Start(s) if s.name().as_ref() == b"TARGET-SPECIFICATION" => {
                let text = read_inner_ref(r, b"TARGET-SPECIFICATION", b"SPECIFICATION-REF")?;
                target_specification = Some(SpecificationId(text));
            }
            Event::Start(s) if s.name().as_ref() == b"SPEC-RELATIONS" => {
                spec_relations = Some(read_spec_relation_refs(r)?);
            }
            Event::Empty(s) if s.name().as_ref() == b"SPEC-RELATIONS" => {
                let _ = s;
                spec_relations = Some(Vec::new());
            }
            Event::End(e) if e.name().as_ref() == b"RELATION-GROUP" => {
                let group_type = group_type.ok_or(ReqIfError::MissingChild {
                    child: "TYPE".into(),
                    parent: "RELATION-GROUP".into(),
                })?;
                let source_specification =
                    source_specification.ok_or(ReqIfError::MissingChild {
                        child: "SOURCE-SPECIFICATION".into(),
                        parent: "RELATION-GROUP".into(),
                    })?;
                let target_specification =
                    target_specification.ok_or(ReqIfError::MissingChild {
                        child: "TARGET-SPECIFICATION".into(),
                        parent: "RELATION-GROUP".into(),
                    })?;
                return Ok(RelationGroup {
                    identifier,
                    description,
                    last_change,
                    long_name,
                    group_type,
                    source_specification,
                    target_specification,
                    spec_relations,
                });
            }
            Event::Eof => {
                return Err(ReqIfError::Xml {
                    pos: r.buffer_position(),
                    msg: "EOF inside <RELATION-GROUP>".into(),
                });
            }
            _ => continue,
        }
    }
}

/// Read the body of an outer wrapper, returning the text of the inner ref
/// child.
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

/// Drive a `<SPEC-RELATIONS>` block: collect every `<SPEC-RELATION-REF>` text
/// child as a [`SpecRelationId`].
fn read_spec_relation_refs(r: &mut ReqIfReader<'_>) -> Result<Vec<SpecRelationId>, ReqIfError> {
    let mut out: Vec<SpecRelationId> = Vec::new();
    loop {
        match r.read_event()? {
            Event::Start(s) if s.name().as_ref() == b"SPEC-RELATION-REF" => {
                let end = s.to_end().into_owned();
                let text = r.read_text_to_end(&end)?;
                out.push(SpecRelationId(text));
            }
            Event::Empty(s) if s.name().as_ref() == b"SPEC-RELATION-REF" => {
                let _ = s;
                out.push(SpecRelationId(String::new()));
            }
            Event::End(e) if e.name().as_ref() == b"SPEC-RELATIONS" => return Ok(out),
            Event::Eof => {
                return Err(ReqIfError::Xml {
                    pos: r.buffer_position(),
                    msg: "EOF inside <SPEC-RELATIONS>".into(),
                });
            }
            _ => continue,
        }
    }
}
