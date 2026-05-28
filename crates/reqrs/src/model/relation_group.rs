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

use chrono::{DateTime, FixedOffset};

use crate::error::ReqIfError;
use crate::helpers::datetime;
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

impl RelationGroup {
    /// Lazily parse `last_change` as a typed [`DateTime`].
    ///
    /// Returns `None` when the source had no `<LAST-CHANGE>` attribute. The
    /// raw string in [`Self::last_change`] is preserved unchanged so
    /// byte-fidelity round-trip is unaffected.
    pub fn last_change_parsed(&self) -> Option<Result<DateTime<FixedOffset>, ReqIfError>> {
        self.last_change.as_deref().map(datetime::parse)
    }
}
