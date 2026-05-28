pub mod attribute_def;
pub mod attribute_value;
pub mod data_type;
pub(crate) mod driver;
pub mod header;
pub(crate) mod reader;
pub mod relation_group;
pub mod spec_hierarchy;
pub mod spec_object;
pub mod spec_relation;
pub mod spec_type;
pub mod specification;

use crate::error::ReqIfError;
use crate::model::ReqIfBundle;

/// Top-level entry point: parse a full `<REQ-IF>` document.
///
/// All three constructors funnel through `driver::parse_bundle`. Cloning is
/// free — this is a unit struct.
#[derive(Debug, Default, Clone, Copy)]
pub struct ReqIfParser;

impl ReqIfParser {
    /// Parse from an in-memory `&str`. Equivalent to [`Self::parse_bytes`]
    /// over the str's bytes.
    pub fn parse_str(s: &str) -> Result<ReqIfBundle, ReqIfError> {
        Self::parse_bytes(s.as_bytes())
    }

    /// Parse from a raw byte slice. The XML prologue + encoding are sniffed
    /// from the first 200 bytes; see `driver::parse_bundle` for details.
    pub fn parse_bytes(b: &[u8]) -> Result<ReqIfBundle, ReqIfError> {
        driver::parse_bundle(b)
    }

    /// Read a file from disk and parse it. Errors from `std::fs::read` are
    /// surfaced as [`ReqIfError::Io`].
    pub fn parse_path(p: impl AsRef<std::path::Path>) -> Result<ReqIfBundle, ReqIfError> {
        let bytes = std::fs::read(p)?;
        Self::parse_bytes(&bytes)
    }
}
