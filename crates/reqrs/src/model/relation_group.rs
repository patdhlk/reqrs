//! `<RELATION-GROUP>` element model.
//!
//! Mirrors `strict-doc-reqif/reqif/models/reqif_relation_group.py`. A relation
//! group bundles a list of [`crate::model::SpecRelation`]s by id under a
//! `<RELATION-GROUP-TYPE>` and ties them to a source/target
//! [`crate::model::Specification`].
//!
//! Note: the singular tag is `<RELATION-GROUP>` (not
//! `<SPEC-RELATION-GROUP>`). The plural container in the ReqIF schema is
//! `<SPEC-RELATION-GROUPS>` but each child element is `<RELATION-GROUP>`.

use crate::ids::{RelationGroupId, SpecRelationId, SpecTypeId, SpecificationId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelationGroup {
    pub identifier: RelationGroupId,
    pub description: Option<String>,
    pub last_change: Option<String>,
    pub long_name: Option<String>,
    /// Target of `<TYPE>/<RELATION-GROUP-TYPE-REF>`.
    pub group_type: SpecTypeId,
    /// Target of `<SOURCE-SPECIFICATION>/<SPECIFICATION-REF>`.
    pub source_specification: SpecificationId,
    /// Target of `<TARGET-SPECIFICATION>/<SPECIFICATION-REF>`.
    pub target_specification: SpecificationId,
    /// Optional `<SPEC-RELATIONS>` block listing
    /// `<SPEC-RELATION-REF>` text-content children. `None` means the source
    /// had no `<SPEC-RELATIONS>` element.
    pub spec_relations: Option<Vec<SpecRelationId>>,
}
