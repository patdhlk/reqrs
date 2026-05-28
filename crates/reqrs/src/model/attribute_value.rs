//! `<ATTRIBUTE-VALUE-*>` element model.
//!
//! ReqIF spec objects carry their typed attribute values under a `<VALUES>`
//! block. Seven shapes are defined — five "scalar" shapes (STRING / INTEGER /
//! REAL / DATE / BOOLEAN) whose value lives in the `THE-VALUE` XML attribute
//! with `<DEFINITION>` as the only child, plus ENUMERATION (siblings
//! `<DEFINITION>` and `<VALUES>`) and XHTML (siblings `<DEFINITION>` and
//! `<THE-VALUE>` carrying inline markup).
//!
//! For the two non-scalar shapes, real-world ReqIF files put `<DEFINITION>`
//! either before or after the sibling block. `was_definition_first` records
//! that ordering so round-trip is byte-exact. Scalar variants do *not* carry
//! the flag — they have only one child (`<DEFINITION>`), so ordering is
//! degenerate.
//!
//! Scalar value bytes (`Integer.value`, `Real.value`, `Date.value`,
//! `String.value`) are stored as `String` rather than parsed numerics. This
//! preserves the exact source form ("1234.5" vs "1234.50" vs "1.2345e3" all
//! survive round-trip). Boolean is the only scalar that is parsed —
//! `"true"` / `"false"` carry no byte-level ambiguity.

use crate::ids::{AttributeDefId, EnumValueId};

/// Sum-type over the seven typed `<ATTRIBUTE-VALUE-*>` elements.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttributeValue {
    String(AttributeValueString),
    Boolean(AttributeValueBoolean),
    Integer(AttributeValueInteger),
    Real(AttributeValueReal),
    Date(AttributeValueDate),
    Xhtml(AttributeValueXhtml),
    Enumeration(AttributeValueEnumeration),
}

impl AttributeValue {
    /// `<ATTRIBUTE-DEFINITION-*-REF>` text under the inner `<DEFINITION>` child.
    pub fn definition_ref(&self) -> &AttributeDefId {
        match self {
            AttributeValue::String(a) => &a.definition_ref,
            AttributeValue::Boolean(a) => &a.definition_ref,
            AttributeValue::Integer(a) => &a.definition_ref,
            AttributeValue::Real(a) => &a.definition_ref,
            AttributeValue::Date(a) => &a.definition_ref,
            AttributeValue::Xhtml(a) => &a.definition_ref,
            AttributeValue::Enumeration(a) => &a.definition_ref,
        }
    }
}

/// `<ATTRIBUTE-VALUE-STRING THE-VALUE="…">` — the value bytes are raw.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttributeValueString {
    pub definition_ref: AttributeDefId,
    pub value: String,
}

/// `<ATTRIBUTE-VALUE-BOOLEAN THE-VALUE="true|false">` — parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttributeValueBoolean {
    pub definition_ref: AttributeDefId,
    pub value: bool,
}

/// `<ATTRIBUTE-VALUE-INTEGER THE-VALUE="…">` — value kept as text to preserve
/// arbitrary precision and the original byte form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttributeValueInteger {
    pub definition_ref: AttributeDefId,
    pub value: String,
}

/// `<ATTRIBUTE-VALUE-REAL THE-VALUE="…">` — value kept as text to preserve
/// trailing zeros and the original byte form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttributeValueReal {
    pub definition_ref: AttributeDefId,
    pub value: String,
}

/// `<ATTRIBUTE-VALUE-DATE THE-VALUE="…">` — value kept as text to defer any
/// datetime parsing (offset preservation, etc.) to consumers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttributeValueDate {
    pub definition_ref: AttributeDefId,
    pub value: String,
}

/// `<ATTRIBUTE-VALUE-XHTML>` carries inline XML inside `<THE-VALUE>`.
///
/// `the_value_raw` holds the verbatim bytes between `<THE-VALUE>` and
/// `</THE-VALUE>` — escaping, whitespace, and child markup are preserved
/// byte-exact via [`crate::parse::reader::ReqIfReader::capture_inner_raw`].
///
/// `was_definition_first` is `true` when the source had `<DEFINITION>` before
/// `<THE-VALUE>`, and `false` when the source had `<THE-VALUE>` before
/// `<DEFINITION>`. The Python reference templates default to
/// `<DEFINITION>` first (so the default value of this field is `true`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttributeValueXhtml {
    pub definition_ref: AttributeDefId,
    pub the_value_raw: String,
    pub was_definition_first: bool,
}

/// `<ATTRIBUTE-VALUE-ENUMERATION>` carries a list of `<ENUM-VALUE-REF>` inside
/// `<VALUES>` plus a single `<DEFINITION>` sibling.
///
/// `was_definition_first` follows the same convention as
/// [`AttributeValueXhtml::was_definition_first`]: `true` matches the Python
/// `ATTRIBUTE_ENUMERATION_TEMPLATE_REVERSE` (`<DEFINITION>` then `<VALUES>`),
/// `false` matches `ATTRIBUTE_ENUMERATION_TEMPLATE` (`<VALUES>` then
/// `<DEFINITION>`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttributeValueEnumeration {
    pub definition_ref: AttributeDefId,
    pub values: Vec<EnumValueId>,
    pub was_definition_first: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn definition_ref_helper_returns_each_variant_ref() {
        let s = AttributeValue::String(AttributeValueString {
            definition_ref: AttributeDefId::new("AD-S"),
            value: "x".into(),
        });
        assert_eq!(s.definition_ref().as_str(), "AD-S");

        let b = AttributeValue::Boolean(AttributeValueBoolean {
            definition_ref: AttributeDefId::new("AD-B"),
            value: true,
        });
        assert_eq!(b.definition_ref().as_str(), "AD-B");

        let e = AttributeValue::Enumeration(AttributeValueEnumeration {
            definition_ref: AttributeDefId::new("AD-E"),
            values: vec![EnumValueId::new("EV-1")],
            was_definition_first: true,
        });
        assert_eq!(e.definition_ref().as_str(), "AD-E");
    }
}
