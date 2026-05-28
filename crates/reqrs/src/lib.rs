#![forbid(unsafe_code)]

//! reqrs — ReqIF parsing and unparsing in Rust.

pub mod commands;
pub mod error;
pub mod ids;
pub mod model;
pub mod parse;
pub mod reqifz;
pub mod specification_iterator;
pub mod unparse;

pub use error::{ReqIfError, SchemaWarning};
pub use ids::{
    AttributeDefId, DataTypeId, EnumValueId, RelationGroupId, SpecObjectId, SpecRelationId,
    SpecTypeId, SpecificationId,
};
pub use parse::ReqIfParser;
pub use reqifz::ReqIfzBundle;
pub use specification_iterator::SpecificationIterator;
pub use unparse::{FormatMode, ReqIfUnparser};
