//! Top-level `ReqIfParser` / `ReqIfUnparser` round-trip tests.
//!
//! These integration tests exercise the full driver — XML declaration
//! sniffing, root attribute harvest, child dispatch, and the inverse emit
//! path. The non-trivial third test additionally validates that the
//! per-element parsers and unparsers compose correctly through the driver.

use pretty_assertions::assert_eq;
use reqrs::{FormatMode, ReqIfParser, ReqIfUnparser};

const MINIMAL: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<REQ-IF xmlns="http://www.omg.org/spec/ReqIF/20110401/reqif.xsd"/>
"#;

#[test]
fn minimal_round_trip() {
    let bundle = ReqIfParser::parse_str(MINIMAL).unwrap();
    assert!(bundle.namespace_info.doctype_is_present);
    assert_eq!(
        bundle.namespace_info.namespace.as_deref(),
        Some("http://www.omg.org/spec/ReqIF/20110401/reqif.xsd")
    );
    assert!(bundle.header.is_none());
    assert!(bundle.core_content.is_none());
    assert!(!bundle.tool_extensions.is_present());

    let out = ReqIfUnparser::unparse(&bundle, FormatMode::Passthrough).unwrap();
    assert_eq!(out, MINIMAL);
}

#[test]
fn empty_reqif_with_no_xml_declaration() {
    let src = "<REQ-IF xmlns=\"http://www.omg.org/spec/ReqIF/20110401/reqif.xsd\"/>\n";
    let bundle = ReqIfParser::parse_str(src).unwrap();
    assert!(!bundle.namespace_info.doctype_is_present);

    let out = ReqIfUnparser::unparse(&bundle, FormatMode::Passthrough).unwrap();
    assert_eq!(out, src);
}

/// Non-trivial round-trip: one DATATYPE-DEFINITION-STRING + one
/// SPEC-OBJECT-TYPE referencing it + one SPEC-OBJECT instance.
/// Exercises the per-element parsers and unparsers in the same pipeline.
#[test]
fn full_pipeline_round_trip_with_datatype_and_spec_type_and_spec_object() {
    let src = r#"<?xml version="1.0" encoding="UTF-8"?>
<REQ-IF xmlns="http://www.omg.org/spec/ReqIF/20110401/reqif.xsd">
  <THE-HEADER>
    <REQ-IF-HEADER IDENTIFIER="hdr-1">
      <TITLE>doc</TITLE>
    </REQ-IF-HEADER>
  </THE-HEADER>
  <CORE-CONTENT>
    <REQ-IF-CONTENT>
      <DATATYPES>
        <DATATYPE-DEFINITION-STRING IDENTIFIER="DT-1" LONG-NAME="text" MAX-LENGTH="255"/>
      </DATATYPES>
      <SPEC-TYPES>
        <SPEC-OBJECT-TYPE IDENTIFIER="SOT-1" LONG-NAME="Req"/>
      </SPEC-TYPES>
      <SPEC-OBJECTS>
        <SPEC-OBJECT IDENTIFIER="SO-1">
          <TYPE>
            <SPEC-OBJECT-TYPE-REF>SOT-1</SPEC-OBJECT-TYPE-REF>
          </TYPE>
          <VALUES/>
        </SPEC-OBJECT>
      </SPEC-OBJECTS>
    </REQ-IF-CONTENT>
  </CORE-CONTENT>
</REQ-IF>
"#;
    let bundle = ReqIfParser::parse_str(src).unwrap();

    // Sanity-check the parsed structure before round-trip.
    let content = bundle
        .core_content
        .as_ref()
        .and_then(|cc| cc.req_if_content.as_ref())
        .expect("REQ-IF-CONTENT must be present");
    assert_eq!(content.data_types.as_ref().map(Vec::len), Some(1));
    assert_eq!(content.spec_types.as_ref().map(Vec::len), Some(1));
    assert_eq!(content.spec_objects.as_ref().map(Vec::len), Some(1));

    // Lookup must index all three.
    assert!(
        bundle
            .lookup
            .data_types
            .contains_key(&reqrs::DataTypeId::new("DT-1"))
    );
    assert!(
        bundle
            .lookup
            .spec_types
            .contains_key(&reqrs::SpecTypeId::new("SOT-1"))
    );
    assert!(
        bundle
            .lookup
            .spec_objects
            .contains_key(&reqrs::SpecObjectId::new("SO-1"))
    );

    let out = ReqIfUnparser::unparse(&bundle, FormatMode::Passthrough).unwrap();
    assert_eq!(out, src);
}

