//! Parser for `<SPEC-OBJECT-TYPE>` / `<SPECIFICATION-TYPE>` /
//! `<SPEC-RELATION-TYPE>` / `<RELATION-GROUP-TYPE>`.
//!
//! Mirrors `parse::attribute_def` in shape: the public `parse_spec_type`
//! entry locates the first start/empty event and defers to the `pub(crate)`
//! inner routine `parse_spec_type_inner`, which is the function the future
//! `<SPEC-TYPES>` list driver (Task 14) will call directly once it has
//! discriminated Start vs Empty events.
//!
//! `<SPEC-ATTRIBUTES>` is parsed by walking its children and dispatching every
//! `<ATTRIBUTE-DEFINITION-*>` Start/Empty event to
//! [`crate::parse::attribute_def::parse_attribute_definition_inner`]. The
//! three-state `Option<Vec<AttributeDefinition>>` semantics mirror
//! `DataTypeEnumeration::specified_values`: `None` for "no SPEC-ATTRIBUTES
//! block", `Some(vec![])` for an empty `<SPEC-ATTRIBUTES/>`.

use crate::error::ReqIfError;
use crate::ids::SpecTypeId;
use crate::model::AttributeDefinition;
use crate::model::spec_type::*;
use crate::parse::attribute_def::parse_attribute_definition_inner;
use crate::parse::reader::{ReqIfReader, optional_attr, required_attr};
use quick_xml::events::{BytesStart, Event};

/// Standalone entry point — typically used by integration tests and
/// list-driver code. Scans for the first `<SPEC-*-TYPE>` / `<RELATION-GROUP-TYPE>`
/// start/empty event, then defers to `parse_spec_type_inner`.
pub fn parse_spec_type(xml: &str) -> Result<SpecType, ReqIfError> {
    let mut r = ReqIfReader::new(xml.as_bytes());
    loop {
        match r.read_event()? {
            Event::Start(s) => {
                let tag = s.name().as_ref().to_vec();
                let owned = s.into_owned();
                return parse_spec_type_inner(&mut r, &owned, &tag, false);
            }
            Event::Empty(s) => {
                let tag = s.name().as_ref().to_vec();
                let owned = s.into_owned();
                return parse_spec_type_inner(&mut r, &owned, &tag, true);
            }
            Event::Eof => {
                return Err(ReqIfError::MissingChild {
                    child:
                        "SPEC-OBJECT-TYPE|SPECIFICATION-TYPE|SPEC-RELATION-TYPE|RELATION-GROUP-TYPE"
                            .into(),
                    parent: "<root>".into(),
                });
            }
            _ => continue,
        }
    }
}

/// Inner parser called once the caller has identified the start event of a
/// spec-type element. The `tag` slice is the element name as raw bytes; this
/// routine dispatches off it for the variant + walks the body to capture an
/// optional `<SPEC-ATTRIBUTES>` block.
pub(crate) fn parse_spec_type_inner(
    r: &mut ReqIfReader<'_>,
    start: &BytesStart<'_>,
    tag: &[u8],
    was_self_closing: bool,
) -> Result<SpecType, ReqIfError> {
    let variant = variant_for_tag(tag)?;

    let identifier = SpecTypeId(required_attr(start, "IDENTIFIER")?);
    let description = optional_attr(start, "DESC");
    let last_change = optional_attr(start, "LAST-CHANGE");
    let long_name = optional_attr(start, "LONG-NAME");

    let spec_attributes = if was_self_closing {
        None
    } else {
        parse_spec_attributes(r, tag)?
    };

    let common = SpecTypeCommon {
        identifier,
        description,
        last_change,
        long_name,
        was_self_closing,
        spec_attributes,
    };

    Ok(build_variant(variant, common))
}

/// Walk the body of a spec-type element looking for the optional
/// `<SPEC-ATTRIBUTES>` block. Returns the three-state
/// `Option<Vec<AttributeDefinition>>`.
fn parse_spec_attributes(
    r: &mut ReqIfReader<'_>,
    tag: &[u8],
) -> Result<Option<Vec<AttributeDefinition>>, ReqIfError> {
    let mut attrs: Option<Vec<AttributeDefinition>> = None;
    loop {
        match r.read_event()? {
            Event::Start(s) if s.name().as_ref() == b"SPEC-ATTRIBUTES" => {
                attrs = Some(parse_attribute_definitions_list(r)?);
            }
            Event::Empty(s) if s.name().as_ref() == b"SPEC-ATTRIBUTES" => {
                // <SPEC-ATTRIBUTES/> — empty list, distinct from `None`.
                let _ = s;
                attrs = Some(Vec::new());
            }
            Event::End(e) if e.name().as_ref() == tag => return Ok(attrs),
            Event::Eof => {
                return Err(ReqIfError::Xml {
                    pos: r.buffer_position(),
                    msg: format!("EOF inside <{}>", String::from_utf8_lossy(tag)),
                });
            }
            _ => continue,
        }
    }
}

/// Walk the body of `<SPEC-ATTRIBUTES>` dispatching each child to the
/// `parse_attribute_definition_inner` routine from Task 8.
fn parse_attribute_definitions_list(
    r: &mut ReqIfReader<'_>,
) -> Result<Vec<AttributeDefinition>, ReqIfError> {
    let mut out: Vec<AttributeDefinition> = Vec::new();
    loop {
        match r.read_event()? {
            Event::Start(s) => {
                let child_tag = s.name().as_ref().to_vec();
                let owned = s.into_owned();
                let ad = parse_attribute_definition_inner(r, &owned, &child_tag, false)?;
                out.push(ad);
            }
            Event::Empty(s) => {
                let child_tag = s.name().as_ref().to_vec();
                let owned = s.into_owned();
                // `parse_attribute_definition_inner` will surface a
                // `MissingChild { child: "TYPE", ... }` for self-closed
                // attribute-definition elements — schema violation, but the
                // error is preferable to silently dropping the row.
                let ad = parse_attribute_definition_inner(r, &owned, &child_tag, true)?;
                out.push(ad);
            }
            Event::End(e) if e.name().as_ref() == b"SPEC-ATTRIBUTES" => return Ok(out),
            Event::Eof => {
                return Err(ReqIfError::Xml {
                    pos: r.buffer_position(),
                    msg: "EOF inside <SPEC-ATTRIBUTES>".into(),
                });
            }
            _ => continue,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Variant {
    SpecObject,
    Specification,
    SpecRelation,
    RelationGroup,
}

fn variant_for_tag(tag: &[u8]) -> Result<Variant, ReqIfError> {
    Ok(match tag {
        b"SPEC-OBJECT-TYPE" => Variant::SpecObject,
        b"SPECIFICATION-TYPE" => Variant::Specification,
        b"SPEC-RELATION-TYPE" => Variant::SpecRelation,
        b"RELATION-GROUP-TYPE" => Variant::RelationGroup,
        _ => {
            return Err(ReqIfError::UnexpectedTag {
                tag: String::from_utf8_lossy(tag).into_owned(),
                parent: "SPEC-TYPES".into(),
            });
        }
    })
}

fn build_variant(variant: Variant, common: SpecTypeCommon) -> SpecType {
    match variant {
        Variant::SpecObject => SpecType::SpecObject(SpecObjectType { common }),
        Variant::Specification => SpecType::Specification(SpecificationType { common }),
        Variant::SpecRelation => SpecType::SpecRelation(SpecRelationType { common }),
        Variant::RelationGroup => SpecType::RelationGroup(RelationGroupType { common }),
    }
}
