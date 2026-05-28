//! Standalone iterator over a [`Specification`]'s hierarchy tree.
//!
//! Mirrors the Python `reqif.specification_iterator.SpecificationIterator`.
//! Functionally equivalent to
//! [`crate::model::ReqIfBundle::iterate_specification_hierarchy`], useful
//! when a caller has a [`Specification`] in hand without the surrounding
//! [`crate::model::ReqIfBundle`].

use crate::model::{SpecHierarchyIter, Specification};

/// Zero-sized convenience type carrying the static [`Self::iterate`] entry
/// point. The Python original is a `class` with one `@staticmethod`; we
/// mirror that shape so the call-site reads identically.
pub struct SpecificationIterator;

impl SpecificationIterator {
    /// Yield every [`crate::model::SpecHierarchy`] under `specification` in
    /// depth-first source order. See
    /// [`crate::model::SpecHierarchyIter`] for the traversal contract.
    pub fn iterate(specification: &Specification) -> SpecHierarchyIter<'_> {
        SpecHierarchyIter::new(specification)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{SpecObjectId, SpecificationId};
    use crate::model::SpecHierarchy;

    fn leaf(id: &str) -> SpecHierarchy {
        SpecHierarchy {
            identifier: id.to_string(),
            last_change: None,
            long_name: None,
            editable: None,
            is_table_internal: None,
            spec_object_ref: SpecObjectId::new(format!("SO-{id}")),
            children: None,
            ref_then_children_order: true,
            level: 1,
            was_self_closing_children: false,
        }
    }

    fn parent(id: &str, children: Vec<SpecHierarchy>) -> SpecHierarchy {
        SpecHierarchy {
            children: Some(children),
            ..leaf(id)
        }
    }

    #[test]
    fn iterate_matches_depth_first_contract() {
        // Same tree shape as SpecHierarchyIter's test:
        // A -> [A1 -> [A1a, A1b], A2], B -> [B1].
        let a1a = leaf("A1a");
        let a1b = leaf("A1b");
        let a1 = parent("A1", vec![a1a, a1b]);
        let a2 = leaf("A2");
        let a = parent("A", vec![a1, a2]);
        let b1 = leaf("B1");
        let b = parent("B", vec![b1]);
        let spec = Specification {
            identifier: SpecificationId::new("SPEC-1"),
            description: None,
            last_change: None,
            long_name: None,
            specification_type: None,
            values: None,
            children: Some(vec![a, b]),
            children_order: Vec::new(),
            children_empty_open_close: false,
            values_empty_open_close: false,
        };
        let ids: Vec<&str> = SpecificationIterator::iterate(&spec)
            .map(|h| h.identifier.as_str())
            .collect();
        assert_eq!(ids, vec!["A", "A1", "A1a", "A1b", "A2", "B", "B1"]);
    }

    #[test]
    fn iterate_on_specification_with_no_children_is_empty() {
        let spec = Specification {
            identifier: SpecificationId::new("SPEC-EMPTY"),
            description: None,
            last_change: None,
            long_name: None,
            specification_type: None,
            values: None,
            children: None,
            children_order: Vec::new(),
            children_empty_open_close: false,
            values_empty_open_close: false,
        };
        assert!(SpecificationIterator::iterate(&spec).next().is_none());
    }
}
