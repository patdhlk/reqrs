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