/// Non-empty `<TOOL-EXTENSIONS>` body must round-trip verbatim. Vendor tools
/// (Polarion, Doors) emit opaque XML payloads here; we capture the inner
/// bytes via `capture_inner_raw` and splice them back unchanged on unparse —
/// the same pattern used for XHTML attribute values.
#[test]
fn tool_extensions_non_empty_content_round_trip() {
    let src = r#"<?xml version="1.0" encoding="UTF-8"?>
<REQ-IF xmlns="http://www.omg.org/spec/ReqIF/20110401/reqif.xsd">
  <TOOL-EXTENSIONS>
    <ReqIFToolExtension>
      <tool-data>vendor-specific data</tool-data>
    </ReqIFToolExtension>
  </TOOL-EXTENSIONS>
</REQ-IF>
"#;
    let bundle = ReqIfParser::parse_str(src).unwrap();

    // The model must classify this as Content(raw), not EmptyOpenClose.
    match &bundle.tool_extensions {
        reqrs::model::ToolExtensions::Content(raw) => {
            // The captured inner bytes must include the child element and its
            // text payload — the surrounding whitespace is also preserved
            // verbatim, which is what makes the byte-equal round-trip work.
            assert!(
                raw.contains("<ReqIFToolExtension>"),
                "captured raw should contain child element, got {raw:?}"
            );
            assert!(
                raw.contains("vendor-specific data"),
                "captured raw should contain text payload, got {raw:?}"
            );
        }
        other => panic!("expected ToolExtensions::Content, got {other:?}"),
    }

    let out = ReqIfUnparser::unparse(&bundle, FormatMode::Passthrough).unwrap();
    assert_eq!(out, src);
}

/// Self-closed `<TOOL-EXTENSIONS/>` must round-trip as self-closed.
#[test]
fn tool_extensions_self_closed_round_trip() {
    let src = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
<REQ-IF xmlns=\"http://www.omg.org/spec/ReqIF/20110401/reqif.xsd\">\n  <TOOL-EXTENSIONS/>\n</REQ-IF>\n";
    let bundle = ReqIfParser::parse_str(src).unwrap();
    assert_eq!(
        bundle.tool_extensions,
        reqrs::model::ToolExtensions::SelfClosed
    );

    let out = ReqIfUnparser::unparse(&bundle, FormatMode::Passthrough).unwrap();
    assert_eq!(out, src);
}

/// `<TOOL-EXTENSIONS>` empty open/close form must round-trip as open/close
/// (NOT promoted to self-closed) — the Mode 3 invariant is preserved.
#[test]
fn tool_extensions_empty_open_close_round_trip() {
    let src = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
<REQ-IF xmlns=\"http://www.omg.org/spec/ReqIF/20110401/reqif.xsd\">\n  <TOOL-EXTENSIONS>\n  </TOOL-EXTENSIONS>\n</REQ-IF>\n";
    let bundle = ReqIfParser::parse_str(src).unwrap();
    assert_eq!(
        bundle.tool_extensions,
        reqrs::model::ToolExtensions::EmptyOpenClose
    );

    let out = ReqIfUnparser::unparse(&bundle, FormatMode::Passthrough).unwrap();
    assert_eq!(out, src);
}
