//! Top-level container for one parsed ReqIF document.
//!
//! Mirrors `strict-doc-reqif/reqif/reqif_bundle.py`. A [`ReqIfBundle`]
//! aggregates everything captured from one `<REQ-IF>` document:
//!
//! - [`NamespaceInfo`] — the XML prologue + root element attributes.
//! - [`ReqIfHeader`] — the `<THE-HEADER>` block.
//! - [`CoreContent`] — the `<CORE-CONTENT>/<REQ-IF-CONTENT>` body.
//! - `tool_extensions` — a [`ToolExtensions`] value capturing whether the
//!   source had a `<TOOL-EXTENSIONS>` element, which form it took
//!   (self-closed vs empty open/close), and — when non-empty — the verbatim
//!   inner XML bytes. The verbatim string is preserved byte-for-byte (same
//!   pattern used for XHTML attribute values) so vendor-specific
//!   tool-extension payloads round-trip without loss.
//! - [`ObjectLookup`] — pre-built indexes for reference resolution.
//! - `exceptions` — non-fatal [`SchemaWarning`]s accumulated during parse.
//!
//! Note: [`ReqIfBundle`] does NOT derive [`PartialEq`] because
//! [`ObjectLookup`] contains [`std::sync::Arc`] values that compare by
//! pointer identity, not structural equality. Equality of two bundles
//! parsed from the same bytes is therefore not a useful comparison through
//! `==`; tests that need it should compare individual fields (the model
//! payload deriveds `PartialEq` field-by-field).

use crate::error::SchemaWarning;
use crate::ids::{SpecObjectId, SpecTypeId};
use crate::model::{
    CoreContent, NamespaceInfo, ObjectLookup, ReqIfHeader, SpecHierarchyIter, SpecObject,
    SpecObjectType, SpecType, Specification,
};

/// Source-form-preserving representation of the optional `<TOOL-EXTENSIONS>`
/// element under `<REQ-IF>`.
///
/// We do not yet model the rich vendor-specific schema that can appear inside
/// the element — but we DO preserve enough of the source to round-trip
/// byte-exact:
///
/// - [`ToolExtensions::Absent`] — the source had no `<TOOL-EXTENSIONS>` at all.
/// - [`ToolExtensions::SelfClosed`] — the source spelled it `<TOOL-EXTENSIONS/>`.
/// - [`ToolExtensions::EmptyOpenClose`] — the source spelled it
///   `<TOOL-EXTENSIONS>\n  </TOOL-EXTENSIONS>\n` (open/close pair with
///   whitespace-only body).
/// - `ToolExtensions::Content` — the source had non-whitespace content
///   between the open and close tags. `raw` holds the inner bytes verbatim
///   (leading/trailing whitespace included) so the unparser can splice them
///   back unchanged. This mirrors the `ReqIfReader::capture_inner_raw`
///   pattern used for XHTML attribute values.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum ToolExtensions {
    #[default]
    Absent,
    SelfClosed,
    EmptyOpenClose,
    Content(String),
}

impl ToolExtensions {
    /// True iff the source had a `<TOOL-EXTENSIONS>` element in any form.
    /// Convenience accessor that mirrors the old `tool_extensions_tag_exists`
    /// boolean for call sites that only care about presence.
    pub fn is_present(&self) -> bool {
        !matches!(self, ToolExtensions::Absent)
    }
}

#[derive(Debug, Clone)]
pub struct ReqIfBundle {
    pub namespace_info: NamespaceInfo,
    pub header: Option<ReqIfHeader>,
    pub core_content: Option<CoreContent>,
    pub tool_extensions: ToolExtensions,
    pub lookup: ObjectLookup,
    pub exceptions: Vec<SchemaWarning>,
}

impl ReqIfBundle {
    /// Construct a synthetic empty bundle. Used by callers that want to
    /// programmatically build a ReqIF document from scratch (no parse).
    ///
    /// `namespace` and `configuration` populate the corresponding attributes
    /// on [`NamespaceInfo`]; all other namespace fields default to `None`
    /// and `doctype_is_present` is `false`.
    pub fn empty(namespace: Option<String>, configuration: Option<String>) -> Self {
        Self {
            namespace_info: NamespaceInfo {
                namespace,
                configuration,
                ..Default::default()
            },
            header: None,
            core_content: None,
            tool_extensions: ToolExtensions::default(),
            lookup: ObjectLookup::empty(),
            exceptions: Vec::new(),
        }
    }

    /// Resolve a [`SpecObjectId`] to its [`SpecObject`] via the embedded
    /// [`ObjectLookup`]. Returns `None` when the id is unknown. Mirrors the
    /// Python `ReqIFBundle.get_spec_object_by_ref` convenience delegate.
    pub fn get_spec_object_by_ref(&self, ref_id: &SpecObjectId) -> Option<&SpecObject> {
        self.lookup.get_spec_object_by_ref(ref_id)
    }

