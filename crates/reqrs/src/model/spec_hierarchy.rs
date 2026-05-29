//! `<SPEC-HIERARCHY>` element model.
//!
//! Mirrors `strict-doc-reqif/reqif/models/reqif_spec_hierarchy.py`. A
//! spec-hierarchy node references one [`crate::model::SpecObject`] via
//! `<OBJECT>/<SPEC-OBJECT-REF>` and optionally nests further `<SPEC-HIERARCHY>`
//! children inside `<CHILDREN>`. Vendors differ on the order of those two
//! sibling elements (some emit OBJECT first, others CHILDREN first); the
//! parser records which arrangement the source used so the unparser can
//! preserve it.
//!
//! `level` is not part of the ReqIF schema — it is a derived depth counter
//! the Python reference uses to compute indentation via
//! `calculate_base_level() = 12 + (level - 1) * 4`. We carry the same
//! invariant.

use std::collections::VecDeque;

use chrono::{DateTime, FixedOffset};

use crate::error::ReqIfError;
use crate::helpers::datetime;
use crate::ids::SpecObjectId;
use crate::model::Specification;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecHierarchy {
    /// `IDENTIFIER` attribute. Held as a bare `String` to mirror the Python
    /// model — there is no `SpecHierarchyId` newtype in the catalog.
    pub identifier: String,
    pub last_change: Option<String>,
    pub long_name: Option<String>,
    pub editable: Option<bool>,
    pub is_table_internal: Option<bool>,
    /// Target of `<OBJECT>/<SPEC-OBJECT-REF>`.
    pub spec_object_ref: SpecObjectId,
    /// Optional `<CHILDREN>` block. `None` means the source had no `<CHILDREN>`
    /// at all; `Some(vec![])` means the source had an empty `<CHILDREN/>` or
    /// `<CHILDREN></CHILDREN>` (see `was_self_closing_children`).
    pub children: Option<Vec<SpecHierarchy>>,
    /// `true` when the source had `<OBJECT>` before `<CHILDREN>`. `false` when
    /// `<CHILDREN>` came first.
    pub ref_then_children_order: bool,
    /// Depth (1-based) — controls indentation per
    /// `calculate_base_level() = 12 + (level - 1) * 4`.
    pub level: usize,
    /// `true` when the source had `<CHILDREN/>` self-closed. Only meaningful
    /// alongside `children == Some(vec![])`.
    pub was_self_closing_children: bool,
}

impl SpecHierarchy {
    /// Lazily parse `last_change` as a typed [`DateTime`].
    ///
    /// Returns `None` when the source had no `<LAST-CHANGE>` attribute. The
    /// raw string in [`Self::last_change`] is preserved unchanged so
    /// byte-fidelity round-trip is unaffected.
    pub fn last_change_parsed(&self) -> Option<Result<DateTime<FixedOffset>, ReqIfError>> {
        self.last_change.as_deref().map(datetime::parse)
    }
}

/// Depth-first iterator over a [`Specification`]'s hierarchy tree.
///
/// Mirrors the Python `ReqIFBundle.iterate_specification_hierarchy` /
/// `SpecificationIterator.iterate_specification` traversal: visit each
/// [`SpecHierarchy`] in source order, descending into a node's children
/// immediately before continuing to its siblings.
///
/// Implementation: a deque used as a stack. We `pop_front` to get the next
/// node, then `push_front` that node's children in reverse so they come out
/// in source order on subsequent pops. This matches Python's
/// `task_list.extendleft(reversed(current.children))` idiom.
pub struct SpecHierarchyIter<'a> {
    queue: VecDeque<&'a SpecHierarchy>,
}

impl<'a> SpecHierarchyIter<'a> {
    /// Seed the iterator from a [`Specification`]'s top-level children.
    /// A specification with `children == None` produces an empty iterator.
    pub(crate) fn new(specification: &'a Specification) -> Self {
        let mut queue = VecDeque::new();
        if let Some(children) = &specification.children {
            for c in children {
                queue.push_back(c);
            }
        }
        Self { queue }
    }
}

impl<'a> Iterator for SpecHierarchyIter<'a> {
    type Item = &'a SpecHierarchy;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.queue.pop_front()?;
        // Push children to the front in reverse so they come out depth-first
        // in source order. Mirrors Python's
        // `task_list.extendleft(reversed(current.children))`.
        if let Some(children) = &current.children {
            for child in children.iter().rev() {
                self.queue.push_front(child);
            }
        }
        Some(current)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{SpecObjectId, SpecificationId};

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

    /// Build a 3-level tree:
    ///
    /// ```text
    /// A
    ///   A1
    ///     A1a
    ///     A1b
    ///   A2
    /// B
    ///   B1
    /// ```
    fn make_three_level_spec() -> Specification {
        let a1a = leaf("A1a");
        let a1b = leaf("A1b");
        let a1 = parent("A1", vec![a1a, a1b]);
        let a2 = leaf("A2");
        let a = parent("A", vec![a1, a2]);
        let b1 = leaf("B1");
        let b = parent("B", vec![b1]);

        Specification {
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
            comments_before: Vec::new(),
            values_trailing_comments: Vec::new(),
        }
    }

    #[test]
    fn iterator_yields_depth_first_in_source_order() {
        let spec = make_three_level_spec();
        let ids: Vec<&str> = SpecHierarchyIter::new(&spec)
            .map(|h| h.identifier.as_str())
            .collect();
        assert_eq!(ids, vec!["A", "A1", "A1a", "A1b", "A2", "B", "B1"]);
    }

    #[test]
    fn iterator_is_empty_when_specification_has_no_children() {
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
            comments_before: Vec::new(),
            values_trailing_comments: Vec::new(),
        };
        assert!(SpecHierarchyIter::new(&spec).next().is_none());
    }

    #[test]
    fn iterator_treats_empty_children_vec_as_no_descendants() {
        let mut leaf_with_empty = leaf("X");
        leaf_with_empty.children = Some(Vec::new());
        let spec = Specification {
            identifier: SpecificationId::new("SPEC-X"),
            description: None,
            last_change: None,
            long_name: None,
            specification_type: None,
            values: None,
            children: Some(vec![leaf_with_empty]),
            children_order: Vec::new(),
            children_empty_open_close: false,
            values_empty_open_close: false,
            comments_before: Vec::new(),
            values_trailing_comments: Vec::new(),
        };
        let ids: Vec<&str> = SpecHierarchyIter::new(&spec)
            .map(|h| h.identifier.as_str())
            .collect();
        assert_eq!(ids, vec!["X"]);
    }
}
