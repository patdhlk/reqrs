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
//! For empty containers we additionally remember whether the source spelled
//! them self-closed (`<X/>`) or open/close (`<X></X>`) so the unparser can
//! emit byte-identical output. That bookkeeping lives in [`ListForms`].
//!
//! The field name `relation_groups` (rather than the Python
//! `spec_relation_groups`) is intentional: it matches the model type
//! [`crate::model::RelationGroup`] (the ReqIF schema element is
//! `<RELATION-GROUP>` even though the plural container is
//! `<SPEC-RELATION-GROUPS>`).

use crate::model::{DataType, RelationGroup, SpecObject, SpecRelation, SpecType, Specification};

/// Per-container record of how an *empty* list appeared in the source.
///
/// Each flag is `true` iff the corresponding container existed in the source
/// and was empty AND spelled in the open/close form (`<X>\n      </X>\n`).
/// `false` (the default) means either:
///
/// - the container was absent (`None`) — flag is ignored,
/// - the container was empty self-closed (`<X/>`) — flag is ignored and the
///   unparser emits the self-closed form,
/// - or the container was non-empty — flag is ignored.
///
/// We only need a `bool` because the only ambiguity between two
/// byte-distinct sources is `<X/>` vs `<X></X>` for the empty case. Once a
/// container has children the open/close shape is forced.
///
/// Synthetic bundles (built via [`Default`] without going through the
/// parser) leave every flag `false`, so the unparser will fall back to the
/// self-closed empty form — matching the v1 behaviour from before this
/// field existed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ListForms {
    pub data_types_empty_open_close: bool,
    pub spec_types_empty_open_close: bool,
    pub spec_objects_empty_open_close: bool,
    pub spec_relations_empty_open_close: bool,
    pub specifications_empty_open_close: bool,
    pub relation_groups_empty_open_close: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ReqIfContent {
    pub data_types: Option<Vec<DataType>>,
    pub spec_types: Option<Vec<SpecType>>,
    pub spec_objects: Option<Vec<SpecObject>>,
    pub spec_relations: Option<Vec<SpecRelation>>,
    pub specifications: Option<Vec<Specification>>,
    pub relation_groups: Option<Vec<RelationGroup>>,
    /// Source-form bookkeeping for empty containers. See [`ListForms`].
    pub list_forms: ListForms,
}
