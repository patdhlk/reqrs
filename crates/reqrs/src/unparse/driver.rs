//! Top-level orchestrator: emit a full `<REQ-IF>` document.
//!
//! Mirrors `strict-doc-reqif/reqif/unparser.py::ReqIFUnparser.unparse` and
//! `unparse_namespace_info`. The driver is intentionally thin — it only:
//!
//! - emits `<?xml version="1.0" encoding="UTF-8"?>` (when
//!   [`NamespaceInfo::doctype_is_present`]),
//! - reconstructs the `<REQ-IF ...>` opener with attributes in canonical order,
//! - delegates each per-element block to the appropriate `unparse_*` helper,
//! - re-emits a `<TOOL-EXTENSIONS/>` placeholder when the parsed source had one.
//!
//! Attribute order on the root opener: `xmlns`, `xmlns:xsi`,
//! `xmlns:configuration`, `xmlns:id`, `xmlns:xhtml`, `xsi:schemaLocation`,
//! `xml:lang`. This matches the Python reference's `unparse_namespace_info`
//! ordering and the order observed in every ReqIF tool fixture (Polarion,
//! Doors, Eclipse RMF).

use crate::error::ReqIfError;
use crate::model::{NamespaceInfo, ReqIfBundle};
use crate::unparse::data_type::unparse_data_type;
use crate::unparse::header::unparse_header;
use crate::unparse::relation_group::unparse_relation_group;
use crate::unparse::spec_object::unparse_spec_object;
use crate::unparse::spec_relation::unparse_spec_relation;
use crate::unparse::spec_type::unparse_spec_type;
use crate::unparse::specification::unparse_specification;
use crate::unparse::writer::{FormatMode, escape_attr};

/// Emit a full `<REQ-IF>` document. `mode` is currently unused — both
/// `Passthrough` and `Canonical` go through the same path because the
/// per-element unparsers consult their own `was_self_closing` flags.
pub fn unparse_bundle(bundle: &ReqIfBundle, _mode: FormatMode) -> Result<String, ReqIfError> {
    let mut out = String::new();

    if bundle.namespace_info.doctype_is_present {
        out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    }

    let self_closed_root = is_self_closed_root(bundle);
    write_root_opener(&mut out, &bundle.namespace_info, self_closed_root);
    if self_closed_root {
        return Ok(out);
    }

    if let Some(header) = &bundle.header {
        out.push_str(&unparse_header(header));
    }

    if let Some(core_content) = &bundle.core_content {
        out.push_str("  <CORE-CONTENT>\n");
        if let Some(content) = &core_content.req_if_content {
            out.push_str("    <REQ-IF-CONTENT>\n");

            if let Some(data_types) = &content.data_types {
                if data_types.is_empty() {
                    out.push_str("      <DATATYPES/>\n");
                } else {
                    out.push_str("      <DATATYPES>\n");
                    for dt in data_types {
                        out.push_str(&unparse_data_type(dt));
                    }
                    out.push_str("      </DATATYPES>\n");
                }
            }

            if let Some(spec_types) = &content.spec_types {
                if spec_types.is_empty() {
                    out.push_str("      <SPEC-TYPES/>\n");
                } else {
                    out.push_str("      <SPEC-TYPES>\n");
                    // unparse_spec_type dispatches per-variant internally, so no
                    // isinstance ladder is needed at this layer (cf. Python
                    // unparser.py:55-69, which inspects the variant in-place).
                    for st in spec_types {
                        out.push_str(&unparse_spec_type(st));
                    }
                    out.push_str("      </SPEC-TYPES>\n");
                }
            }

            if let Some(spec_objects) = &content.spec_objects {
                if spec_objects.is_empty() {
                    out.push_str("      <SPEC-OBJECTS/>\n");
                } else {
                    out.push_str("      <SPEC-OBJECTS>\n");
                    for so in spec_objects {
                        out.push_str(&unparse_spec_object(so));
                    }
                    out.push_str("      </SPEC-OBJECTS>\n");
                }
            }

            if let Some(spec_relations) = &content.spec_relations {
                if spec_relations.is_empty() {
                    out.push_str("      <SPEC-RELATIONS/>\n");
                } else {
                    out.push_str("      <SPEC-RELATIONS>\n");
                    for sr in spec_relations {
                        out.push_str(&unparse_spec_relation(sr));
                    }
                    out.push_str("      </SPEC-RELATIONS>\n");
                }
            }

            if let Some(specs) = &content.specifications {
                if specs.is_empty() {
                    out.push_str("      <SPECIFICATIONS/>\n");
                } else {
                    out.push_str("      <SPECIFICATIONS>\n");
                    for spec in specs {
                        out.push_str(&unparse_specification(spec));
                    }
                    out.push_str("      </SPECIFICATIONS>\n");
                }
            }

            if let Some(groups) = &content.relation_groups {
                if groups.is_empty() {
                    out.push_str("      <SPEC-RELATION-GROUPS/>\n");
                } else {
                    out.push_str("      <SPEC-RELATION-GROUPS>\n");
                    for rg in groups {
                        out.push_str(&unparse_relation_group(rg));
                    }
                    out.push_str("      </SPEC-RELATION-GROUPS>\n");
                }
            }

            out.push_str("    </REQ-IF-CONTENT>\n");
        }
        out.push_str("  </CORE-CONTENT>\n");
    }

    if bundle.tool_extensions_tag_exists {
        // Python re-emits as `<TOOL-EXTENSIONS>\n  </TOOL-EXTENSIONS>\n` (open
        // + close on separate lines). For v1 we collapse to self-closed: every
        // fixture we have either has an open/close pair with no body or a
        // self-closed empty. Both forms semantically denote the same content.
        // If round-trip on the corpus shows divergence we'll widen this.
        out.push_str("  <TOOL-EXTENSIONS/>\n");
    }

    out.push_str("</REQ-IF>\n");
    Ok(out)
}

