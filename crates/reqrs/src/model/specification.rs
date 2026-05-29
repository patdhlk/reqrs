//! `<SPECIFICATION>` element model.
//!
//! Mirrors `strict-doc-reqif/reqif/models/reqif_specification.py`. A
//! specification is structurally similar to a [`crate::model::SpecObject`] but
//! carries a `<CHILDREN>` block of [`crate::model::SpecHierarchy`] nodes plus
//! optional `<TYPE>/<SPECIFICATION-TYPE-REF>` and `<VALUES>` siblings.
//!
//! The three children (`TYPE`, `CHILDREN`, `VALUES`) appear in different
//! permutations across vendors. The Python reference recovers the original
//! order via `xml_node`; we record an explicit [`SpecificationChildTag`]
//! sequence as the parser walks the body.

use chrono::{DateTime, FixedOffset};

use crate::error::ReqIfError;
use crate::helpers::datetime;
use crate::ids::{SpecTypeId, SpecificationId};
use crate::model::{AttributeValue, SpecHierarchy};

/// Marker recording which of the three children was seen during parse, in
/// source order. The unparser iterates this list and re-emits the children in
/// the recorded order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpecificationChildTag {
    Type,
    Children,
    Values,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Specification {
    pub identifier: SpecificationId,
    pub description: Option<String>,
    pub last_change: Option<String>,
    pub long_name: Option<String>,
    pub specification_type: Option<SpecTypeId>,
    pub values: Option<Vec<AttributeValue>>,
    pub children: Option<Vec<SpecHierarchy>>,
    /// Source order of `<TYPE>`, `<CHILDREN>`, `<VALUES>` children — recorded
    /// only for children actually present in the source.
    pub children_order: Vec<SpecificationChildTag>,
    /// True iff the source had `<CHILDREN>\n          </CHILDREN>\n` (empty
    /// open/close form). False (default) means the source had `<CHILDREN/>`
    /// or `children` is `None` or non-empty. The unparser consults this flag
    /// only when `children` is `Some(vec![])`; otherwise it has no effect.
    /// Synthetic Specifications built via field init leave this `false`,
    /// matching the legacy self-closed default.
    pub children_empty_open_close: bool,
    /// True iff the source had `<VALUES>\n          </VALUES>\n` (empty
    /// open/close form). Same semantics as [`Self::children_empty_open_close`].
    pub values_empty_open_close: bool,
    /// Inline `<!-- ... -->` comments captured between the previous sibling
    /// (or `<SPECIFICATIONS>` open) and this element. Each string is the
    /// comment body (no `<!--` / `-->` delimiters), in source order. Round-trip
    /// emits one comment per line above the element using the element's own
    /// indent. Defaults to `vec![]` when the source had no comments or when
    /// the value is constructed synthetically.
    pub comments_before: Vec<String>,
}

impl Specification {
    /// Lazily parse `last_change` as a typed [`DateTime`].
    ///
    /// Returns `None` when the source had no `<LAST-CHANGE>` attribute. The
    /// raw string in [`Self::last_change`] is preserved unchanged so
    /// byte-fidelity round-trip is unaffected.
    pub fn last_change_parsed(&self) -> Option<Result<DateTime<FixedOffset>, ReqIfError>> {
        self.last_change.as_deref().map(datetime::parse)
    }
}
