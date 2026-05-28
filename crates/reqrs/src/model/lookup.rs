//! Pre-built indexes over a [`ReqIfContent`] for O(1) reference resolution.
//!
//! Mirrors `strict-doc-reqif/reqif/object_lookup.py`. ReqIF references between
//! elements are by string id: a `<SPEC-OBJECT>`'s
//! `<TYPE>/<SPEC-OBJECT-TYPE-REF>` names a `<SPEC-OBJECT-TYPE>` IDENTIFIER, a
//! `<SPEC-RELATION>` names a source and target `<SPEC-OBJECT>` IDENTIFIER, and
//! so on. Traversal code that needs to resolve a reference would otherwise
//! linear-scan the relevant list each time.
//!
//! [`ObjectLookup`] flattens the four lookup tables into a single struct and
//! holds each value behind [`std::sync::Arc`] so consumers can hand out cheap
//! shared references without cloning the underlying model nodes.
//!
//! Note: the lookup does NOT hold a reference back into the
//! [`crate::model::ReqIfContent`] it was built from. It clones each model
//! node into an [`Arc`]. The lookup is therefore independent of the lifetime
//! of the source content and survives content mutation, but it does NOT
//! reflect later edits to the source content — callers are expected to
//! rebuild the lookup after any mutation. This matches the Python
//! reference's behaviour.

use std::collections::HashMap;
use std::sync::Arc;

use crate::ids::{DataTypeId, SpecObjectId, SpecTypeId};
use crate::model::{DataType, ReqIfContent, SpecObject, SpecType};

#[derive(Debug, Default, Clone)]
pub struct ObjectLookup {
    pub data_types: HashMap<DataTypeId, Arc<DataType>>,
    pub spec_types: HashMap<SpecTypeId, Arc<SpecType>>,
    pub spec_objects: HashMap<SpecObjectId, Arc<SpecObject>>,
    /// For each [`SpecObject`] that appears as the `source` of one or more
    /// [`crate::model::SpecRelation`]s, the ordered list of corresponding
    /// `target` ids. Used by upcoming traversal helpers that walk relation
    /// chains forward from a source.
    pub spec_relations_parent: HashMap<SpecObjectId, Vec<SpecObjectId>>,
}

impl ObjectLookup {
    /// An empty lookup with no entries. Used by [`crate::model::ReqIfBundle::empty`].
    pub fn empty() -> Self {
        Self::default()
    }

    /// `true` iff the lookup currently holds a [`SpecObject`] under `ref_id`.
    ///
    /// Mirrors the Python `ReqIFObjectLookup.spec_object_exists`.
    pub fn spec_object_exists(&self, ref_id: &SpecObjectId) -> bool {
        self.spec_objects.contains_key(ref_id)
    }

    /// Resolve a [`DataTypeId`] to its [`DataType`], or `None` if no such
    /// data type was indexed.
    ///
    /// The Python reference panics (`KeyError`) on a missing key; we return
    /// `Option` so callers can choose their own failure mode (`.expect()` to
    /// mirror the panic, or `match` / `?` to handle gracefully).
    pub fn get_data_type_by_ref(&self, ref_id: &DataTypeId) -> Option<&DataType> {
        self.data_types.get(ref_id).map(|a| a.as_ref())
    }

    /// Resolve a [`SpecTypeId`] to its [`SpecType`], or `None` if no such
    /// spec type was indexed. See [`Self::get_data_type_by_ref`] for the
    /// `Option`-vs-panic rationale.
    pub fn get_spec_type_by_ref(&self, ref_id: &SpecTypeId) -> Option<&SpecType> {
        self.spec_types.get(ref_id).map(|a| a.as_ref())
    }

    /// Resolve a [`SpecObjectId`] to its [`SpecObject`], or `None` if no
    /// such spec object was indexed. See [`Self::get_data_type_by_ref`] for
    /// the `Option`-vs-panic rationale.
    pub fn get_spec_object_by_ref(&self, ref_id: &SpecObjectId) -> Option<&SpecObject> {
        self.spec_objects.get(ref_id).map(|a| a.as_ref())
    }

