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
    /// Render `bundle` back to XML.
    ///
    /// Under [`FormatMode::Passthrough`] every captured byte (XHTML
    /// indentation, self-closing forms, attribute ordering captured by the
    /// parser) is replayed verbatim, giving byte-exact round-trip on the
    /// fixture corpus. Under [`FormatMode::Canonical`] XHTML bodies inside
    /// `<ATTRIBUTE-VALUE-XHTML>` are reflowed to the Python reference's
    /// 16-space margin via
    /// [`crate::helpers::xhtml_indent`]; other structural decisions still
    /// honor the per-element flags captured during parse.
    pub fn unparse(bundle: &ReqIfBundle, mode: FormatMode) -> Result<String, ReqIfError> {
        driver::unparse_bundle(bundle, mode)
    }
}
