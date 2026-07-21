//! `validate` command — internal semantic checks over a parsed bundle.
//!
//! Mirrors the always-on portion of
//! `strict-doc-reqif/reqif/commands/validate/validate.py`. The Python
//! implementation supports two layers:
//!
//! 1. Strict XSD conformance via the `xmlschema` library, gated on
//!    `--use-reqif-schema`. Implemented here by shelling out to
//!    `xmllint` against the bundled OMG ReqIF XSD tree (extracted to a
//!    tempdir on demand). The `include_dir` crate embeds the schemas at
//!    compile time; missing `xmllint` on `$PATH` surfaces as a clear
//!    [`ReqIfError::Schema`] with an install hint.
//! 2. Internal semantic checks that always run after a successful parse —
//!    these are what this module implements.
//!
//! The checks are intentionally narrow: anything the parser would reject
//! (missing required attributes, malformed XML) never reaches us.
//! [`validate`] only flags issues that the parser tolerates but that break
//! reference integrity, plus a handful of XML-prologue advisories that
//! mirror Python's `ReqIFErrorBundle` output.
//!
//! Findings are partitioned into three tiers via [`ValidateReport`]:
//!
//! - [`ValidateReport::xml_errors`] — fatal parse-level problems. Today
//!   this list is always empty (a malformed document fails [`crate::parse::ReqIfParser`]
//!   before we ever reach the checks), but the field exists for parity
//!   with Python's `ReqIFErrorBundle.xml_errors` and gives the CLI a
//!   stable place to surface future parse-recovery diagnostics.
//! - [`ValidateReport::schema_errors`] — XSD violations reported by
//!   `xmllint` plus any [`crate::error::Issue`]s the parser accumulated on
//!   [`ReqIfBundle::exceptions`].
//! - [`ValidateReport::semantic_warnings`] — everything else: duplicate
//!   IDENTIFIERs, dangling refs, missing XML declaration, non-UTF-8
//!   encoding.
//!
//! Checks that land here:
//!
//! - **Duplicate `IDENTIFIER`s** across the six top-level lists
//!   (`<DATATYPES>`, `<SPEC-TYPES>`, `<SPEC-OBJECTS>`, `<SPECIFICATIONS>`,
//!   `<SPEC-RELATIONS>`, `<SPEC-RELATION-GROUPS>`). ReqIF identifiers must
//!   be globally unique within a document.
//! - **Dangling `<SPEC-OBJECT-TYPE-REF>`** — every `<SPEC-OBJECT>` points
//!   at a [`crate::model::SpecType`] via its `<TYPE>` child; the target
//!   must exist.
//! - **Dangling `<SPECIFICATION-TYPE-REF>`** — same constraint for
//!   `<SPECIFICATION>` elements that carry a `<TYPE>` child (optional).
//! - **Dangling `<SPEC-RELATION>` source/target** — every relation's
//!   `<SOURCE>` and `<TARGET>` must resolve to a known
//!   [`crate::model::SpecObject`].
//! - **Dangling `<SPEC-HIERARCHY>` object ref** — every hierarchy node's
//!   `<OBJECT>/<SPEC-OBJECT-REF>` must resolve to a known
//!   [`crate::model::SpecObject`]. Iterated via
//!   [`ReqIfBundle::iterate_specification_hierarchy`].
//! - **Missing XML declaration** — Python's reference flags documents that
//!   lack the `<?xml ... ?>` prologue.
//! - **Non-UTF-8 encoding** — Python's reference recommends UTF-8 per the
//!   ReqIF Implementation Guide.

use std::collections::HashSet;
use std::path::PathBuf;
use std::process::Command;

use include_dir::{Dir, include_dir};

use crate::error::ReqIfError;
use crate::model::ReqIfBundle;
use crate::parse::ReqIfParser;

/// Embedded OMG ReqIF XSD tree (`reqif.xsd` at the root plus all
/// imported XHTML modularization schemas). Extracted on demand to a
/// tempdir whenever [`validate`] is called with
/// `use_reqif_schema = true`.
static SCHEMA_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/src/commands/schema");

#[derive(Debug, Clone)]
pub struct ValidateOpts {
    pub input: PathBuf,
    pub use_reqif_schema: bool,
}

