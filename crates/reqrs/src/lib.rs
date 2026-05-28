#![forbid(unsafe_code)]

//! reqrs — ReqIF parsing and unparsing in Rust.

pub mod error;
pub mod ids;
mod parse;
pub mod unparse;

pub use error::{ReqIfError, SchemaWarning};
pub use ids::{
    AttributeDefId, DataTypeId, EnumValueId, RelationGroupId, SpecObjectId, SpecRelationId,
    SpecTypeId, SpecificationId,
};
pub use unparse::FormatMode;
