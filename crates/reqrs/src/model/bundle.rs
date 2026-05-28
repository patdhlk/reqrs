//! Top-level container for one parsed ReqIF document.
//!
//! Mirrors `strict-doc-reqif/reqif/reqif_bundle.py`. A [`ReqIfBundle`]
//! aggregates everything captured from one `<REQ-IF>` document:
//!
//! - [`NamespaceInfo`] — the XML prologue + root element attributes.
//! - [`ReqIfHeader`] — the `<THE-HEADER>` block.
//! - [`CoreContent`] — the `<CORE-CONTENT>/<REQ-IF-CONTENT>` body.
//! - `tool_extensions_tag_exists` — whether the source had a
//!   `<TOOL-EXTENSIONS>` element. We don't yet model the contents of tool
//!   extensions (vendor-specific opaque XML); this boolean simply records
//!   that the tag was present so the unparser can re-emit a placeholder.
//! - `tool_extensions_empty_open_close` — companion of
//!   `tool_extensions_tag_exists`. True iff the source spelled the (empty)
//!   tag as `<TOOL-EXTENSIONS>\n  </TOOL-EXTENSIONS>\n` (open/close form);
//!   false means `<TOOL-EXTENSIONS/>` (self-closed) or the bundle is
//!   synthetic. Only consulted when `tool_extensions_tag_exists` is true.
//! - [`ObjectLookup`] — pre-built indexes for reference resolution.
//! - `exceptions` — non-fatal [`SchemaWarning`]s accumulated during parse.
//!
//! Note: [`ReqIfBundle`] does NOT derive [`PartialEq`] because
//! [`ObjectLookup`] contains [`std::sync::Arc`] values that compare by
//! pointer identity, not structural equality. Equality of two bundles
//! parsed from the same bytes is therefore not a useful comparison through
//! `==`; tests that need it should compare individual fields (the model
//! payload deriveds `PartialEq` field-by-field).

use crate::error::SchemaWarning;
use crate::model::{CoreContent, NamespaceInfo, ObjectLookup, ReqIfHeader};

#[derive(Debug, Clone)]
pub struct ReqIfBundle {
    pub namespace_info: NamespaceInfo,
    pub header: Option<ReqIfHeader>,
    pub core_content: Option<CoreContent>,
    pub tool_extensions_tag_exists: bool,
    /// True iff the source had the empty open/close form
    /// `<TOOL-EXTENSIONS>\n  </TOOL-EXTENSIONS>\n`. Ignored when
    /// `tool_extensions_tag_exists` is false.
    pub tool_extensions_empty_open_close: bool,
    pub lookup: ObjectLookup,
    pub exceptions: Vec<SchemaWarning>,
}

impl ReqIfBundle {
    /// Construct a synthetic empty bundle. Used by callers that want to
    /// programmatically build a ReqIF document from scratch (no parse).
    ///
    /// `namespace` and `configuration` populate the corresponding attributes
    /// on [`NamespaceInfo`]; all other namespace fields default to `None`
    /// and `doctype_is_present` is `false`.
    pub fn empty(namespace: Option<String>, configuration: Option<String>) -> Self {
        Self {
            namespace_info: NamespaceInfo {
                namespace,
                configuration,
                ..Default::default()
            },
            header: None,
            core_content: None,
            tool_extensions_tag_exists: false,
            tool_extensions_empty_open_close: false,
            lookup: ObjectLookup::empty(),
            exceptions: Vec::new(),
        }
    }
}
