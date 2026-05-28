//! Top-level orchestrator: walk a full `<REQ-IF>` document, dispatching each
//! direct child to the appropriate per-element parser, and assemble the
//! resulting [`ReqIfBundle`].
//!
//! Mirrors `strict-doc-reqif/reqif/parser.py::ReqIFParser._parse_reqif` and
//! `_parse_reqif_content`. The driver is intentionally thin — it only:
//!
//! - detects the XML declaration (best-effort) to capture
//!   [`NamespaceInfo::doctype_is_present`] + `encoding`,
//! - harvests namespace / schema-location / xml:lang attributes off the root,
//! - dispatches each direct child of `<REQ-IF>` / `<CORE-CONTENT>` /
//!   `<REQ-IF-CONTENT>` to the matching `pub(crate)` per-element inner parser,
//! - builds an [`ObjectLookup`] from the assembled content,
//! - records `TOOL-EXTENSIONS` presence as a boolean.
//!
//! Unknown elements are skipped with a [`SchemaWarning`] (Python behaviour).

use crate::error::{ReqIfError, SchemaWarning};
use crate::model::{
    CoreContent, NamespaceInfo, ObjectLookup, ReqIfBundle, ReqIfContent, ReqIfHeader,
};
use crate::parse::data_type::parse_data_type_inner;
use crate::parse::header::parse_header_from_reader;
use crate::parse::reader::ReqIfReader;
use crate::parse::relation_group::parse_relation_group_inner;
use crate::parse::spec_object::parse_spec_object_inner;
use crate::parse::spec_relation::parse_spec_relation_inner;
use crate::parse::spec_type::parse_spec_type_inner;
use crate::parse::specification::parse_specification_inner;
use quick_xml::events::{BytesStart, Event};

/// Parse a full `<REQ-IF>` document from raw bytes.
pub(crate) fn parse_bundle(bytes: &[u8]) -> Result<ReqIfBundle, ReqIfError> {
    let (doctype_is_present, encoding) = sniff_xml_declaration(bytes);

    let mut r = ReqIfReader::new(bytes);
    let (root_start, was_self_closing) = locate_root(&mut r)?;

    let mut namespace_info = harvest_namespace_info(&root_start);
    namespace_info.doctype_is_present = doctype_is_present;
    namespace_info.encoding = encoding;

    // Self-closed `<REQ-IF/>` (or `<REQ-IF></REQ-IF>` with no children) → empty
    // bundle. Python (`parser.py:179-182`) returns `ReqIFBundle.create_empty`
    // in this case; we preserve namespace_info verbatim because Python's
    // `create_empty` would otherwise discard schema_location / language etc.
    if was_self_closing {
        return Ok(ReqIfBundle {
            namespace_info,
            header: None,
            core_content: None,
            tool_extensions_tag_exists: false,
            tool_extensions_empty_open_close: false,
            lookup: ObjectLookup::empty(),
            exceptions: Vec::new(),
        });
    }

    let mut header: Option<ReqIfHeader> = None;
    let mut core_content: Option<CoreContent> = None;
    let mut tool_extensions_tag_exists = false;
    let mut tool_extensions_empty_open_close = false;
    let mut exceptions: Vec<SchemaWarning> = Vec::new();

    loop {
        match r.read_event()? {
            Event::Start(s) => {
                let name = s.name().as_ref().to_vec();
                match name.as_slice() {
                    b"THE-HEADER" => {
                        // `parse_header_from_reader` itself skips until it finds
                        // `<REQ-IF-HEADER>`, then consumes through its close. We
                        // must additionally consume up through `</THE-HEADER>`.
                        let owned = s.into_owned();
                        let h = parse_header_from_reader(&mut r)?;
                        header = Some(h);
                        consume_to_end(&mut r, &owned)?;
                    }
                    b"CORE-CONTENT" => {
                        let owned = s.into_owned();
                        core_content = Some(parse_core_content(&mut r, &owned, &mut exceptions)?);
                    }
                    b"TOOL-EXTENSIONS" => {
                        tool_extensions_tag_exists = true;
                        let owned = s.into_owned();
                        // Walk to the matching end, noting whether the body
                        // contained any element child (Start or Empty). If it
                        // did not, the source spelled the tag as
                        // `<TOOL-EXTENSIONS>\n  </TOOL-EXTENSIONS>\n` (empty
                        // open/close form) and we must preserve that on
                        // round-trip — `<TOOL-EXTENSIONS/>` is byte-distinct.
                        tool_extensions_empty_open_close =
                            scan_tool_extensions_empty(&mut r, &owned)?;
                    }
                    _ => {
                        exceptions.push(
                            SchemaWarning::new(format!(
                                "unknown element <{}> inside <REQ-IF>",
                                String::from_utf8_lossy(&name)
                            ))
                            .with_context("parsing REQ-IF root"),
                        );
                        let owned = s.into_owned();
                        r.skip_to_end(&owned)?;
                    }
                }
            }
            Event::Empty(s) => {
                let name = s.name().as_ref().to_vec();
                match name.as_slice() {
                    b"TOOL-EXTENSIONS" => {
                        tool_extensions_tag_exists = true;
                    }
                    b"CORE-CONTENT" => {
                        core_content = Some(CoreContent {
                            req_if_content: None,
                        });
                    }
                    b"THE-HEADER" => {
                        exceptions.push(
                            SchemaWarning::new(
                                "<THE-HEADER/> is self-closed; expected `<REQ-IF-HEADER>` child",
                            )
                            .with_context("parsing REQ-IF root"),
                        );
                    }
                    _ => {
                        exceptions.push(
                            SchemaWarning::new(format!(
                                "unknown empty element <{}> inside <REQ-IF>",
                                String::from_utf8_lossy(&name)
                            ))
                            .with_context("parsing REQ-IF root"),
                        );
                    }
                }
            }
            Event::End(e) if e.name().as_ref() == b"REQ-IF" => break,
            Event::Eof => {
                return Err(ReqIfError::Xml {
                    pos: r.buffer_position(),
                    msg: "EOF inside <REQ-IF>".into(),
                });
            }
            _ => continue,
        }
    }

    let lookup = match core_content
        .as_ref()
        .and_then(|cc| cc.req_if_content.as_ref())
    {
        Some(content) => ObjectLookup::build(content),
        None => ObjectLookup::empty(),
    };

    Ok(ReqIfBundle {
        namespace_info,
        header,
        core_content,
        tool_extensions_tag_exists,
        tool_extensions_empty_open_close,
        lookup,
        exceptions,
    })
}

