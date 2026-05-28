//! `<SPECIFICATION>` element model.
//!
//! Mirrors `strict-doc-reqif/reqif/models/reqif_specification.py`. A
//! specification is structurally similar to a [`crate::model::SpecObject`] but
//! carries a `<CHILDREN>` block of [`crate::model::SpecHierarchy`] nodes plus
//! optional `<TYPE>/<SPECIFICATION-TYPE-REF>` and `<VALUES>` siblings.
//!
//! The three children (`TYPE`, `CHILDREN`, `VALUES`) appear in different
//! permutations across vendors. The Python reference recovers the original
//! order via `xml_node`; we record an explicit [`SpecificationChildTag`]
//! sequence as the parser walks the body.

use crate::ids::{SpecTypeId, SpecificationId};
use crate::model::{AttributeValue, SpecHierarchy};

/// Marker recording which of the three children was seen during parse, in
/// source order. The unparser iterates this list and re-emits the children in
/// the recorded order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpecificationChildTag {
    Type,
    Children,
    Values,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Specification {
    pub identifier: SpecificationId,
    pub description: Option<String>,
    pub last_change: Option<String>,
    pub long_name: Option<String>,
    pub specification_type: Option<SpecTypeId>,
    pub values: Option<Vec<AttributeValue>>,
    pub children: Option<Vec<SpecHierarchy>>,
    /// Source order of `<TYPE>`, `<CHILDREN>`, `<VALUES>` children — recorded
    /// only for children actually present in the source.
    pub children_order: Vec<SpecificationChildTag>,
}