/// Three-tier validation report mirroring Python's `ReqIFErrorBundle`.
///
/// See the module-level docs for the semantics of each tier. A clean
/// document yields a report where every list is empty —
/// [`ValidateReport::has_any_errors`] is the canonical "did anything
/// fail?" predicate for the CLI exit code decision.
#[derive(Debug, Default, Clone)]
pub struct ValidateReport {
    /// Parse-level errors (malformed XML). If non-empty, the document is
    /// unparseable. Today always empty (the parser fails fast); reserved
    /// for future parse-recovery diagnostics for parity with Python.
    pub xml_errors: Vec<String>,
    /// Schema-level errors — XSD violations from `xmllint` plus the
    /// [`crate::error::Issue`]s surfaced via
    /// [`ReqIfBundle::exceptions`] during parse.
    pub schema_errors: Vec<String>,
    /// Semantic warnings — dangling refs, duplicate IDENTIFIERs, missing
    /// XML declaration, non-UTF-8 encoding.
    pub semantic_warnings: Vec<String>,
}

impl ValidateReport {
    /// `true` iff any tier has at least one entry. The CLI maps this to a
    /// non-zero exit code.
    pub fn has_any_errors(&self) -> bool {
        !self.xml_errors.is_empty()
            || !self.schema_errors.is_empty()
            || !self.semantic_warnings.is_empty()
    }
}

/// Parse `opts.input` and run the always-on semantic checks. If
/// `opts.use_reqif_schema` is set, also run the XSD conformance check.
/// Returns a [`ValidateReport`] whose tiers are populated according to
/// the kinds of issues found; an all-empty report means a clean document.
pub fn validate(opts: ValidateOpts) -> Result<ValidateReport, ReqIfError> {
    let bundle = ReqIfParser::parse_path(&opts.input)?;
    let mut report = ValidateReport::default();

    check_duplicate_identifiers(&bundle, &mut report);
    check_dangling_refs(&bundle, &mut report);
    check_dangling_hierarchy_refs(&bundle, &mut report);
    check_xml_prologue(&bundle, &mut report);
    surface_parse_exceptions(&bundle, &mut report);

    if opts.use_reqif_schema {
        check_xsd(&opts, &mut report)?;
    }

    Ok(report)
}

fn check_duplicate_identifiers(bundle: &ReqIfBundle, report: &mut ValidateReport) {
    let mut seen: HashSet<String> = HashSet::new();
    let mut check = |id: &str, kind: &str| {
        if !seen.insert(id.to_owned()) {
            report
                .semantic_warnings
                .push(format!("duplicate IDENTIFIER {id:?} (in {kind})"));
        }
    };

    let Some(cc) = &bundle.core_content else {
        return;
    };
    let Some(content) = &cc.req_if_content else {
        return;
    };

    if let Some(dts) = &content.data_types {
        for dt in dts {
            check(dt.identifier().as_str(), "DATATYPES");
        }
    }
    if let Some(sts) = &content.spec_types {
        for st in sts {
            check(st.identifier().as_str(), "SPEC-TYPES");
        }
    }
    if let Some(objs) = &content.spec_objects {
        for o in objs {
            check(o.identifier.as_str(), "SPEC-OBJECTS");
        }
    }
    if let Some(specs) = &content.specifications {
        for s in specs {
            check(s.identifier.as_str(), "SPECIFICATIONS");
        }
    }
    if let Some(srs) = &content.spec_relations {
        for sr in srs {
            check(sr.identifier.as_str(), "SPEC-RELATIONS");
        }
    }
    if let Some(rgs) = &content.relation_groups {
        for rg in rgs {
            check(rg.identifier.as_str(), "SPEC-RELATION-GROUPS");
        }
    }
}

fn check_dangling_refs(bundle: &ReqIfBundle, report: &mut ValidateReport) {
    let Some(cc) = &bundle.core_content else {
        return;
    };
    let Some(content) = &cc.req_if_content else {
        return;
    };

    let spec_type_ids: HashSet<&str> = content
        .spec_types
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .map(|st| st.identifier().as_str())
        .collect();

    let spec_object_ids: HashSet<&str> = content
        .spec_objects
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .map(|so| so.identifier.as_str())
        .collect();

    if let Some(objs) = &content.spec_objects {
        for o in objs {
            if !spec_type_ids.contains(o.spec_object_type.as_str()) {
                report.semantic_warnings.push(format!(
                    "<SPEC-OBJECT IDENTIFIER={:?}> references unknown SPEC-OBJECT-TYPE-REF {:?}",
                    o.identifier.as_str(),
                    o.spec_object_type.as_str()
                ));
            }
        }
    }

    if let Some(specs) = &content.specifications {
        for s in specs {
            if let Some(st_ref) = &s.specification_type
                && !spec_type_ids.contains(st_ref.as_str())
            {
                report.semantic_warnings.push(format!(
                    "<SPECIFICATION IDENTIFIER={:?}> references unknown SPECIFICATION-TYPE-REF {:?}",
                    s.identifier.as_str(),
                    st_ref.as_str()
                ));
            }
        }
    }

    if let Some(srs) = &content.spec_relations {
        for sr in srs {
            if !spec_object_ids.contains(sr.source.as_str()) {
                report.semantic_warnings.push(format!(
                    "<SPEC-RELATION IDENTIFIER={:?}> source SPEC-OBJECT-REF {:?} not found",
                    sr.identifier.as_str(),
                    sr.source.as_str()
                ));
            }
            if !spec_object_ids.contains(sr.target.as_str()) {
                report.semantic_warnings.push(format!(
                    "<SPEC-RELATION IDENTIFIER={:?}> target SPEC-OBJECT-REF {:?} not found",
                    sr.identifier.as_str(),
                    sr.target.as_str()
                ));
            }
        }
    }
}