/// Best-effort detection of the XML declaration. quick-xml exposes the
/// declaration as an `Event::Decl` but we want to capture it before consuming
/// it (and the parser-level fields prefer a one-shot scan over the prefix).
///
/// Returns `(doctype_is_present, encoding)`:
/// - `doctype_is_present` is `true` iff the first ≤200 bytes contain `<?xml`.
/// - `encoding` is the value of `encoding="..."` extracted from that prefix,
///   if any.
///
/// This matches the Python proxy of "docinfo.standalone is not None" closely
/// enough for the round-trip use case — every real ReqIF file ships with
/// `<?xml version="1.0" encoding="UTF-8"?>` as the first line.
fn sniff_xml_declaration(bytes: &[u8]) -> (bool, Option<String>) {
    let prefix_len = bytes.len().min(200);
    let prefix = match std::str::from_utf8(&bytes[..prefix_len]) {
        Ok(s) => s,
        Err(e) => {
            // Truncate at the last valid char boundary so we don't choke on a
            // multi-byte char split by the 200-byte window.
            let valid_up_to = e.valid_up_to();
            std::str::from_utf8(&bytes[..valid_up_to]).unwrap_or("")
        }
    };
    let Some(decl_start) = prefix.find("<?xml") else {
        return (false, None);
    };
    let after = &prefix[decl_start + 5..];
    let decl_end = after.find("?>").unwrap_or(after.len());
    let decl_body = &after[..decl_end];

    let encoding = extract_attr(decl_body, "encoding");
    (true, encoding)
}

/// Extract `name="value"` from a small attribute-list snippet. Used to pull
/// `encoding` off the XML declaration. Returns `None` if the attribute is not
/// found or the quotes are unbalanced.
fn extract_attr(body: &str, name: &str) -> Option<String> {
    let key = format!("{name}=");
    let idx = body.find(&key)?;
    let after = &body[idx + key.len()..];
    let quote = after.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let inner = &after[1..];
    let end = inner.find(quote)?;
    Some(inner[..end].to_string())
}

