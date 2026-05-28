//! `<REQ-IF>` outer element + XML prologue capture.
//!
//! Mirrors `strict-doc-reqif/reqif/models/reqif_namespace_info.py`. The Python
//! reference snapshots the namespace / schema / xml declaration metadata that
//! lives on the root `<REQ-IF>` element together with the optional DOCTYPE and
//! XML declaration so they can be re-emitted verbatim on round-trip.
//!
//! - `original_reqif_tag_dump` — raw text of the `<REQ-IF ...>` opening tag.
//!   Captured because vendors emit the namespace attribute set in different
//!   orders and on multiple lines; preserving the literal bytes avoids spurious
//!   diffs on round-trip.
//! - `doctype_is_present` — whether a `<!DOCTYPE>` line was seen.
//! - `encoding` — value of `encoding=` on the XML declaration. Kept as
//!   `Option<String>` so a synthetic bundle created via [`Default`] starts as
//!   `None`; the unparser may fall back to a sensible default.
//! - The remaining `Option<String>` fields are the attributes
//!   (`xmlns`/`configuration`/`xmlns:id`/`xmlns:xhtml`/`xmlns:xsi`/
//!   `xsi:schemaLocation`/`xml:lang`) on the `<REQ-IF>` element.

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NamespaceInfo {
    pub original_reqif_tag_dump: Option<String>,
    pub doctype_is_present: bool,
    pub encoding: Option<String>,
    pub namespace: Option<String>,
    pub configuration: Option<String>,
    pub namespace_id: Option<String>,
    pub namespace_xhtml: Option<String>,
    pub schema_namespace: Option<String>,
    pub schema_location: Option<String>,
    pub language: Option<String>,
}