    /// For a [`SpecObject`] referenced as the source of one or more
    /// [`crate::model::SpecRelation`]s, return the ordered list of target
    /// ids. Returns an empty slice if `ref_id` is unknown OR if it is known
    /// but has never appeared as a relation source.
    ///
    /// Unlike the Python reference (which raises `KeyError` on unknown keys),
    /// we collapse "no parents" and "unknown id" into a single empty-slice
    /// answer — both are semantically "no parent edges to walk".
    pub fn get_spec_object_parents(&self, ref_id: &SpecObjectId) -> &[SpecObjectId] {
        self.spec_relations_parent
            .get(ref_id)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// Build a fresh lookup from a [`ReqIfContent`].
    ///
    /// Clones each model node into an [`Arc`]. The cost is O(N) clones plus
    /// O(N) hash inserts; for typical ReqIF corpora this is cheap.
    pub fn build(content: &ReqIfContent) -> Self {
        let mut out = Self::default();

        if let Some(dts) = &content.data_types {
            for dt in dts {
                out.data_types
                    .insert(dt.identifier().clone(), Arc::new(dt.clone()));
            }
        }

        if let Some(sts) = &content.spec_types {
            for st in sts {
                out.spec_types
                    .insert(st.identifier().clone(), Arc::new(st.clone()));
            }
        }

        if let Some(sos) = &content.spec_objects {
            for so in sos {
                out.spec_objects
                    .insert(so.identifier.clone(), Arc::new(so.clone()));
            }
        }

        // For each SpecRelation, push target into the source's parent list.
        if let Some(srs) = &content.spec_relations {
            for sr in srs {
                out.spec_relations_parent
                    .entry(sr.source.clone())
                    .or_default()
                    .push(sr.target.clone());
            }
        }

        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{DataTypeId, SpecObjectId, SpecRelationId, SpecTypeId};
    use crate::model::{
        DataType, DataTypeBoolean, DataTypeCommon, ReqIfContent, SpecObject, SpecObjectChildTag,
        SpecRelation,
    };

    fn make_spec_object(id: &str, type_id: &str) -> SpecObject {
        SpecObject {
            identifier: SpecObjectId::new(id),
            description: None,
            last_change: None,
            long_name: None,
            spec_object_type: SpecTypeId::new(type_id),
            attributes: vec![],
            children_order: vec![SpecObjectChildTag::Type, SpecObjectChildTag::Values],
        }
    }

    #[test]
    fn build_indexes_data_types_by_identifier() {
        let dt = DataType::Boolean(DataTypeBoolean {
            identifier: DataTypeId::new("DT-BOOL"),
            common: DataTypeCommon {
                description: None,
                last_change: None,
                long_name: None,
                was_self_closing: true,
            },
        });
        let content = ReqIfContent {
            data_types: Some(vec![dt]),
            ..Default::default()
        };
        let lookup = ObjectLookup::build(&content);
        assert!(lookup.data_types.contains_key(&DataTypeId::new("DT-BOOL")));
    }

    #[test]
    fn build_indexes_spec_objects_by_identifier() {
        let so = make_spec_object("SO-1", "SOT-1");
        let content = ReqIfContent {
            spec_objects: Some(vec![so]),
            ..Default::default()
        };
        let lookup = ObjectLookup::build(&content);
        assert!(lookup.spec_objects.contains_key(&SpecObjectId::new("SO-1")));
    }

    #[test]
    fn spec_object_exists_distinguishes_present_and_absent() {
        let so = make_spec_object("SO-1", "SOT-1");
        let content = ReqIfContent {
            spec_objects: Some(vec![so]),
            ..Default::default()
        };
        let lookup = ObjectLookup::build(&content);
        assert!(lookup.spec_object_exists(&SpecObjectId::new("SO-1")));
        assert!(!lookup.spec_object_exists(&SpecObjectId::new("SO-MISSING")));
    }

    #[test]
    fn get_data_type_by_ref_returns_some_for_present_none_for_absent() {
        let dt = DataType::Boolean(DataTypeBoolean {
            identifier: DataTypeId::new("DT-BOOL"),
            common: DataTypeCommon {
                description: None,
                last_change: None,
                long_name: None,
                was_self_closing: true,
            },
        });
        let content = ReqIfContent {
            data_types: Some(vec![dt]),
            ..Default::default()
        };
        let lookup = ObjectLookup::build(&content);

        let present = lookup.get_data_type_by_ref(&DataTypeId::new("DT-BOOL"));
        assert!(present.is_some());
        assert_eq!(present.unwrap().identifier().as_str(), "DT-BOOL");

        assert!(
            lookup
                .get_data_type_by_ref(&DataTypeId::new("DT-MISSING"))
                .is_none()
        );
    }

    #[test]
    fn get_spec_object_by_ref_returns_some_for_present_none_for_absent() {
        let so = make_spec_object("SO-1", "SOT-1");
        let content = ReqIfContent {
            spec_objects: Some(vec![so]),
            ..Default::default()
        };
        let lookup = ObjectLookup::build(&content);

        let present = lookup.get_spec_object_by_ref(&SpecObjectId::new("SO-1"));
        assert!(present.is_some());
        assert_eq!(present.unwrap().identifier.as_str(), "SO-1");

        assert!(
            lookup
                .get_spec_object_by_ref(&SpecObjectId::new("SO-MISSING"))
                .is_none()
        );
    }

    #[test]
    fn get_spec_object_parents_returns_empty_slice_for_unknown_id() {
        let lookup = ObjectLookup::empty();
        let parents = lookup.get_spec_object_parents(&SpecObjectId::new("SO-NONE"));
        assert!(parents.is_empty());
    }

    #[test]
    fn get_spec_object_parents_returns_recorded_targets_for_known_source() {
        let so_a = make_spec_object("SO-A", "T");
        let so_b = make_spec_object("SO-B", "T");
        let so_c = make_spec_object("SO-C", "T");
        let sr1 = SpecRelation {
            identifier: SpecRelationId::new("SR-1"),
            description: None,
            last_change: None,
            long_name: None,
            relation_type: SpecTypeId::new("SRT-1"),
            source: SpecObjectId::new("SO-A"),
            target: SpecObjectId::new("SO-B"),
            values: None,
            children_order: Vec::new(),
        };
        let sr2 = SpecRelation {
            identifier: SpecRelationId::new("SR-2"),
            description: None,
            last_change: None,
            long_name: None,
            relation_type: SpecTypeId::new("SRT-1"),
            source: SpecObjectId::new("SO-A"),
            target: SpecObjectId::new("SO-C"),
            values: None,
            children_order: Vec::new(),
        };
        let content = ReqIfContent {
            spec_objects: Some(vec![so_a, so_b, so_c]),
            spec_relations: Some(vec![sr1, sr2]),
            ..Default::default()
        };
        let lookup = ObjectLookup::build(&content);
        let parents = lookup.get_spec_object_parents(&SpecObjectId::new("SO-A"));
        assert_eq!(
            parents,
            &[SpecObjectId::new("SO-B"), SpecObjectId::new("SO-C")]
        );
    }

    #[test]
    fn build_records_spec_relation_parent_chain() {
        let so_a = make_spec_object("SO-A", "T");
        let so_b = make_spec_object("SO-B", "T");
        let sr = SpecRelation {
            identifier: SpecRelationId::new("SR-1"),
            description: None,
            last_change: None,
            long_name: None,
            relation_type: SpecTypeId::new("SRT-1"),
            source: SpecObjectId::new("SO-A"),
            target: SpecObjectId::new("SO-B"),
            values: None,
            children_order: Vec::new(),
        };
        let content = ReqIfContent {
            spec_objects: Some(vec![so_a, so_b]),
            spec_relations: Some(vec![sr]),
            ..Default::default()
        };
        let lookup = ObjectLookup::build(&content);
        let parents = lookup
            .spec_relations_parent
            .get(&SpecObjectId::new("SO-A"))
            .unwrap();
        assert_eq!(parents, &vec![SpecObjectId::new("SO-B")]);
    }
}
