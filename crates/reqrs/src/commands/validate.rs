//! `validate` command — internal semantic checks over a parsed bundle.
//!
//! Mirrors the always-on portion of
//! `strict-doc-reqif/reqif/commands/validate/validate.py`. The Python
//! implementation supports two layers:
//!
//! 1. Strict XSD conformance via the `xmlschema` library, gated on
//!    `--use-reqif-schema`. Stubbed here; wired in Task 20.
//! 2. Internal semantic checks that always run after a successful parse —
//!    these are what this module implements.
//!
//! The checks are intentionally narrow: anything the parser would reject
//! (missing required attributes, malformed XML) never reaches us.
//! [`validate`] only flags issues that the parser tolerates but that break
//! reference integrity:
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
//!
//! Errors are collected into [`ValidateReport::errors`]; the CLI layer
//! (Task 21) translates a non-empty list into a non-zero exit code.

use std::collections::HashSet;
use std::path::PathBuf;

use crate::error::ReqIfError;
use crate::model::ReqIfBundle;
use crate::parse::ReqIfParser;

#[derive(Debug, Clone)]
pub struct ValidateOpts {
    pub input: PathBuf,
    pub use_reqif_schema: bool,
}

#[derive(Debug, Default, Clone)]
pub struct ValidateReport {
    pub errors: Vec<String>,
}

/// Parse `opts.input` and run the always-on semantic checks. If
/// `opts.use_reqif_schema` is set, also run the XSD conformance check
/// (Task 20). Returns an empty `errors` list when the document is clean.
pub fn validate(opts: ValidateOpts) -> Result<ValidateReport, ReqIfError> {
    let bundle = ReqIfParser::parse_path(&opts.input)?;
    let mut report = ValidateReport::default();

    check_duplicate_identifiers(&bundle, &mut report);
    check_dangling_refs(&bundle, &mut report);

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
                .errors
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
                report.errors.push(format!(
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
                report.errors.push(format!(
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
                report.errors.push(format!(
                    "<SPEC-RELATION IDENTIFIER={:?}> source SPEC-OBJECT-REF {:?} not found",
                    sr.identifier.as_str(),
                    sr.source.as_str()
                ));
            }
            if !spec_object_ids.contains(sr.target.as_str()) {
                report.errors.push(format!(
                    "<SPEC-RELATION IDENTIFIER={:?}> target SPEC-OBJECT-REF {:?} not found",
                    sr.identifier.as_str(),
                    sr.target.as_str()
                ));
            }
        }
    }
}

fn check_xsd(_opts: &ValidateOpts, _report: &mut ValidateReport) -> Result<(), ReqIfError> {
    // Implemented in Task 20 (xmllint shell-out).
    Err(ReqIfError::Schema(
        "schema validation requires Task 20 (xmllint shell-out) to be implemented".into(),
    ))
}
