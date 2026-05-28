//! `<ATTRIBUTE-DEFINITION-*>` element model.
//!
//! Each variant corresponds to a typed attribute definition that lives inside
//! `<SPEC-ATTRIBUTES>` of a spec-type element (SPEC-OBJECT-TYPE,
//! SPECIFICATION-TYPE, SPEC-RELATION-TYPE). All variants share the same
//! `<TYPE>` wrapper child carrying a `<DATATYPE-DEFINITION-*-REF>` text node
//! pointing at the corresponding `DataType` identifier.
//!
//! The optional `<DEFAULT-VALUE>` child is held verbatim as raw XML via
//! [`DefaultValueRaw`]. This decouples the attribute-definition layer from the
//! `AttributeValue` parser (Task 9): the bytes are captured by parsing and
//! re-emitted untouched by unparsing, guaranteeing byte-exact round-trip even
//! when the inner shape is more elaborate than the current model knows about.

use chrono::{DateTime, FixedOffset};

use crate::error::ReqIfError;
use crate::helpers::datetime;
use crate::ids::{AttributeDefId, DataTypeId};

/// Sum-type over the seven typed `<ATTRIBUTE-DEFINITION-*>` elements.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttributeDefinition {
    String(AttributeDefinitionString),
    Boolean(AttributeDefinitionBoolean),
    Integer(AttributeDefinitionInteger),
    Real(AttributeDefinitionReal),
    Date(AttributeDefinitionDate),
    Xhtml(AttributeDefinitionXhtml),
    Enumeration(AttributeDefinitionEnumeration),
}

impl AttributeDefinition {
    /// The element's `IDENTIFIER` attribute.
    pub fn identifier(&self) -> &AttributeDefId {
        match self {
            AttributeDefinition::String(a) => &a.identifier,
            AttributeDefinition::Boolean(a) => &a.identifier,
            AttributeDefinition::Integer(a) => &a.identifier,
            AttributeDefinition::Real(a) => &a.identifier,
            AttributeDefinition::Date(a) => &a.identifier,
            AttributeDefinition::Xhtml(a) => &a.identifier,
            AttributeDefinition::Enumeration(a) => &a.identifier,
        }
    }

    /// Identifier of the `<DATATYPE-DEFINITION-*-REF>` text node carried inside `<TYPE>`.
    pub fn type_ref(&self) -> &DataTypeId {
        match self {
            AttributeDefinition::String(a) => &a.type_ref,
            AttributeDefinition::Boolean(a) => &a.type_ref,
            AttributeDefinition::Integer(a) => &a.type_ref,
            AttributeDefinition::Real(a) => &a.type_ref,
            AttributeDefinition::Date(a) => &a.type_ref,
            AttributeDefinition::Xhtml(a) => &a.type_ref,
            AttributeDefinition::Enumeration(a) => &a.type_ref,
        }
    }
}

/// Attributes shared by every `<ATTRIBUTE-DEFINITION-*>` element.
///
/// `is_editable` corresponds to the optional `IS-EDITABLE` XML attribute and
/// parses as `"true"`/`"false"`. `was_self_closing` mirrors the
/// `DataTypeCommon` precedent and is preserved across round-trip even though
/// the Python reference unparser never emits the self-closing form (we let the
/// caller decide).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttributeDefCommon {
    pub description: Option<String>,
    pub last_change: Option<String>,
    pub long_name: Option<String>,
    pub is_editable: Option<bool>,
    pub was_self_closing: bool,
}

impl AttributeDefCommon {
    /// Lazily parse `last_change` as a typed [`DateTime`].
    ///
    /// Returns `None` when the source had no `<LAST-CHANGE>` attribute. The
    /// raw string in [`Self::last_change`] is preserved unchanged so
    /// byte-fidelity round-trip is unaffected.
    pub fn last_change_parsed(&self) -> Option<Result<DateTime<FixedOffset>, ReqIfError>> {
        self.last_change.as_deref().map(datetime::parse)
    }
}