/// Walk events until the `<REQ-IF>` root opens, returning the start event and
/// whether the form was self-closed.
fn locate_root(r: &mut ReqIfReader<'_>) -> Result<(BytesStart<'static>, bool), ReqIfError> {
    loop {
        match r.read_event()? {
            Event::Start(s) if s.name().as_ref() == b"REQ-IF" => {
                return Ok((s.into_owned(), false));
            }
            Event::Empty(s) if s.name().as_ref() == b"REQ-IF" => return Ok((s.into_owned(), true)),
            Event::Eof => {
                return Err(ReqIfError::MissingChild {
                    child: "REQ-IF".into(),
                    parent: "<document>".into(),
                });
            }
            _ => continue,
        }
    }
}

/// Harvest the XML namespace declarations + xml:lang + xsi:schemaLocation
/// attributes off the `<REQ-IF>` start event into a [`NamespaceInfo`].
///
/// Populates the typed fields for ergonomic introspection AND
/// `attributes_in_order` with every attribute in source order. The latter is
/// what the unparser walks for byte-exact round-trip — it preserves both the
/// original ordering (which varies between vendors) and any vendor-specific
/// attributes (e.g. `xmlns:doors`, `xmlns:reqif-common`) that the typed
/// fields do not model.
///
/// `original_reqif_tag_dump` is intentionally left as `None`: the byte-exact
/// reconstruction goes through `attributes_in_order`, which is sufficient for
/// every vendor we have a fixture for and avoids storing the raw tag.
fn harvest_namespace_info(start: &BytesStart<'_>) -> NamespaceInfo {
    let mut info = NamespaceInfo::default();
    for attr in start.attributes().flatten() {
        let key_bytes = attr.key.as_ref();
        let key_str = String::from_utf8_lossy(key_bytes).into_owned();
        let value = match attr.unescape_value() {
            Ok(cow) => cow.into_owned(),
            Err(_) => String::from_utf8_lossy(&attr.value).into_owned(),
        };
        match key_bytes {
            b"xmlns" => info.namespace = Some(value.clone()),
            b"xmlns:configuration" => info.configuration = Some(value.clone()),
            b"xmlns:id" => info.namespace_id = Some(value.clone()),
            b"xmlns:xhtml" => info.namespace_xhtml = Some(value.clone()),
            b"xmlns:xsi" => info.schema_namespace = Some(value.clone()),
            b"xsi:schemaLocation" => info.schema_location = Some(value.clone()),
            b"xml:lang" => info.language = Some(value.clone()),
            // Vendor-specific attributes (xmlns:doors, xmlns:reqif-common, …)
            // are not surfaced through typed fields but ARE retained in
            // `attributes_in_order` below for byte-exact round-trip.
            _ => {}
        }
        info.attributes_in_order.push((key_str, value));
    }
    info
}

/// Drive a `<CORE-CONTENT>` block: locate its single `<REQ-IF-CONTENT>` child
/// (if present) and dispatch the body of that child.
fn parse_core_content(
    r: &mut ReqIfReader<'_>,
    core_content_start: &BytesStart<'_>,
    exceptions: &mut Vec<SchemaWarning>,
) -> Result<CoreContent, ReqIfError> {
    let mut req_if_content: Option<ReqIfContent> = None;

    loop {
        match r.read_event()? {
            Event::Start(s) if s.name().as_ref() == b"REQ-IF-CONTENT" => {
                let owned = s.into_owned();
                req_if_content = Some(parse_req_if_content(r, &owned, exceptions)?);
            }
            Event::Empty(s) if s.name().as_ref() == b"REQ-IF-CONTENT" => {
                let _ = s;
                req_if_content = Some(ReqIfContent::default());
            }
            Event::End(e) if e.name().as_ref() == core_content_start.name().as_ref() => {
                return Ok(CoreContent { req_if_content });
            }
            Event::Eof => {
                return Err(ReqIfError::Xml {
                    pos: r.buffer_position(),
                    msg: "EOF inside <CORE-CONTENT>".into(),
                });
            }
            _ => continue,
        }
    }
}

