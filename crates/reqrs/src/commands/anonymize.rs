//! `anonymize` command — strip user-visible strings while preserving structure.
//!
//! Mirrors `strict-doc-reqif/reqif/commands/anonymize/anonymize.py`. Every
//! user-visible string in the bundle is replaced with `Anonymized-<digits>`,
//! where `<digits>` is a [`rustc_hash::FxHasher`] of the seed concatenated
//! with the original string. Two consequences:
//!
//! - Same input + same `seed` → byte-identical output (deterministic within
//!   a run and across reruns).
//! - Same string appearing in multiple fields → mapped to the same opaque
//!   token (preserves referential consistency across e.g. spec object
//!   description fields that quote each other).
//!
//! Anonymized fields:
//! - Header: `title`, `comment`, `source_tool_id`, `req_if_tool_id`, and
//!   the `Text` variant of `repository_id`.
//! - [`DataTypeCommon`]: `description`, `long_name`.
//! - [`SpecObject`]: `description`, `long_name`, plus `String` and `Xhtml`
//!   attribute values.
//! - [`Specification`]: `description`, `long_name`, plus `String` and `Xhtml`
//!   attribute values inside `values`.
//!
//! For `Xhtml` attribute values, the original raw markup is replaced with
//! `<xhtml:div>Anonymized-<digits></xhtml:div>` so the output remains
//! well-formed XHTML. This assumes the source bundle declares the
//! `xmlns:xhtml` namespace — which any document carrying XHTML values
//! necessarily does.
//!
//! Identifiers, dates, numeric values, enum keys, and boolean flags are
//! left intact so the document remains structurally valid and references
//! still resolve.

use std::collections::HashMap;
use std::fs;
use std::hash::{BuildHasher, Hasher};
use std::path::PathBuf;

use rustc_hash::FxBuildHasher;

use crate::error::ReqIfError;
use crate::model::{AttributeValue, DataType, RepositoryId, ReqIfBundle, SpecObject};
use crate::parse::ReqIfParser;
use crate::unparse::{FormatMode, ReqIfUnparser};

#[derive(Debug, Clone)]
pub struct AnonymizeOpts {
    pub input: PathBuf,
    pub output: PathBuf,
    pub seed: u64,
}

/// Parse `opts.input`, rewrite user-visible strings deterministically per
/// `opts.seed`, then write the anonymized document to `opts.output`.
pub fn anonymize(opts: AnonymizeOpts) -> Result<(), ReqIfError> {
    let mut bundle = ReqIfParser::parse_path(&opts.input)?;
    let mut state = AnonState::new(opts.seed);
    anon_bundle(&mut bundle, &mut state);
    let out = ReqIfUnparser::unparse(&bundle, FormatMode::Passthrough)?;
    fs::write(&opts.output, out)?;
    Ok(())
}

/// Per-run anonymization state. The `cache` keeps mappings stable so the
/// same source string always maps to the same opaque token within one run.
struct AnonState {
    seed: u64,
    cache: HashMap<String, String>,
}

impl AnonState {
    fn new(seed: u64) -> Self {
        Self {
            seed,
            cache: HashMap::new(),
        }
    }

    fn map(&mut self, s: &str) -> String {
        if let Some(v) = self.cache.get(s) {
            return v.clone();
        }
        let mut h = FxBuildHasher.build_hasher();
        h.write_u64(self.seed);
        h.write(s.as_bytes());
        let digit = h.finish();
        let out = format!("Anonymized-{digit}");
        self.cache.insert(s.to_owned(), out.clone());
        out
    }
}

fn anon_opt(s: &mut Option<String>, state: &mut AnonState) {
    if let Some(v) = s {
        *v = state.map(v);
    }
}

fn anon_bundle(bundle: &mut ReqIfBundle, state: &mut AnonState) {
    if let Some(h) = &mut bundle.header {
        anon_opt(&mut h.title, state);
        anon_opt(&mut h.comment, state);
        anon_opt(&mut h.source_tool_id, state);
        anon_opt(&mut h.req_if_tool_id, state);
        if let Some(RepositoryId::Text(t)) = &mut h.repository_id {
            *t = state.map(t);
        }
    }

    let Some(cc) = &mut bundle.core_content else {
        return;
    };
    let Some(content) = &mut cc.req_if_content else {
        return;
    };

    if let Some(dts) = &mut content.data_types {
        for dt in dts {
            let common = match dt {
                DataType::String(d) => &mut d.common,
                DataType::Boolean(d) => &mut d.common,
                DataType::Integer(d) => &mut d.common,
                DataType::Real(d) => &mut d.common,
                DataType::Date(d) => &mut d.common,
                DataType::Xhtml(d) => &mut d.common,
                DataType::Enumeration(d) => &mut d.common,
            };
            anon_opt(&mut common.description, state);
            anon_opt(&mut common.long_name, state);
        }
    }

    if let Some(objs) = &mut content.spec_objects {
        for o in objs {
            anon_spec_object(o, state);
        }
    }

    if let Some(specs) = &mut content.specifications {
        for s in specs {
            anon_opt(&mut s.long_name, state);
            anon_opt(&mut s.description, state);
            if let Some(values) = &mut s.values {
                for v in values {
                    anon_attribute_value(v, state);
                }
            }
        }
    }
}

fn anon_spec_object(o: &mut SpecObject, state: &mut AnonState) {
    anon_opt(&mut o.description, state);
    anon_opt(&mut o.long_name, state);
    for v in &mut o.attributes {
        anon_attribute_value(v, state);
    }
}

/// Anonymize a single `<ATTRIBUTE-VALUE-*>` in place. Shared by [`SpecObject`]
/// attributes and `<SPECIFICATION>` `<VALUES>` blocks.
///
/// - `String` → opaque `Anonymized-<digits>` token.
/// - `Xhtml` → token wrapped in `<xhtml:div>…</xhtml:div>` so the output is
///   still well-formed XHTML (Python parity).
/// - INTEGER / REAL / DATE / BOOLEAN / ENUMERATION are not free-text and are
///   left intact.
fn anon_attribute_value(v: &mut AttributeValue, state: &mut AnonState) {
    match v {
        AttributeValue::String(s) => s.value = state.map(&s.value),
        AttributeValue::Xhtml(x) => {
            let anonymized = state.map(&x.the_value_raw);
            x.the_value_raw = format!("<xhtml:div>{anonymized}</xhtml:div>");
        }
        _ => {}
    }
}
