//! `<SPEC-RELATION>` element model.
//!
//! Mirrors `strict-doc-reqif/reqif/models/reqif_spec_relation.py`. A spec
//! relation links one [`crate::model::SpecObject`] (`source`) to another
//! (`target`), tagged by a [`crate::model::SpecRelationType`] reference
//! (`relation_type`), and optionally carrying a `<VALUES>` list of
//! [`crate::model::AttributeValue`]s.
//!
//! The Python implementation stashes the raw `xml_node` so the unparser can
//! recover the original child order — vendors disagree: the canonical order
//! is TYPE → SOURCE → TARGET → VALUES, but Polarion / ReqIF Studio emit
//! VALUES first and SparxSystems emits SOURCE first. Rather than carry an
//! lxml-shaped node graph through the model, we record an explicit
//! [`SpecRelationChildTag`] sequence as the parser walks the body.

use crate::ids::{SpecObjectId, SpecRelationId, SpecTypeId};
use crate::model::AttributeValue;

/// Marker recording which child was seen during parse, in source order. The
/// unparser iterates this list to re-emit children in the original order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpecRelationChildTag {
    Type,
    Source,
    Target,
    Values,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecRelation {
    pub identifier: SpecRelationId,
    pub description: Option<String>,
    pub last_change: Option<String>,
    pub long_name: Option<String>,
    /// Target of `<TYPE>/<SPEC-RELATION-TYPE-REF>`.
    pub relation_type: SpecTypeId,
    /// Target of `<SOURCE>/<SPEC-OBJECT-REF>`.
    pub source: SpecObjectId,
    /// Target of `<TARGET>/<SPEC-OBJECT-REF>`.
    pub target: SpecObjectId,
    /// Optional `<VALUES>` block. `None` means the source had no `<VALUES>` at
    /// all; the Python reference treats this as the common case.
    pub values: Option<Vec<AttributeValue>>,
    /// Source order of the body children. Populated by the parser; the
    /// unparser iterates it to preserve vendor-specific orderings. When empty
    /// (e.g. synthetic construction), the unparser falls back to the canonical
    /// `[Type, Source, Target, Values]` order.
    pub children_order: Vec<SpecRelationChildTag>,
}