/// Drive a `<REQ-IF-CONTENT>` block: dispatch each direct child container
/// (`<DATATYPES>`, `<SPEC-TYPES>`, `<SPEC-OBJECTS>`, `<SPEC-RELATIONS>`,
/// `<SPECIFICATIONS>`, `<SPEC-RELATION-GROUPS>`) to its per-element walker.
fn parse_req_if_content(
    r: &mut ReqIfReader<'_>,
    req_if_content_start: &BytesStart<'_>,
    exceptions: &mut Vec<SchemaWarning>,
) -> Result<ReqIfContent, ReqIfError> {
    let mut content = ReqIfContent::default();

    loop {
        match r.read_event()? {
            Event::Start(s) => {
                let name = s.name().as_ref().to_vec();
                let owned = s.into_owned();
                match name.as_slice() {
                    b"DATATYPES" => {
                        let v = parse_data_types(r)?;
                        content.list_forms.data_types_empty_open_close = v.is_empty();
                        content.data_types = Some(v);
                    }
                    b"SPEC-TYPES" => {
                        let v = parse_spec_types(r)?;
                        content.list_forms.spec_types_empty_open_close = v.is_empty();
                        content.spec_types = Some(v);
                    }
                    b"SPEC-OBJECTS" => {
                        let v = parse_spec_objects(r)?;
                        content.list_forms.spec_objects_empty_open_close = v.is_empty();
                        content.spec_objects = Some(v);
                    }
                    b"SPEC-RELATIONS" => {
                        let v = parse_spec_relations(r)?;
                        content.list_forms.spec_relations_empty_open_close = v.is_empty();
                        content.spec_relations = Some(v);
                    }
                    b"SPECIFICATIONS" => {
                        let v = parse_specifications(r)?;
                        content.list_forms.specifications_empty_open_close = v.is_empty();
                        content.specifications = Some(v);
                    }
                    b"SPEC-RELATION-GROUPS" => {
                        let v = parse_relation_groups(r)?;
                        content.list_forms.relation_groups_empty_open_close = v.is_empty();
                        content.relation_groups = Some(v);
                    }
                    _ => {
                        exceptions.push(
                            SchemaWarning::new(format!(
                                "unknown element <{}> inside <REQ-IF-CONTENT>",
                                String::from_utf8_lossy(&name)
                            ))
                            .with_context("parsing REQ-IF-CONTENT"),
                        );
                        r.skip_to_end(&owned)?;
                    }
                }
            }
            Event::Empty(s) => {
                let name = s.name().as_ref().to_vec();
                match name.as_slice() {
                    b"DATATYPES" => content.data_types = Some(Vec::new()),
                    b"SPEC-TYPES" => content.spec_types = Some(Vec::new()),
                    b"SPEC-OBJECTS" => content.spec_objects = Some(Vec::new()),
                    b"SPEC-RELATIONS" => content.spec_relations = Some(Vec::new()),
                    b"SPECIFICATIONS" => content.specifications = Some(Vec::new()),
                    b"SPEC-RELATION-GROUPS" => content.relation_groups = Some(Vec::new()),
                    _ => {
                        exceptions.push(
                            SchemaWarning::new(format!(
                                "unknown empty element <{}> inside <REQ-IF-CONTENT>",
                                String::from_utf8_lossy(&name)
                            ))
                            .with_context("parsing REQ-IF-CONTENT"),
                        );
                    }
                }
            }
            Event::End(e) if e.name().as_ref() == req_if_content_start.name().as_ref() => {
                return Ok(content);
            }
            Event::Eof => {
                return Err(ReqIfError::Xml {
                    pos: r.buffer_position(),
                    msg: "EOF inside <REQ-IF-CONTENT>".into(),
                });
            }
            _ => continue,
        }
    }
}

/// Walk a `<DATATYPES>` block, dispatching each `<DATATYPE-DEFINITION-*>` child
/// to [`parse_data_type_inner`].
fn parse_data_types(r: &mut ReqIfReader<'_>) -> Result<Vec<crate::model::DataType>, ReqIfError> {
    let mut out = Vec::new();
    loop {
        match r.read_event()? {
            Event::Start(s) => {
                let tag = s.name().as_ref().to_vec();
                let owned = s.into_owned();
                out.push(parse_data_type_inner(r, &owned, &tag, false)?);
            }
            Event::Empty(s) => {
                let tag = s.name().as_ref().to_vec();
                let owned = s.into_owned();
                out.push(parse_data_type_inner(r, &owned, &tag, true)?);
            }
            Event::End(e) if e.name().as_ref() == b"DATATYPES" => return Ok(out),
            Event::Eof => {
                return Err(ReqIfError::Xml {
                    pos: r.buffer_position(),
                    msg: "EOF inside <DATATYPES>".into(),
                });
            }
            _ => continue,
        }
    }
}

