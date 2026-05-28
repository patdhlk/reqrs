#![forbid(unsafe_code)]

//! reqrs — ReqIF parsing and unparsing in Rust.

pub mod error;
pub mod ids;

pub use error::{ReqIfError, SchemaWarning};
pub use ids::{
    AttributeDefId, DataTypeId, EnumValueId, RelationGroupId, SpecObjectId, SpecRelationId,
    SpecTypeId, SpecificationId,
};
