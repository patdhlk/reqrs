//! Parser for `<SPEC-HIERARCHY>` elements.
//!
//! Mirrors `strict-doc-reqif/reqif/parsers/spec_hierarchy_parser.py`. The
//! public [`parse_spec_hierarchy`] entry seeds `level = 1` and the inner
//! routine increments it on every recursion into a `<CHILDREN>` block.
//!
//! Two source-order traits are captured during the body walk so the unparser
//! can re-emit byte-exact:
//! - `ref_then_children_order` — `true` when `<OBJECT>` came before
//!   `<CHILDREN>` (Polarion convention), `false` when the order was reversed.
//! - `was_self_closing_children` — `true` when the source had `<CHILDREN/>`
//!   self-closed; only meaningful when `children == Some(vec![])`.

use crate::error::ReqIfError;
use crate::ids::SpecObjectId;
use crate::model::spec_hierarchy::SpecHierarchy;
use crate::parse::reader::{ReqIfReader, optional_attr, required_attr};
use quick_xml::events::{BytesStart, Event};

/// Standalone entry point — typically used by integration tests and list-driver
/// code. Scans for the first `<SPEC-HIERARCHY>` start event then defers to
/// [`parse_spec_hierarchy_inner`] with `level = 1`.
pub fn parse_spec_hierarchy(xml: &str) -> Result<SpecHierarchy, ReqIfError> {
    let mut r = ReqIfReader::new(xml.as_bytes());
    loop {
        match r.read_event()? {
            Event::Start(s) if s.name().as_ref() == b"SPEC-HIERARCHY" => {
                let owned = s.into_owned();
                return parse_spec_hierarchy_inner(&mut r, &owned, false, 1);
            }
            Event::Empty(s) if s.name().as_ref() == b"SPEC-HIERARCHY" => {
                let owned = s.into_owned();
                return parse_spec_hierarchy_inner(&mut r, &owned, true, 1);
            }
            Event::Eof => {
                return Err(ReqIfError::MissingChild {
                    child: "SPEC-HIERARCHY".into(),
                    parent: "<root>".into(),
                });
            }
            _ => continue,
        }
    }
}

/// Inner parser called once the caller has identified a `<SPEC-HIERARCHY>`
/// start event. The schema requires the `<OBJECT>` child, so a self-closed
/// `<SPEC-HIERARCHY/>` is rejected.
pub(crate) fn parse_spec_hierarchy_inner(
    r: &mut ReqIfReader<'_>,
    start: &BytesStart<'_>,
    was_self_closing: bool,
    level: usize,
) -> Result<SpecHierarchy, ReqIfError> {
    if was_self_closing {
        return Err(ReqIfError::MissingChild {
            child: "OBJECT".into(),
            parent: "SPEC-HIERARCHY".into(),
        });
    }

    let identifier = required_attr(start, "IDENTIFIER")?;
    let last_change = optional_attr(start, "LAST-CHANGE");
    let long_name = optional_attr(start, "LONG-NAME");
    let editable = optional_attr(start, "IS-EDITABLE").map(|v| v == "true");
    let is_table_internal = optional_attr(start, "IS-TABLE-INTERNAL").map(|v| v == "true");

    let mut spec_object_ref: Option<SpecObjectId> = None;
    let mut children: Option<Vec<SpecHierarchy>> = None;
    let mut was_self_closing_children = false;
    // Tracks order of the two children. `None` => not yet seen; `Some(true)` =>
    // OBJECT first; `Some(false)` => CHILDREN first.
    let mut object_first: Option<bool> = None;

    loop {
        match r.read_event()? {
            Event::Start(s) if s.name().as_ref() == b"OBJECT" => {
                if object_first.is_none() {
                    object_first = Some(true);
                }
                spec_object_ref = Some(read_spec_object_ref(r)?);
            }
            Event::Start(s) if s.name().as_ref() == b"CHILDREN" => {
                if object_first.is_none() {
                    object_first = Some(false);
                }
                children = Some(read_children(r, level + 1)?);
            }
            Event::Empty(s) if s.name().as_ref() == b"CHILDREN" => {
                let _ = s;
                if object_first.is_none() {
                    object_first = Some(false);
                }
                children = Some(Vec::new());
                was_self_closing_children = true;
            }
            Event::End(e) if e.name().as_ref() == b"SPEC-HIERARCHY" => {
                let spec_object_ref = spec_object_ref.ok_or(ReqIfError::MissingChild {
                    child: "OBJECT".into(),
                    parent: "SPEC-HIERARCHY".into(),
                })?;
                // Default `ref_then_children_order = true` mirrors the Python
                // model's default — most tools emit OBJECT first, and a
                // hierarchy with no `<CHILDREN>` still has a well-defined order.
                let ref_then_children_order = object_first.unwrap_or(true);
                return Ok(SpecHierarchy {
                    identifier,
                    last_change,
                    long_name,
                    editable,
                    is_table_internal,
                    spec_object_ref,
                    children,
                    ref_then_children_order,
                    level,
                    was_self_closing_children,
                });
            }
            Event::Eof => {
                return Err(ReqIfError::Xml {
                    pos: r.buffer_position(),
                    msg: "EOF inside <SPEC-HIERARCHY>".into(),
                });
            }
            _ => continue,
        }
    }
}

/// Read the body of an `<OBJECT>` element, returning the text of its inner
/// `<SPEC-OBJECT-REF>` child.
fn read_spec_object_ref(r: &mut ReqIfReader<'_>) -> Result<SpecObjectId, ReqIfError> {
    let mut value: Option<String> = None;
    loop {
        match r.read_event()? {
            Event::Start(s) if s.name().as_ref() == b"SPEC-OBJECT-REF" => {
                let end = s.to_end().into_owned();
                value = Some(r.read_text_to_end(&end)?);
            }
            Event::Empty(s) if s.name().as_ref() == b"SPEC-OBJECT-REF" => {
                let _ = s;
                value = Some(String::new());
            }
            Event::End(e) if e.name().as_ref() == b"OBJECT" => {
                let v = value.ok_or(ReqIfError::MissingChild {
                    child: "SPEC-OBJECT-REF".into(),
                    parent: "OBJECT".into(),
                })?;
                return Ok(SpecObjectId(v));
            }
            Event::Eof => {
                return Err(ReqIfError::Xml {
                    pos: r.buffer_position(),
                    msg: "EOF inside <OBJECT>".into(),
                });
            }
            _ => continue,
        }
    }
}

/// Drive a `<CHILDREN>` block: read every nested `<SPEC-HIERARCHY>` until the
/// closing `</CHILDREN>` is reached. Precondition: the caller has just
/// consumed the `<CHILDREN>` Start event.
fn read_children(
    r: &mut ReqIfReader<'_>,
    child_level: usize,
) -> Result<Vec<SpecHierarchy>, ReqIfError> {
    let mut out: Vec<SpecHierarchy> = Vec::new();
    loop {
        match r.read_event()? {
            Event::Start(s) if s.name().as_ref() == b"SPEC-HIERARCHY" => {
                let owned = s.into_owned();
                out.push(parse_spec_hierarchy_inner(r, &owned, false, child_level)?);
            }
            Event::Empty(s) if s.name().as_ref() == b"SPEC-HIERARCHY" => {
                let owned = s.into_owned();
                // Self-closed `<SPEC-HIERARCHY/>` is a schema violation;
                // `parse_spec_hierarchy_inner` will surface the appropriate
                // `MissingChild` error rather than silently dropping the row.
                out.push(parse_spec_hierarchy_inner(r, &owned, true, child_level)?);
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