/// Walk a `<SPEC-TYPES>` block, dispatching each child to
/// [`parse_spec_type_inner`].
fn parse_spec_types(r: &mut ReqIfReader<'_>) -> Result<Vec<crate::model::SpecType>, ReqIfError> {
    let mut out = Vec::new();
    loop {
        match r.read_event()? {
            Event::Start(s) => {
                let tag = s.name().as_ref().to_vec();
                let owned = s.into_owned();
                out.push(parse_spec_type_inner(r, &owned, &tag, false)?);
            }
            Event::Empty(s) => {
                let tag = s.name().as_ref().to_vec();
                let owned = s.into_owned();
                out.push(parse_spec_type_inner(r, &owned, &tag, true)?);
            }
            Event::End(e) if e.name().as_ref() == b"SPEC-TYPES" => return Ok(out),
            Event::Eof => {
                return Err(ReqIfError::Xml {
                    pos: r.buffer_position(),
                    msg: "EOF inside <SPEC-TYPES>".into(),
                });
            }
            _ => continue,
        }
    }
}

/// Walk a `<SPEC-OBJECTS>` block, dispatching each `<SPEC-OBJECT>` child to
/// [`parse_spec_object_inner`].
fn parse_spec_objects(
    r: &mut ReqIfReader<'_>,
) -> Result<Vec<crate::model::SpecObject>, ReqIfError> {
    let mut out = Vec::new();
    loop {
        match r.read_event()? {
            Event::Start(s) if s.name().as_ref() == b"SPEC-OBJECT" => {
                let owned = s.into_owned();
                out.push(parse_spec_object_inner(r, &owned, false)?);
            }
            Event::Empty(s) if s.name().as_ref() == b"SPEC-OBJECT" => {
                let owned = s.into_owned();
                out.push(parse_spec_object_inner(r, &owned, true)?);
            }
            Event::End(e) if e.name().as_ref() == b"SPEC-OBJECTS" => return Ok(out),
            Event::Eof => {
                return Err(ReqIfError::Xml {
                    pos: r.buffer_position(),
                    msg: "EOF inside <SPEC-OBJECTS>".into(),
                });
            }
            _ => continue,
        }
    }
}