/// For every `<SPEC-HIERARCHY>` reachable from any `<SPECIFICATION>`, verify
/// that its `<OBJECT>/<SPEC-OBJECT-REF>` target exists in the bundle's
/// [`crate::model::ObjectLookup`]. Walks each specification tree via
/// [`ReqIfBundle::iterate_specification_hierarchy`].
fn check_dangling_hierarchy_refs(bundle: &ReqIfBundle, report: &mut ValidateReport) {
    let Some(cc) = &bundle.core_content else {
        return;
    };
    let Some(content) = &cc.req_if_content else {
        return;
    };
    let Some(specs) = &content.specifications else {
        return;
    };

    for spec in specs {
        for node in bundle.iterate_specification_hierarchy(spec) {
            if !bundle.lookup.spec_object_exists(&node.spec_object_ref) {
                report.semantic_warnings.push(format!(
                    "<SPEC-HIERARCHY IDENTIFIER={:?}> references unknown SPEC-OBJECT-REF {:?}",
                    node.identifier,
                    node.spec_object_ref.as_str()
                ));
            }
        }
    }
}

/// Mirrors Python's prologue advisories: documents without an XML
/// declaration and documents declaring a non-UTF-8 encoding both earn a
/// warning. Both land in [`ValidateReport::semantic_warnings`] (Python
/// emits them as non-fatal too).
fn check_xml_prologue(bundle: &ReqIfBundle, report: &mut ValidateReport) {
    if !bundle.namespace_info.doctype_is_present {
        report
            .semantic_warnings
            .push("Document is missing XML declaration".to_string());
    }
    if bundle.namespace_info.encoding.as_deref() != Some("UTF-8") {
        report
            .semantic_warnings
            .push("ReqIF Implementation Guide recommends UTF-8 encoding".to_string());
    }
}

/// Forward any [`crate::error::Issue`]s the parser accumulated
/// during the most recent parse onto the schema-error tier. The
/// `Issue` Display already formats as `"<message> (while <context>)"`.
fn surface_parse_exceptions(bundle: &ReqIfBundle, report: &mut ValidateReport) {
    for w in &bundle.exceptions {
        report.schema_errors.push(w.to_string());
    }
}

fn check_xsd(opts: &ValidateOpts, report: &mut ValidateReport) -> Result<(), ReqIfError> {
    // Extract the bundled schema tree to a tempdir. `xmllint` reads the
    // files synchronously, so we let `TempDir`'s `Drop` clean up once
    // this function returns.
    let tmp = tempfile::tempdir()?;
    SCHEMA_DIR
        .extract(tmp.path())
        .map_err(|e| ReqIfError::Schema(format!("failed to extract embedded schema: {e}")))?;

    let xsd_path = tmp.path().join("reqif.xsd");
    if !xsd_path.exists() {
        return Err(ReqIfError::Schema(
            "bundled reqif.xsd not found after extraction (internal bug)".into(),
        ));
    }

    let output = Command::new("xmllint")
        .arg("--noout")
        .arg("--schema")
        .arg(&xsd_path)
        .arg(&opts.input)
        .output();

    let output = match output {
        Ok(o) => o,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(ReqIfError::Schema(
                "xmllint not found on PATH (required for --use-reqif-schema; \
                 install libxml2-utils on Debian/Ubuntu or libxml2 on macOS via Homebrew)"
                    .into(),
            ));
        }
        Err(e) => return Err(ReqIfError::Io(e)),
    };

    if !output.status.success() {
        let msg = String::from_utf8_lossy(&output.stderr);
        for line in msg.lines() {
            if !line.trim().is_empty() {
                report.schema_errors.push(line.to_owned());
            }
        }
    }
    Ok(())
}
