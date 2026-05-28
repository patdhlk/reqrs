//! `<SPEC-OBJECT-TYPE>` / `<SPECIFICATION-TYPE>` / `<SPEC-RELATION-TYPE>` /
//! `<RELATION-GROUP-TYPE>` element model.
//!
//! The four `<SPEC-TYPES>` children share an identical structural shape:
//! they carry the common `IDENTIFIER` + optional `DESC` / `LAST-CHANGE` /
//! `LONG-NAME` attributes, plus an optional `<SPEC-ATTRIBUTES>` child whose
//! body is a list of [`AttributeDefinition`] from Task 8. Only the outer tag
//! name distinguishes them, so the variants compose a shared
//! [`SpecTypeCommon`] payload.
//!
//! `spec_attributes` is `Option<Vec<AttributeDefinition>>` to mirror the
//! three-state semantics that `DataTypeEnumeration::specified_values` uses
//! for `<SPECIFIED-VALUES>`:
//!
//! - `None` — the source had no `<SPEC-ATTRIBUTES>` element.
//! - `Some(vec![])` — the source had an empty `<SPEC-ATTRIBUTES/>` (or
//!   `<SPEC-ATTRIBUTES></SPEC-ATTRIBUTES>` collapsed to self-closed on emit).
//! - `Some(vec![ad, ...])` — the source had one or more
//!   `<ATTRIBUTE-DEFINITION-*>` children.

use chrono::{DateTime, FixedOffset};

use crate::error::ReqIfError;
use crate::helpers::datetime;
use crate::ids::SpecTypeId;
use crate::model::AttributeDefinition;

/// Sum-type over the four `<SPEC-TYPES>` children.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpecType {
    SpecObject(SpecObjectType),
    Specification(SpecificationType),
    SpecRelation(SpecRelationType),
    RelationGroup(RelationGroupType),
}

impl SpecType {
    /// The element's `IDENTIFIER` attribute.
    pub fn identifier(&self) -> &SpecTypeId {
        match self {
            SpecType::SpecObject(t) => &t.common.identifier,
            SpecType::Specification(t) => &t.common.identifier,
            SpecType::SpecRelation(t) => &t.common.identifier,
            SpecType::RelationGroup(t) => &t.common.identifier,
        }
    }
}

/// Attributes and children shared by every `<SPEC-TYPES>` element variant.
///
/// `was_self_closing` mirrors the [`crate::model::DataTypeCommon`] precedent
/// and is preserved across round-trip — a `<SPEC-OBJECT-TYPE IDENTIFIER="X"/>`
/// re-emits self-closed if `spec_attributes` is also `None`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecTypeCommon {
    pub identifier: SpecTypeId,
    pub description: Option<String>,
    pub last_change: Option<String>,
    pub long_name: Option<String>,
    pub was_self_closing: bool,
    pub spec_attributes: Option<Vec<AttributeDefinition>>,
    /// Inline `<!-- ... -->` comments captured between the previous sibling
    /// (or `<SPEC-TYPES>` open) and this element. Each string is the comment
    /// body (the text between `<!--` and `-->`, delimiters not included), in
    /// source order. Round-trip emits one comment per line above the element
    /// using the element's own indent. Defaults to `vec![]` when the source
    /// had no comments or when the value is constructed synthetically.
    pub comments_before: Vec<String>,
}

impl SpecTypeCommon {
    /// Lazily parse `last_change` as a typed [`DateTime`].
    ///
    /// Returns `None` when the source had no `<LAST-CHANGE>` attribute. The
    /// raw string in [`Self::last_change`] is preserved unchanged so
    /// byte-fidelity round-trip is unaffected.
    pub fn last_change_parsed(&self) -> Option<Result<DateTime<FixedOffset>, ReqIfError>> {
        self.last_change.as_deref().map(datetime::parse)
    }
}

macro_rules! spec_type_struct {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct $name {
            pub common: SpecTypeCommon,
        }
    };
}

spec_type_struct!(SpecObjectType);
spec_type_struct!(SpecificationType);
spec_type_struct!(SpecRelationType);
spec_type_struct!(RelationGroupType);

#[cfg(test)]
mod tests {
    use super::*;

    fn common() -> SpecTypeCommon {
        SpecTypeCommon {
            identifier: SpecTypeId::new("ST-1"),
            description: None,
            last_change: None,
            long_name: Some("T".into()),
            was_self_closing: false,
            spec_attributes: None,
            comments_before: vec![],
        }
    }

    #[test]
    fn identifier_helper_returns_per_variant_id() {
        let s = SpecType::SpecObject(SpecObjectType { common: common() });
        assert_eq!(s.identifier().as_str(), "ST-1");

        let s = SpecType::Specification(SpecificationType { common: common() });
        assert_eq!(s.identifier().as_str(), "ST-1");

        let s = SpecType::SpecRelation(SpecRelationType { common: common() });
        assert_eq!(s.identifier().as_str(), "ST-1");

        let s = SpecType::RelationGroup(RelationGroupType { common: common() });
        assert_eq!(s.identifier().as_str(), "ST-1");
    }

    #[test]
    fn spec_attributes_tri_state_is_addressable() {
        // None: no <SPEC-ATTRIBUTES> in source.
        let none = SpecTypeCommon {
            spec_attributes: None,
            ..common()
        };
        assert!(none.spec_attributes.is_none());

        // Some(vec![]): self-closed <SPEC-ATTRIBUTES/>.
        let empty = SpecTypeCommon {
            spec_attributes: Some(Vec::new()),
            ..common()
        };
        assert_eq!(empty.spec_attributes.as_ref().map(Vec::len), Some(0));
    }
}