/// Walk a `<SPEC-RELATIONS>` block, dispatching each `<SPEC-RELATION>` child to
/// [`parse_spec_relation_inner`].
fn parse_spec_relations(
    r: &mut ReqIfReader<'_>,
) -> Result<Vec<crate::model::SpecRelation>, ReqIfError> {
    let mut out = Vec::new();
    loop {
        match r.read_event()? {
            Event::Start(s) if s.name().as_ref() == b"SPEC-RELATION" => {
                let owned = s.into_owned();
                out.push(parse_spec_relation_inner(r, &owned, false)?);
            }
            Event::Empty(s) if s.name().as_ref() == b"SPEC-RELATION" => {
                let owned = s.into_owned();
                out.push(parse_spec_relation_inner(r, &owned, true)?);
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

/// Walk a `<SPECIFICATIONS>` block, dispatching each `<SPECIFICATION>` child to
/// [`parse_specification_inner`].
fn parse_specifications(
    r: &mut ReqIfReader<'_>,
) -> Result<Vec<crate::model::Specification>, ReqIfError> {
    let mut out = Vec::new();
    loop {
        match r.read_event()? {
            Event::Start(s) if s.name().as_ref() == b"SPECIFICATION" => {
                let owned = s.into_owned();
                out.push(parse_specification_inner(r, &owned, false)?);
            }
            Event::Empty(s) if s.name().as_ref() == b"SPECIFICATION" => {
                let owned = s.into_owned();
                out.push(parse_specification_inner(r, &owned, true)?);
            }
            Event::End(e) if e.name().as_ref() == b"SPECIFICATIONS" => return Ok(out),
            Event::Eof => {
                return Err(ReqIfError::Xml {
                    pos: r.buffer_position(),
                    msg: "EOF inside <SPECIFICATIONS>".into(),
                });
            }
            _ => continue,
        }
    }
}

/// Walk a `<SPEC-RELATION-GROUPS>` block, dispatching each `<RELATION-GROUP>`
/// child to [`parse_relation_group_inner`].
fn parse_relation_groups(
    r: &mut ReqIfReader<'_>,
) -> Result<Vec<crate::model::RelationGroup>, ReqIfError> {
    let mut out = Vec::new();
    loop {
        match r.read_event()? {
            Event::Start(s) if s.name().as_ref() == b"RELATION-GROUP" => {
                let owned = s.into_owned();
                out.push(parse_relation_group_inner(r, &owned, false)?);
            }
            Event::Empty(s) if s.name().as_ref() == b"RELATION-GROUP" => {
                let owned = s.into_owned();
                out.push(parse_relation_group_inner(r, &owned, true)?);
            }
            Event::End(e) if e.name().as_ref() == b"SPEC-RELATION-GROUPS" => return Ok(out),
            Event::Eof => {
                return Err(ReqIfError::Xml {
                    pos: r.buffer_position(),
                    msg: "EOF inside <SPEC-RELATION-GROUPS>".into(),
                });
            }
            _ => continue,
        }
    }
}

/// Consume the body of a `<TOOL-EXTENSIONS>` block (caller has just read the
/// Start event) up to and including the matching `</TOOL-EXTENSIONS>`.
/// Returns `true` iff the body contained no element children — i.e. the
/// source was `<TOOL-EXTENSIONS>\n  </TOOL-EXTENSIONS>\n` (empty open/close
/// form), not `<TOOL-EXTENSIONS><child/>...</TOOL-EXTENSIONS>`.
///
/// We treat both `Event::Start` and `Event::Empty` as "had a child"; pure
/// `Text` (whitespace, comments) does NOT count as content because the
/// unparser's placeholder body is also pure whitespace, so any content here
/// has already been lost to the model and we can only honor the empty/non-empty
/// distinction.
fn scan_tool_extensions_empty(
    r: &mut ReqIfReader<'_>,
    start: &BytesStart<'_>,
) -> Result<bool, ReqIfError> {
    let name = start.name().as_ref().to_vec();
    let mut had_child = false;
    let mut depth = 1usize;
    loop {
        match r.read_event()? {
            Event::Start(s) => {
                if depth == 1 {
                    had_child = true;
                }
                if s.name().as_ref() == name.as_slice() {
                    depth += 1;
                }
            }
            Event::Empty(_) => {
                if depth == 1 {
                    had_child = true;
                }
            }
            Event::End(e) if e.name().as_ref() == name.as_slice() => {
                depth -= 1;
                if depth == 0 {
                    return Ok(!had_child);
                }
            }
            Event::Eof => {
                return Err(ReqIfError::Xml {
                    pos: r.buffer_position(),
                    msg: "EOF inside <TOOL-EXTENSIONS>".into(),
                });
            }
            _ => continue,
        }
    }
}

/// Consume events until the matching `</TAG>` for `start`. Distinct from
/// `ReqIfReader::skip_to_end` only in that this version uses the local
/// pos-only error message style for consistency with the rest of the driver.
fn consume_to_end(r: &mut ReqIfReader<'_>, start: &BytesStart<'_>) -> Result<(), ReqIfError> {
    let name = start.name().as_ref().to_vec();
    let mut depth = 1usize;
    loop {
        match r.read_event()? {
            Event::Start(s) if s.name().as_ref() == name.as_slice() => depth += 1,
            Event::End(e) if e.name().as_ref() == name.as_slice() => {
                depth -= 1;
                if depth == 0 {
                    return Ok(());
                }
            }
            Event::Eof => {
                return Err(ReqIfError::Xml {
                    pos: r.buffer_position(),
                    msg: format!("EOF inside <{}>", String::from_utf8_lossy(&name)),
                });
            }
            _ => continue,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sniff_xml_declaration_extracts_encoding() {
        let bytes = b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<REQ-IF/>";
        let (present, enc) = sniff_xml_declaration(bytes);
        assert!(present);
        assert_eq!(enc.as_deref(), Some("UTF-8"));
    }

    #[test]
    fn sniff_xml_declaration_no_decl() {
        let bytes = b"<REQ-IF/>";
        let (present, enc) = sniff_xml_declaration(bytes);
        assert!(!present);
        assert!(enc.is_none());
    }

    #[test]
    fn sniff_xml_declaration_present_no_encoding() {
        let bytes = b"<?xml version=\"1.0\"?><REQ-IF/>";
        let (present, enc) = sniff_xml_declaration(bytes);
        assert!(present);
        assert!(enc.is_none());
    }
}
