//! `<SPEC-RELATION>` element model.
//!
//! Mirrors `strict-doc-reqif/reqif/models/reqif_spec_relation.py`. A spec
//! relation links one [`crate::model::SpecObject`] (`source`) to another
//! (`target`), tagged by a [`crate::model::SpecRelationType`] reference
//! (`relation_type`), and optionally carrying a `<VALUES>` list of
//! [`crate::model::AttributeValue`]s.

use crate::ids::{SpecObjectId, SpecRelationId, SpecTypeId};
use crate::model::AttributeValue;

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
}
