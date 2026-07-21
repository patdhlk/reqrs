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
//! - records `<TOOL-EXTENSIONS>` as a [`ToolExtensions`] value capturing
//!   presence, form (self-closed vs empty open/close), and — when non-empty —
//!   the verbatim inner XML bytes for byte-exact round-trip.
//!
//! Unknown elements are skipped with an [`Issue`] (Python behaviour).

use crate::error::{Issue, IssueKind, Location, ReqIfError};
use crate::model::{
    CoreContent, NamespaceInfo, ObjectLookup, ReqIfBundle, ReqIfContent, ReqIfHeader,
    ToolExtensions,
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
            tool_extensions: ToolExtensions::Absent,
            lookup: ObjectLookup::empty(),
            exceptions: Vec::new(),
        });
    }

    let mut header: Option<ReqIfHeader> = None;
    let mut core_content: Option<CoreContent> = None;
    let mut tool_extensions = ToolExtensions::Absent;
    let mut exceptions: Vec<Issue> = Vec::new();

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
                        let owned = s.into_owned();
                        // Capture the inner bytes verbatim. We must distinguish
                        // three open/close cases for byte-exact round-trip:
                        //
                        //   `<TOOL-EXTENSIONS>...non-empty...</TOOL-EXTENSIONS>`
                        //       → `Content(raw)` — preserve verbatim. Vendor
                        //         payloads (Polarion, Doors, etc.) live here.
                        //
                        //   `<TOOL-EXTENSIONS>\n  </TOOL-EXTENSIONS>`
                        //       → `EmptyOpenClose` — whitespace-only body. The
                        //         indentation/newlines themselves are vendor
                        //         convention and our unparser regenerates a
                        //         canonical form, so we don't preserve them
                        //         byte-for-byte; we only honor the open/close
                        //         vs self-close distinction.
                        //
                        //   (separately) `<TOOL-EXTENSIONS/>` → `SelfClosed`,
                        //         handled in the `Event::Empty` arm below.
                        let raw = r.capture_inner_raw(&owned)?;
                        tool_extensions = if raw.trim().is_empty() {
                            ToolExtensions::EmptyOpenClose
                        } else {
                            ToolExtensions::Content(raw)
                        };
                    }
                    _ => {
                        let tag = String::from_utf8_lossy(&name).into_owned();
                        let owned = s.into_owned();
                        let byte_offset = r.buffer_position() as u64;
                        exceptions.push(
                            Issue::new(IssueKind::UnknownElement {
                                tag,
                                parent: "REQ-IF".into(),
                            })
                            .with_location(Location::Xml { byte_offset })
                            .with_context("parsing REQ-IF root"),
                        );
                        r.skip_to_end(&owned)?;
                    }
                }
            }
            Event::Empty(s) => {
                let name = s.name().as_ref().to_vec();
                drop(s);
                match name.as_slice() {
                    b"TOOL-EXTENSIONS" => {
                        tool_extensions = ToolExtensions::SelfClosed;
                    }
                    b"CORE-CONTENT" => {
                        core_content = Some(CoreContent {
                            req_if_content: None,
                        });
                    }
                    b"THE-HEADER" => {
                        let byte_offset = r.buffer_position() as u64;
                        exceptions.push(
                            Issue::new(IssueKind::ExpectedNonEmptyElement {
                                tag: "THE-HEADER".into(),
                                parent: "REQ-IF".into(),
                            })
                            .with_location(Location::Xml { byte_offset })
                            .with_context("parsing REQ-IF root"),
                        );
                    }
                    _ => {
                        let tag = String::from_utf8_lossy(&name).into_owned();
                        let byte_offset = r.buffer_position() as u64;
                        exceptions.push(
                            Issue::new(IssueKind::UnknownElement {
                                tag,
                                parent: "REQ-IF".into(),
                            })
                            .with_location(Location::Xml { byte_offset })
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
        tool_extensions,
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
    exceptions: &mut Vec<Issue>,
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
    exceptions: &mut Vec<Issue>,
) -> Result<ReqIfContent, ReqIfError> {
    let mut content = ReqIfContent::default();

    loop {
        match r.read_event()? {
            Event::Start(s) => {
                let name = s.name().as_ref().to_vec();
                let owned = s.into_owned();
                match name.as_slice() {
                    b"DATATYPES" => {
                        let (v, trailing) = parse_data_types(r)?;
                        content.list_forms.data_types_empty_open_close = v.is_empty();
                        content.data_types = Some(v);
                        content.data_types_trailing_comments = trailing;
                    }
                    b"SPEC-TYPES" => {
                        let (v, trailing) = parse_spec_types(r)?;
                        content.list_forms.spec_types_empty_open_close = v.is_empty();
                        content.spec_types = Some(v);
                        content.spec_types_trailing_comments = trailing;
                    }
                    b"SPEC-OBJECTS" => {
                        let (v, trailing) = parse_spec_objects(r)?;
                        content.list_forms.spec_objects_empty_open_close = v.is_empty();
                        content.spec_objects = Some(v);
                        content.spec_objects_trailing_comments = trailing;
                    }
                    b"SPEC-RELATIONS" => {
                        let (v, trailing) = parse_spec_relations(r)?;
                        content.list_forms.spec_relations_empty_open_close = v.is_empty();
                        content.spec_relations = Some(v);
                        content.spec_relations_trailing_comments = trailing;
                    }
                    b"SPECIFICATIONS" => {
                        let (v, trailing) = parse_specifications(r)?;
                        content.list_forms.specifications_empty_open_close = v.is_empty();
                        content.specifications = Some(v);
                        content.specifications_trailing_comments = trailing;
                    }
                    b"SPEC-RELATION-GROUPS" => {
                        let (v, trailing) = parse_relation_groups(r)?;
                        content.list_forms.relation_groups_empty_open_close = v.is_empty();
                        content.relation_groups = Some(v);
                        content.relation_groups_trailing_comments = trailing;
                    }
                    _ => {
                        let tag = String::from_utf8_lossy(&name).into_owned();
                        let byte_offset = r.buffer_position() as u64;
                        exceptions.push(
                            Issue::new(IssueKind::UnknownElement {
                                tag,
                                parent: "REQ-IF-CONTENT".into(),
                            })
                            .with_location(Location::Xml { byte_offset })
                            .with_context("parsing REQ-IF-CONTENT"),
                        );
                        r.skip_to_end(&owned)?;
                    }
                }
            }
            Event::Empty(s) => {
                let name = s.name().as_ref().to_vec();
                drop(s);
                match name.as_slice() {
                    b"DATATYPES" => content.data_types = Some(Vec::new()),
                    b"SPEC-TYPES" => content.spec_types = Some(Vec::new()),
                    b"SPEC-OBJECTS" => content.spec_objects = Some(Vec::new()),
                    b"SPEC-RELATIONS" => content.spec_relations = Some(Vec::new()),
                    b"SPECIFICATIONS" => content.specifications = Some(Vec::new()),
                    b"SPEC-RELATION-GROUPS" => content.relation_groups = Some(Vec::new()),
                    _ => {
                        let tag = String::from_utf8_lossy(&name).into_owned();
                        let byte_offset = r.buffer_position() as u64;
                        exceptions.push(
                            Issue::new(IssueKind::UnknownElement {
                                tag,
                                parent: "REQ-IF-CONTENT".into(),
                            })
                            .with_location(Location::Xml { byte_offset })
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
/// to [`parse_data_type_inner`]. Inter-sibling `Event::Comment` events are
/// accumulated and attached to the next element's `common.comments_before`.
/// Comments that appear AFTER the last sibling and before the closing
/// `</DATATYPES>` end up in the returned trailing-comments vec — the caller
/// stashes them in `ReqIfContent::data_types_trailing_comments` so they
/// survive round-trip.
fn parse_data_types(
    r: &mut ReqIfReader<'_>,
) -> Result<(Vec<crate::model::DataType>, Vec<String>), ReqIfError> {
    let mut out = Vec::new();
    let mut pending_comments: Vec<String> = Vec::new();
    loop {
        match r.read_event()? {
            Event::Comment(c) => {
                pending_comments.push(String::from_utf8_lossy(c.as_ref()).into_owned());
            }
            Event::Start(s) => {
                let tag = s.name().as_ref().to_vec();
                let owned = s.into_owned();
                let mut dt = parse_data_type_inner(r, &owned, &tag, false)?;
                attach_data_type_comments(&mut dt, std::mem::take(&mut pending_comments));
                out.push(dt);
            }
            Event::Empty(s) => {
                let tag = s.name().as_ref().to_vec();
                let owned = s.into_owned();
                let mut dt = parse_data_type_inner(r, &owned, &tag, true)?;
                attach_data_type_comments(&mut dt, std::mem::take(&mut pending_comments));
                out.push(dt);
            }
            Event::End(e) if e.name().as_ref() == b"DATATYPES" => {
                return Ok((out, pending_comments));
            }
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

/// Stash `comments` on the variant's `DataTypeCommon.comments_before`.
fn attach_data_type_comments(dt: &mut crate::model::DataType, comments: Vec<String>) {
    use crate::model::DataType;
    let common = match dt {
        DataType::String(d) => &mut d.common,
        DataType::Boolean(d) => &mut d.common,
        DataType::Integer(d) => &mut d.common,
        DataType::Real(d) => &mut d.common,
        DataType::Date(d) => &mut d.common,
        DataType::Xhtml(d) => &mut d.common,
        DataType::Enumeration(d) => &mut d.common,
    };
    common.comments_before = comments;
}

/// Walk a `<SPEC-TYPES>` block, dispatching each child to
/// [`parse_spec_type_inner`]. Inter-sibling `Event::Comment` events are
/// accumulated and attached to the next element's `comments_before`. Comments
/// that appear AFTER the last sibling end up in the returned
/// trailing-comments vec.
fn parse_spec_types(
    r: &mut ReqIfReader<'_>,
) -> Result<(Vec<crate::model::SpecType>, Vec<String>), ReqIfError> {
    use crate::model::SpecType;
    let mut out: Vec<SpecType> = Vec::new();
    let mut pending_comments: Vec<String> = Vec::new();
    loop {
        match r.read_event()? {
            Event::Comment(c) => {
                pending_comments.push(String::from_utf8_lossy(c.as_ref()).into_owned());
            }
            Event::Start(s) => {
                let tag = s.name().as_ref().to_vec();
                let owned = s.into_owned();
                let mut st = parse_spec_type_inner(r, &owned, &tag, false)?;
                attach_spec_type_comments(&mut st, std::mem::take(&mut pending_comments));
                out.push(st);
            }
            Event::Empty(s) => {
                let tag = s.name().as_ref().to_vec();
                let owned = s.into_owned();
                let mut st = parse_spec_type_inner(r, &owned, &tag, true)?;
                attach_spec_type_comments(&mut st, std::mem::take(&mut pending_comments));
                out.push(st);
            }
            Event::End(e) if e.name().as_ref() == b"SPEC-TYPES" => {
                return Ok((out, pending_comments));
            }
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

/// Stash `comments` on the variant's `SpecTypeCommon.comments_before`.
fn attach_spec_type_comments(st: &mut crate::model::SpecType, comments: Vec<String>) {
    use crate::model::SpecType;
    let common = match st {
        SpecType::SpecObject(t) => &mut t.common,
        SpecType::Specification(t) => &mut t.common,
        SpecType::SpecRelation(t) => &mut t.common,
        SpecType::RelationGroup(t) => &mut t.common,
    };
    common.comments_before = comments;
}

/// Walk a `<SPEC-OBJECTS>` block, dispatching each `<SPEC-OBJECT>` child to
/// [`parse_spec_object_inner`]. Inter-sibling `Event::Comment` events are
/// accumulated and attached to the next element's `comments_before`. Comments
/// that appear AFTER the last sibling end up in the returned
/// trailing-comments vec.
fn parse_spec_objects(
    r: &mut ReqIfReader<'_>,
) -> Result<(Vec<crate::model::SpecObject>, Vec<String>), ReqIfError> {
    let mut out = Vec::new();
    let mut pending_comments: Vec<String> = Vec::new();
    loop {
        match r.read_event()? {
            Event::Comment(c) => {
                pending_comments.push(String::from_utf8_lossy(c.as_ref()).into_owned());
            }
            Event::Start(s) if s.name().as_ref() == b"SPEC-OBJECT" => {
                let owned = s.into_owned();
                let mut so = parse_spec_object_inner(r, &owned, false)?;
                so.comments_before = std::mem::take(&mut pending_comments);
                out.push(so);
            }
            Event::Empty(s) if s.name().as_ref() == b"SPEC-OBJECT" => {
                let owned = s.into_owned();
                let mut so = parse_spec_object_inner(r, &owned, true)?;
                so.comments_before = std::mem::take(&mut pending_comments);
                out.push(so);
            }
            Event::End(e) if e.name().as_ref() == b"SPEC-OBJECTS" => {
                return Ok((out, pending_comments));
            }
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
/// [`parse_spec_relation_inner`]. Inter-sibling `Event::Comment` events are
/// accumulated and attached to the next element's `comments_before`. Comments
/// that appear AFTER the last sibling end up in the returned
/// trailing-comments vec.
fn parse_spec_relations(
    r: &mut ReqIfReader<'_>,
) -> Result<(Vec<crate::model::SpecRelation>, Vec<String>), ReqIfError> {
    let mut out = Vec::new();
    let mut pending_comments: Vec<String> = Vec::new();
    loop {
        match r.read_event()? {
            Event::Comment(c) => {
                pending_comments.push(String::from_utf8_lossy(c.as_ref()).into_owned());
            }
            Event::Start(s) if s.name().as_ref() == b"SPEC-RELATION" => {
                let owned = s.into_owned();
                let mut sr = parse_spec_relation_inner(r, &owned, false)?;
                sr.comments_before = std::mem::take(&mut pending_comments);
                out.push(sr);
            }
            Event::Empty(s) if s.name().as_ref() == b"SPEC-RELATION" => {
                let owned = s.into_owned();
                let mut sr = parse_spec_relation_inner(r, &owned, true)?;
                sr.comments_before = std::mem::take(&mut pending_comments);
                out.push(sr);
            }
            Event::End(e) if e.name().as_ref() == b"SPEC-RELATIONS" => {
                return Ok((out, pending_comments));
            }
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
/// [`parse_specification_inner`]. Inter-sibling `Event::Comment` events are
/// accumulated and attached to the next element's `comments_before`. Comments
/// that appear AFTER the last sibling end up in the returned
/// trailing-comments vec.
fn parse_specifications(
    r: &mut ReqIfReader<'_>,
) -> Result<(Vec<crate::model::Specification>, Vec<String>), ReqIfError> {
    let mut out = Vec::new();
    let mut pending_comments: Vec<String> = Vec::new();
    loop {
        match r.read_event()? {
            Event::Comment(c) => {
                pending_comments.push(String::from_utf8_lossy(c.as_ref()).into_owned());
            }
            Event::Start(s) if s.name().as_ref() == b"SPECIFICATION" => {
                let owned = s.into_owned();
                let mut spec = parse_specification_inner(r, &owned, false)?;
                spec.comments_before = std::mem::take(&mut pending_comments);
                out.push(spec);
            }
            Event::Empty(s) if s.name().as_ref() == b"SPECIFICATION" => {
                let owned = s.into_owned();
                let mut spec = parse_specification_inner(r, &owned, true)?;
                spec.comments_before = std::mem::take(&mut pending_comments);
                out.push(spec);
            }
            Event::End(e) if e.name().as_ref() == b"SPECIFICATIONS" => {
                return Ok((out, pending_comments));
            }
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
/// child to [`parse_relation_group_inner`]. Inter-sibling `Event::Comment`
/// events are accumulated and attached to the next element's
/// `comments_before`. Comments that appear AFTER the last sibling end up in
/// the returned trailing-comments vec.
fn parse_relation_groups(
    r: &mut ReqIfReader<'_>,
) -> Result<(Vec<crate::model::RelationGroup>, Vec<String>), ReqIfError> {
    let mut out = Vec::new();
    let mut pending_comments: Vec<String> = Vec::new();
    loop {
        match r.read_event()? {
            Event::Comment(c) => {
                pending_comments.push(String::from_utf8_lossy(c.as_ref()).into_owned());
            }
            Event::Start(s) if s.name().as_ref() == b"RELATION-GROUP" => {
                let owned = s.into_owned();
                let mut rg = parse_relation_group_inner(r, &owned, false)?;
                rg.comments_before = std::mem::take(&mut pending_comments);
                out.push(rg);
            }
            Event::Empty(s) if s.name().as_ref() == b"RELATION-GROUP" => {
                let owned = s.into_owned();
                let mut rg = parse_relation_group_inner(r, &owned, true)?;
                rg.comments_before = std::mem::take(&mut pending_comments);
                out.push(rg);
            }
            Event::End(e) if e.name().as_ref() == b"SPEC-RELATION-GROUPS" => {
                return Ok((out, pending_comments));
            }
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