/// A `<REQ-IF>` opens self-closed iff it has no children at all and the
/// source parser saw the self-closed form. We use "no children" as the
/// signal — if the bundle has no header, no core_content, and no
/// tool_extensions tag, we emit `<REQ-IF .../>` instead of an open/close pair.
fn is_self_closed_root(bundle: &ReqIfBundle) -> bool {
    bundle.header.is_none() && bundle.core_content.is_none() && !bundle.tool_extensions_tag_exists
}

/// Reconstruct the `<REQ-IF>` opener.
///
/// If [`NamespaceInfo::attributes_in_order`] is non-empty (the normal
/// parse-then-unparse path), walk it verbatim — this preserves both the
/// source-order of attributes AND any vendor-specific xmlns declarations
/// (e.g. `xmlns:doors`, `xmlns:reqif-common`) that the typed fields do not
/// model. This is required for byte-exact round-trip on Doors and other
/// vendor fixtures.
///
/// Otherwise (synthetic bundle built via [`Default`]), fall back to the
/// canonical attribute order used by the Python reference and observed
/// across Polarion / Eclipse RMF fixtures:
///
/// 1. `xmlns`
/// 2. `xmlns:xsi`
/// 3. `xmlns:configuration`
/// 4. `xmlns:id`
/// 5. `xmlns:xhtml`
/// 6. `xsi:schemaLocation`
/// 7. `xml:lang`
///
/// The closing `>` vs `/>` is chosen by `self_closed`.
fn write_root_opener(out: &mut String, ns: &NamespaceInfo, self_closed: bool) {
    out.push_str("<REQ-IF");

    if ns.attributes_in_order.is_empty() {
        push_attr(out, "xmlns", ns.namespace.as_deref());
        push_attr(out, "xmlns:xsi", ns.schema_namespace.as_deref());
        push_attr(out, "xmlns:configuration", ns.configuration.as_deref());
        push_attr(out, "xmlns:id", ns.namespace_id.as_deref());
        push_attr(out, "xmlns:xhtml", ns.namespace_xhtml.as_deref());
        push_attr(out, "xsi:schemaLocation", ns.schema_location.as_deref());
        push_attr(out, "xml:lang", ns.language.as_deref());
    } else {
        for (name, value) in &ns.attributes_in_order {
            push_attr(out, name, Some(value.as_str()));
        }
    }

    if self_closed {
        out.push_str("/>\n");
    } else {
        out.push_str(">\n");
    }
}

fn push_attr(out: &mut String, name: &str, value: Option<&str>) {
    let Some(v) = value else { return };
    out.push(' ');
    out.push_str(name);
    out.push_str("=\"");
    escape_attr(out, v);
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ns_minimal() -> NamespaceInfo {
        NamespaceInfo {
            doctype_is_present: true,
            encoding: Some("UTF-8".into()),
            namespace: Some("http://www.omg.org/spec/ReqIF/20110401/reqif.xsd".into()),
            ..Default::default()
        }
    }

    #[test]
    fn opener_emits_only_present_attributes_in_canonical_order() {
        let mut out = String::new();
        let ns = NamespaceInfo {
            namespace: Some("ns".into()),
            schema_namespace: Some("xsi-ns".into()),
            configuration: Some("cfg".into()),
            namespace_id: Some("id-ns".into()),
            namespace_xhtml: Some("xhtml-ns".into()),
            schema_location: Some("loc".into()),
            language: Some("en".into()),
            ..Default::default()
        };
        write_root_opener(&mut out, &ns, false);
        assert_eq!(
            out,
            "<REQ-IF xmlns=\"ns\" xmlns:xsi=\"xsi-ns\" xmlns:configuration=\"cfg\" xmlns:id=\"id-ns\" xmlns:xhtml=\"xhtml-ns\" xsi:schemaLocation=\"loc\" xml:lang=\"en\">\n"
        );
    }

    #[test]
    fn opener_self_closed_when_requested() {
        let mut out = String::new();
        write_root_opener(&mut out, &ns_minimal(), true);
        assert_eq!(
            out,
            "<REQ-IF xmlns=\"http://www.omg.org/spec/ReqIF/20110401/reqif.xsd\"/>\n"
        );
    }

    #[test]
    fn opener_skips_absent_attributes() {
        let mut out = String::new();
        let ns = NamespaceInfo {
            namespace: Some("ns".into()),
            ..Default::default()
        };
        write_root_opener(&mut out, &ns, false);
        assert_eq!(out, "<REQ-IF xmlns=\"ns\">\n");
    }

    #[test]
    fn opener_replays_attributes_in_order_when_present() {
        // Vendor-specific xmlns declarations interleaved with standard ones,
        // in a non-canonical order — mirrors what Doors fixtures emit.
        let mut out = String::new();
        let ns = NamespaceInfo {
            // Typed fields are deliberately left out to prove the unparser
            // does NOT consult them when `attributes_in_order` is populated.
            attributes_in_order: vec![
                ("xmlns".into(), "ns".into()),
                ("xmlns:doors".into(), "doors-ns".into()),
                ("xmlns:reqif-common".into(), "rc-ns".into()),
                ("xmlns:xsi".into(), "xsi-ns".into()),
            ],
            ..Default::default()
        };
        write_root_opener(&mut out, &ns, false);
        assert_eq!(
            out,
            "<REQ-IF xmlns=\"ns\" xmlns:doors=\"doors-ns\" xmlns:reqif-common=\"rc-ns\" xmlns:xsi=\"xsi-ns\">\n"
        );
    }
}
