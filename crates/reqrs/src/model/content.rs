//! `<REQ-IF-CONTENT>` element model — container for the body of a ReqIF doc.
//!
//! Mirrors `strict-doc-reqif/reqif/models/reqif_req_if_content.py`. Each
//! field is `Option<Vec<_>>` to keep the three-state semantics that the rest
//! of the model uses for absent / empty / populated containers:
//!
//! - `None` — the source had no corresponding `<DATATYPES>` / `<SPEC-TYPES>` /
//!   `<SPEC-OBJECTS>` / `<SPEC-RELATIONS>` / `<SPECIFICATIONS>` /
//!   `<SPEC-RELATION-GROUPS>` element at all.
//! - `Some(vec![])` — the source had an empty container (self-closed or
//!   explicit open+close).
//! - `Some(vec![..])` — the source had one or more children.
//!
//! The field name `relation_groups` (rather than the Python
//! `spec_relation_groups`) is intentional: it matches the model type
//! [`crate::model::RelationGroup`] (the ReqIF schema element is
//! `<RELATION-GROUP>` even though the plural container is
//! `<SPEC-RELATION-GROUPS>`).

use crate::model::{DataType, RelationGroup, SpecObject, SpecRelation, SpecType, Specification};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ReqIfContent {
    pub data_types: Option<Vec<DataType>>,
    pub spec_types: Option<Vec<SpecType>>,
    pub spec_objects: Option<Vec<SpecObject>>,
    pub spec_relations: Option<Vec<SpecRelation>>,
    pub specifications: Option<Vec<Specification>>,
    pub relation_groups: Option<Vec<RelationGroup>>,
}