    /// Resolve a [`SpecTypeId`] to a [`SpecObjectType`], returning `None`
    /// when the id is unknown OR when the id resolves to a different
    /// [`SpecType`] variant ([`SpecType::Specification`],
    /// [`SpecType::SpecRelation`], [`SpecType::RelationGroup`]).
    ///
    /// Mirrors the Python `ReqIFBundle.get_spec_object_type_by_ref`, which
    /// filters via `isinstance(spec_type, ReqIFSpecObjectType)`. Note the
    /// Python original returns `None` if `core_content.req_if_content.spec_types`
    /// is missing entirely; our [`ObjectLookup`] simply has no entry in that
    /// case, so the same `None` falls out of the lookup-miss branch.
    pub fn get_spec_object_type_by_ref(&self, ref_id: &SpecTypeId) -> Option<&SpecObjectType> {
        let st = self.lookup.get_spec_type_by_ref(ref_id)?;
        match st {
            SpecType::SpecObject(t) => Some(t),
            _ => None,
        }
    }

    /// Iterate every [`crate::model::SpecHierarchy`] node under
    /// `specification` in depth-first source order. See
    /// [`SpecHierarchyIter`] for the traversal contract.
    pub fn iterate_specification_hierarchy<'a>(
        &'a self,
        specification: &'a Specification,
    ) -> SpecHierarchyIter<'a> {
        SpecHierarchyIter::new(specification)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        ReqIfContent, SpecObjectChildTag, SpecObjectType, SpecRelationType, SpecTypeCommon,
    };

    fn make_spec_object(id: &str, type_id: &str) -> SpecObject {
        SpecObject {
            identifier: SpecObjectId::new(id),
            description: None,
            last_change: None,
            long_name: None,
            spec_object_type: SpecTypeId::new(type_id),
            attributes: vec![],
            children_order: vec![SpecObjectChildTag::Type, SpecObjectChildTag::Values],
            comments_before: vec![],
            values_trailing_comments: vec![],
        }
    }

    fn common(id: &str) -> SpecTypeCommon {
        SpecTypeCommon {
            identifier: SpecTypeId::new(id),
            description: None,
            last_change: None,
            long_name: None,
            was_self_closing: true,
            spec_attributes: None,
            comments_before: vec![],
        }
    }

    fn bundle_with_content(content: ReqIfContent) -> ReqIfBundle {
        let lookup = ObjectLookup::build(&content);
        ReqIfBundle {
            namespace_info: NamespaceInfo::default(),
            header: None,
            core_content: None,
            tool_extensions: ToolExtensions::default(),
            lookup,
            exceptions: Vec::new(),
        }
    }

    #[test]
    fn get_spec_object_by_ref_delegates_to_lookup() {
        let so = make_spec_object("SO-1", "ST-1");
        let bundle = bundle_with_content(ReqIfContent {
            spec_objects: Some(vec![so]),
            ..Default::default()
        });
        assert!(
            bundle
                .get_spec_object_by_ref(&SpecObjectId::new("SO-1"))
                .is_some()
        );
        assert!(
            bundle
                .get_spec_object_by_ref(&SpecObjectId::new("SO-MISSING"))
                .is_none()
        );
    }

    #[test]
    fn get_spec_object_type_by_ref_returns_some_for_spec_object_variant() {
        let sot = SpecType::SpecObject(SpecObjectType {
            common: common("ST-1"),
        });
        let bundle = bundle_with_content(ReqIfContent {
            spec_types: Some(vec![sot]),
            ..Default::default()
        });
        let got = bundle.get_spec_object_type_by_ref(&SpecTypeId::new("ST-1"));
        assert!(got.is_some());
        assert_eq!(got.unwrap().common.identifier.as_str(), "ST-1");
    }

    #[test]
    fn get_spec_object_type_by_ref_returns_none_for_other_variant() {
        let srt = SpecType::SpecRelation(SpecRelationType {
            common: common("ST-REL"),
        });
        let bundle = bundle_with_content(ReqIfContent {
            spec_types: Some(vec![srt]),
            ..Default::default()
        });
        assert!(
            bundle
                .get_spec_object_type_by_ref(&SpecTypeId::new("ST-REL"))
                .is_none()
        );
    }

    #[test]
    fn get_spec_object_type_by_ref_returns_none_for_unknown_id() {
        let bundle = ReqIfBundle::empty(None, None);
        assert!(
            bundle
                .get_spec_object_type_by_ref(&SpecTypeId::new("ST-NONE"))
                .is_none()
        );
    }
}
