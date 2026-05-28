//! `<SPEC-OBJECT>` element model.
//!
//! Mirrors `strict-doc-reqif/reqif/models/reqif_spec_object.py`. A spec object
//! carries the standard `IDENTIFIER` + optional common attributes, a
//! `<TYPE>/<SPEC-OBJECT-TYPE-REF>` pointer, and a `<VALUES>` list of
//! [`crate::model::AttributeValue`]s.
//!
//! The Python implementation stashes the raw `xml_node` so the unparser can
//! recover the original `<TYPE>` / `<VALUES>` child order — most tools emit
//! TYPE first, but some emit VALUES first and round-trip must preserve which.
//! Rather than carry an lxml-shaped node graph through the model, we record an
//! explicit [`SpecObjectChildTag`] sequence as the parser walks the body.

use chrono::{DateTime, FixedOffset};

use crate::error::ReqIfError;
use crate::helpers::datetime;
use crate::ids::{SpecObjectId, SpecTypeId};
use crate::model::AttributeValue;

/// Marker recording which of the two children was seen during parse, in source
/// order. The unparser iterates this list and re-emits the children in the
/// recorded order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpecObjectChildTag {
    Type,
    Values,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecObject {
    pub identifier: SpecObjectId,
    pub description: Option<String>,
    pub last_change: Option<String>,
    pub long_name: Option<String>,
    pub spec_object_type: SpecTypeId,
    pub attributes: Vec<AttributeValue>,
    /// Source order of `<TYPE>` and `<VALUES>` children. Always exactly two
    /// entries for a well-formed `<SPEC-OBJECT>`; either ordering is legal.
    pub children_order: Vec<SpecObjectChildTag>,
    /// Inline `<!-- ... -->` comments captured between the previous sibling
    /// (or `<SPEC-OBJECTS>` open) and this element. Each string is the
    /// comment body (no `<!--` / `-->` delimiters), in source order. Round-trip
    /// emits one comment per line above the element. Defaults to `vec![]`.
    pub comments_before: Vec<String>,
}

impl SpecObject {
    /// Lazily parse `last_change` as a typed [`DateTime`].
    ///
    /// Returns `None` when the source had no `<LAST-CHANGE>` attribute. The
    /// raw string in [`Self::last_change`] is preserved unchanged so
    /// byte-fidelity round-trip is unaffected.
    pub fn last_change_parsed(&self) -> Option<Result<DateTime<FixedOffset>, ReqIfError>> {
        self.last_change.as_deref().map(datetime::parse)
    }
}
