use chrono::{DateTime, FixedOffset};

use crate::error::ReqIfError;
use crate::helpers::datetime;
use crate::ids::{DataTypeId, EnumValueId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataType {
    String(DataTypeString),
    Boolean(DataTypeBoolean),
    Integer(DataTypeInteger),
    Real(DataTypeReal),
    Date(DataTypeDate),
    Xhtml(DataTypeXhtml),
    Enumeration(DataTypeEnumeration),
}

impl DataType {
    pub fn identifier(&self) -> &DataTypeId {
        match self {
            DataType::String(d) => &d.identifier,
            DataType::Boolean(d) => &d.identifier,
            DataType::Integer(d) => &d.identifier,
            DataType::Real(d) => &d.identifier,
            DataType::Date(d) => &d.identifier,
            DataType::Xhtml(d) => &d.identifier,
            DataType::Enumeration(d) => &d.identifier,
        }
    }
}

/// Common attributes shared by all `<DATATYPE-DEFINITION-*>` tags.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataTypeCommon {
    pub description: Option<String>,
    pub last_change: Option<String>,
    pub long_name: Option<String>,
    pub was_self_closing: bool,
    /// Inline `<!-- ... -->` comments captured between the previous sibling
    /// (or `<DATATYPES>` open) and this element. Each string is the comment
    /// body (the text between `<!--` and `-->`, delimiters not included), in
    /// source order. Round-trip emits one comment per line above the element
    /// using the element's own indent. Defaults to `vec![]` when the source
    /// had no comments or when the value is constructed synthetically.
    pub comments_before: Vec<String>,
}

impl DataTypeCommon {
    /// Lazily parse `last_change` as a typed [`DateTime`].
    ///
    /// Returns `None` when the source had no `<LAST-CHANGE>` attribute. The
    /// raw string in [`Self::last_change`] is preserved unchanged so
    /// byte-fidelity round-trip is unaffected.
    pub fn last_change_parsed(&self) -> Option<Result<DateTime<FixedOffset>, ReqIfError>> {
        self.last_change.as_deref().map(datetime::parse)
    }
}

macro_rules! dt_struct {
    ($name:ident { $($field:ident : $ty:ty),* $(,)? }) => {
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct $name {
            pub identifier: DataTypeId,
            pub common: DataTypeCommon,
            $(pub $field: $ty,)*
        }
    };
}

dt_struct!(DataTypeString { max_length: Option<String> });
dt_struct!(DataTypeBoolean {});
dt_struct!(DataTypeInteger { max_value: Option<String>, min_value: Option<String> });
dt_struct!(DataTypeReal    { accuracy: Option<String>, max_value: Option<String>, min_value: Option<String> });
dt_struct!(DataTypeDate {});
dt_struct!(DataTypeXhtml {});

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataTypeEnumeration {
    pub identifier: DataTypeId,
    pub common: DataTypeCommon,
    pub specified_values: Option<Vec<EnumValue>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumValue {
    pub identifier: EnumValueId,
    pub long_name: Option<String>,
    pub description: Option<String>,
    pub last_change: Option<String>,
    pub key: String,
    pub other_content: Option<String>,
}
