//! `<SPEC-HIERARCHY>` element model.
//!
//! Mirrors `strict-doc-reqif/reqif/models/reqif_spec_hierarchy.py`. A
//! spec-hierarchy node references one [`crate::model::SpecObject`] via
//! `<OBJECT>/<SPEC-OBJECT-REF>` and optionally nests further `<SPEC-HIERARCHY>`
//! children inside `<CHILDREN>`. Vendors differ on the order of those two
//! sibling elements (some emit OBJECT first, others CHILDREN first); the
//! parser records which arrangement the source used so the unparser can
//! preserve it.
//!
//! `level` is not part of the ReqIF schema — it is a derived depth counter
//! the Python reference uses to compute indentation via
//! `calculate_base_level() = 12 + (level - 1) * 4`. We carry the same
//! invariant.

use crate::ids::SpecObjectId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecHierarchy {
    /// `IDENTIFIER` attribute. Held as a bare `String` to mirror the Python
    /// model — there is no `SpecHierarchyId` newtype in the catalog.
    pub identifier: String,
    pub last_change: Option<String>,
    pub long_name: Option<String>,
    pub editable: Option<bool>,
    pub is_table_internal: Option<bool>,
    /// Target of `<OBJECT>/<SPEC-OBJECT-REF>`.
    pub spec_object_ref: SpecObjectId,
    /// Optional `<CHILDREN>` block. `None` means the source had no `<CHILDREN>`
    /// at all; `Some(vec![])` means the source had an empty `<CHILDREN/>` or
    /// `<CHILDREN></CHILDREN>` (see `was_self_closing_children`).
    pub children: Option<Vec<SpecHierarchy>>,
    /// `true` when the source had `<OBJECT>` before `<CHILDREN>`. `false` when
    /// `<CHILDREN>` came first.
    pub ref_then_children_order: bool,
    /// Depth (1-based) — controls indentation per
    /// `calculate_base_level() = 12 + (level - 1) * 4`.
    pub level: usize,
    /// `true` when the source had `<CHILDREN/>` self-closed. Only meaningful
    /// alongside `children == Some(vec![])`.
    pub was_self_closing_children: bool,
}
