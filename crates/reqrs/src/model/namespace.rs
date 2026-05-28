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
//! - The remaining `Option<String>` fields are the *recognized* attributes
//!   (`xmlns`/`configuration`/`xmlns:id`/`xmlns:xhtml`/`xmlns:xsi`/
//!   `xsi:schemaLocation`/`xml:lang`) on the `<REQ-IF>` element. These give
//!   ergonomic typed access for callers that need to introspect a specific
//!   namespace.
//! - `attributes_in_order` captures *every* `<REQ-IF>` attribute (recognized
//!   or vendor-specific) in source order as `(qualified-name, value)` tuples.
//!   This is the round-trip-fidelity source-of-truth: the unparser walks it
//!   to emit the opener byte-exact. If empty (e.g. a synthetic bundle built
//!   via [`Default`]), the unparser falls back to a canonical attribute order
//!   built from the typed fields.

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
    /// All `<REQ-IF>` attributes in their original source order. Each entry is
    /// `(qualified-name, value)`. Includes recognized attributes (which also
    /// surface in the typed fields above) as well as vendor-specific ones such
    /// as `xmlns:doors`, `xmlns:reqif-common`, etc. that the typed fields do
    /// not model. Populated by the parser; the unparser prefers this vector
    /// over the typed fields whenever it is non-empty.
    pub attributes_in_order: Vec<(String, String)>,
}