/// Verbatim raw inner XML of a `<DEFAULT-VALUE>` block, captured between
/// `<DEFAULT-VALUE>` and `</DEFAULT-VALUE>` exclusive of the tags themselves.
///
/// Holding the bytes as opaque text — including surrounding whitespace —
/// is what lets the unparser re-emit the block byte-exact, and is what
/// keeps Task 8 independent of Task 9's typed `AttributeValue` enum.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefaultValueRaw(pub String);

/// Order of `<TYPE>` and `<DEFAULT-VALUE>` children as they appeared in the
/// source XML. Real-world fixtures show both orderings (DEFAULT-VALUE before
/// TYPE in some, after in others) so round-trip requires preserving it.
///
/// Carried inside [`DefaultValuePresence`] so the order is only addressable
/// when a `<DEFAULT-VALUE>` child actually exists — `Absent` cannot pair with
/// an order, which removes the previously-meaningless `(Absent, _)` state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChildOrder {
    /// `<TYPE>` preceded `<DEFAULT-VALUE>` in the source.
    #[default]
    TypeFirst,
    /// `<DEFAULT-VALUE>` preceded `<TYPE>` in the source.
    DefaultFirst,
}

/// Whether `<DEFAULT-VALUE>` appeared in the source and, when it did, in which
/// position relative to `<TYPE>`. Self-closing vs open form is also tracked so
/// the unparser can re-emit the exact original shape.
///
/// The position is carried per-variant — `Absent` deliberately has no
/// `ChildOrder` field because position is meaningless when the child does not
/// exist. This makes the previously-possible `(Absent, _)` state
/// unrepresentable.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum DefaultValuePresence {
    /// No `<DEFAULT-VALUE>` child in the source.
    #[default]
    Absent,
    /// Source had `<DEFAULT-VALUE/>` at the given position relative to `<TYPE>` —
    /// emit self-closed in that position.
    SelfClosed(ChildOrder),
    /// Source had `<DEFAULT-VALUE>...</DEFAULT-VALUE>` at the given position —
    /// carry inner verbatim and emit open/close at that position.
    Open(DefaultValueRaw, ChildOrder),
}

macro_rules! ad_struct {
    ($name:ident { $($field:ident : $ty:ty),* $(,)? }) => {
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct $name {
            pub identifier: AttributeDefId,
            pub common: AttributeDefCommon,
            pub type_ref: DataTypeId,
            pub default_value: DefaultValuePresence,
            $(pub $field: $ty,)*
        }
    };
}

ad_struct!(AttributeDefinitionString {});
ad_struct!(AttributeDefinitionBoolean {});
ad_struct!(AttributeDefinitionInteger {});
ad_struct!(AttributeDefinitionReal {});
ad_struct!(AttributeDefinitionDate {});
ad_struct!(AttributeDefinitionXhtml {});
ad_struct!(AttributeDefinitionEnumeration {
    multi_valued: Option<bool>,
});

#[cfg(test)]
mod tests {
    use super::*;

    fn common() -> AttributeDefCommon {
        AttributeDefCommon {
            description: None,
            last_change: None,
            long_name: Some("T".into()),
            is_editable: None,
            was_self_closing: false,
        }
    }

    #[test]
    fn identifier_helper_returns_per_variant_id() {
        let s = AttributeDefinition::String(AttributeDefinitionString {
            identifier: AttributeDefId::new("AD-S"),
            common: common(),
            type_ref: DataTypeId::new("DT-1"),
            default_value: DefaultValuePresence::Absent,
        });
        assert_eq!(s.identifier().as_str(), "AD-S");
        assert_eq!(s.type_ref().as_str(), "DT-1");
    }

    #[test]
    fn enumeration_carries_multi_valued() {
        let e = AttributeDefinitionEnumeration {
            identifier: AttributeDefId::new("AD-E"),
            common: common(),
            type_ref: DataTypeId::new("DT-E"),
            default_value: DefaultValuePresence::Absent,
            multi_valued: Some(true),
        };
        assert_eq!(e.multi_valued, Some(true));
    }
}
