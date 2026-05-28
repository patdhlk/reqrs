pub mod attribute_def;
pub mod attribute_value;
pub mod data_type;
pub mod driver;
pub mod header;
pub mod relation_group;
pub mod spec_hierarchy;
pub mod spec_object;
pub mod spec_relation;
pub mod spec_type;
pub mod specification;
pub mod writer;

pub use writer::FormatMode;

use crate::error::ReqIfError;
use crate::model::ReqIfBundle;

/// Top-level entry point: emit a full `<REQ-IF>` document.
///
/// Mirrors [`crate::parse::ReqIfParser`] on the inverse path.
#[derive(Debug, Default, Clone, Copy)]
pub struct ReqIfUnparser;

impl ReqIfUnparser {
    /// Render `bundle` back to XML. The [`FormatMode`] argument is reserved
    /// for forthcoming canonicalization work; today both modes emit the same
    /// bytes because the per-element unparsers consult their own
    /// `was_self_closing` flags directly.
    pub fn unparse(bundle: &ReqIfBundle, mode: FormatMode) -> Result<String, ReqIfError> {
        driver::unparse_bundle(bundle, mode)
    }
}
