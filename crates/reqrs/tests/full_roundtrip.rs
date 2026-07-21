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

/// Escaped control whitespace inside attribute values (`&#10;`, `&#9;`,
/// `&#13;`) must round-trip byte-exact. Real exports (IBM Engineering via
/// requisis ReqIF-Manager) carry multi-line `THE-VALUE` / `DESC` attributes
/// this way; writing the decoded characters back literally would lose them —
/// XML attribute-value normalization folds literal #x9/#xA/#xD to spaces on
/// the next conforming parse.
#[test]
fn escaped_newlines_in_attribute_values_round_trip() {
    let src = r#"<?xml version="1.0" encoding="UTF-8"?>
<REQ-IF xmlns="http://www.omg.org/spec/ReqIF/20110401/reqif.xsd">
  <CORE-CONTENT>
    <REQ-IF-CONTENT>
      <SPEC-TYPES>
        <SPEC-OBJECT-TYPE DESC="first line&#10;second line" IDENTIFIER="SOT-1" LONG-NAME="Info"/>
      </SPEC-TYPES>
      <SPEC-OBJECTS>
        <SPEC-OBJECT IDENTIFIER="SO-1">
          <TYPE>
            <SPEC-OBJECT-TYPE-REF>SOT-1</SPEC-OBJECT-TYPE-REF>
          </TYPE>
          <VALUES>
            <ATTRIBUTE-VALUE-STRING THE-VALUE="bullet one&#10;bullet two&#9;indented&#13;cr">
              <DEFINITION>
                <ATTRIBUTE-DEFINITION-STRING-REF>AD-1</ATTRIBUTE-DEFINITION-STRING-REF>
              </DEFINITION>
            </ATTRIBUTE-VALUE-STRING>
          </VALUES>
        </SPEC-OBJECT>
      </SPEC-OBJECTS>
    </REQ-IF-CONTENT>
  </CORE-CONTENT>
</REQ-IF>
"#;
    let bundle = ReqIfParser::parse_str(src).unwrap();

    // The parser must decode the references into real control characters.
    let content = bundle
        .core_content
        .as_ref()
        .and_then(|cc| cc.req_if_content.as_ref())
        .expect("REQ-IF-CONTENT must be present");
    use reqrs::model::SpecType;
    let desc = match &content.spec_types.as_ref().unwrap()[0] {
        SpecType::SpecObject(t) => t.common.description.clone(),
        _ => panic!("expected SpecObject variant"),
    };
    assert_eq!(desc.as_deref(), Some("first line\nsecond line"));

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

/// Inline `<!-- ... -->` comments between siblings in `<SPEC-TYPES>`,
/// `<SPEC-OBJECTS>`, and `<VALUES>` round-trip byte-exact. This is the
/// end-to-end exercise of the same three positions the Polarion fixture uses
/// in `tests/corpus/examples/04_convert_reqif_to_json/sample_polarion_reqifz.reqifz`:
///
/// - between sibling spec-type elements (here: between two SPEC-OBJECT-TYPEs)
/// - before the first sibling inside `<SPEC-OBJECTS>`
/// - between sibling `<ATTRIBUTE-VALUE-*>` elements inside `<VALUES>`
///
/// Each comment is captured on the *next* element's `comments_before` field
/// and re-emitted above it at the element's own indent (8 / 8 / 12 spaces).
#[test]
fn inline_xml_comments_round_trip_between_siblings() {
    let src = r#"<?xml version="1.0" encoding="UTF-8"?>
<REQ-IF xmlns="http://www.omg.org/spec/ReqIF/20110401/reqif.xsd">
  <CORE-CONTENT>
    <REQ-IF-CONTENT>
      <DATATYPES>
        <DATATYPE-DEFINITION-STRING IDENTIFIER="DT-1" LONG-NAME="text" MAX-LENGTH="255"/>
      </DATATYPES>
      <SPEC-TYPES>
        <SPEC-OBJECT-TYPE IDENTIFIER="SOT-1" LONG-NAME="One"/>
        <!-- second spec type follows -->
        <SPEC-OBJECT-TYPE IDENTIFIER="SOT-2" LONG-NAME="Two"/>
      </SPEC-TYPES>
      <SPEC-OBJECTS>
        <!-- first object -->
        <SPEC-OBJECT IDENTIFIER="SO-1">
          <TYPE>
            <SPEC-OBJECT-TYPE-REF>SOT-1</SPEC-OBJECT-TYPE-REF>
          </TYPE>
          <VALUES>
            <!-- chapter name follows -->
            <ATTRIBUTE-VALUE-STRING THE-VALUE="Section">
              <DEFINITION>
                <ATTRIBUTE-DEFINITION-STRING-REF>AD-1</ATTRIBUTE-DEFINITION-STRING-REF>
              </DEFINITION>
            </ATTRIBUTE-VALUE-STRING>
            <!-- body text follows -->
            <ATTRIBUTE-VALUE-STRING THE-VALUE="Body">
              <DEFINITION>
                <ATTRIBUTE-DEFINITION-STRING-REF>AD-2</ATTRIBUTE-DEFINITION-STRING-REF>
              </DEFINITION>
            </ATTRIBUTE-VALUE-STRING>
          </VALUES>
        </SPEC-OBJECT>
      </SPEC-OBJECTS>
    </REQ-IF-CONTENT>
  </CORE-CONTENT>
</REQ-IF>
"#;

    let bundle = ReqIfParser::parse_str(src).unwrap();

    // Sanity: each captured comment landed on the correct element.
    let content = bundle
        .core_content
        .as_ref()
        .and_then(|cc| cc.req_if_content.as_ref())
        .expect("REQ-IF-CONTENT must be present");

    let spec_types = content.spec_types.as_ref().unwrap();
    assert_eq!(spec_types.len(), 2);
    // The second SPEC-OBJECT-TYPE owns the inter-sibling comment; the first owns none.
    use reqrs::model::SpecType;
    let common_of = |st: &SpecType| match st {
        SpecType::SpecObject(t) => t.common.clone(),
        _ => panic!("expected SpecObject variant"),
    };
    assert!(common_of(&spec_types[0]).comments_before.is_empty());
    assert_eq!(
        common_of(&spec_types[1]).comments_before,
        vec![" second spec type follows ".to_string()]
    );

    let spec_objects = content.spec_objects.as_ref().unwrap();
    assert_eq!(spec_objects.len(), 1);
    assert_eq!(
        spec_objects[0].comments_before,
        vec![" first object ".to_string()]
    );

    let values = &spec_objects[0].attributes;
    assert_eq!(values.len(), 2);
    assert_eq!(
        values[0].comments_before(),
        &[" chapter name follows ".to_string()]
    );
    assert_eq!(
        values[1].comments_before(),
        &[" body text follows ".to_string()]
    );

    let out = ReqIfUnparser::unparse(&bundle, FormatMode::Passthrough).unwrap();
    assert_eq!(out, src);
}

/// Multiple comments stacked above one sibling are preserved in source order.
/// This verifies the `pending_comments` accumulator flushes its entire vec
/// onto the next element, not just the latest.
#[test]
fn multiple_consecutive_comments_round_trip_in_order() {
    let src = r#"<?xml version="1.0" encoding="UTF-8"?>
<REQ-IF xmlns="http://www.omg.org/spec/ReqIF/20110401/reqif.xsd">
  <CORE-CONTENT>
    <REQ-IF-CONTENT>
      <SPEC-TYPES>
        <!-- first comment -->
        <!-- second comment -->
        <SPEC-OBJECT-TYPE IDENTIFIER="SOT-1" LONG-NAME="One"/>
      </SPEC-TYPES>
    </REQ-IF-CONTENT>
  </CORE-CONTENT>
</REQ-IF>
"#;
    let bundle = ReqIfParser::parse_str(src).unwrap();
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

/// A comment appearing AFTER the last `<DATATYPE-DEFINITION-*>` and before the
/// closing `</DATATYPES>` lands in `ReqIfContent::data_types_trailing_comments`
/// during parse, and the unparser emits it at the 8-space inner-element indent
/// before `</DATATYPES>`. Closes the trailing-comments gap noted in the
/// container walkers' design notes.
#[test]
fn data_types_with_trailing_comment_round_trips() {
    let src = r#"<?xml version="1.0" encoding="UTF-8"?>
<REQ-IF xmlns="http://www.omg.org/spec/ReqIF/20110401/reqif.xsd">
  <CORE-CONTENT>
    <REQ-IF-CONTENT>
      <DATATYPES>
        <DATATYPE-DEFINITION-STRING IDENTIFIER="DT-1" LONG-NAME="text" MAX-LENGTH="255"/>
        <!-- trailing comment in DATATYPES -->
      </DATATYPES>
    </REQ-IF-CONTENT>
  </CORE-CONTENT>
</REQ-IF>
"#;
    let bundle = ReqIfParser::parse_str(src).unwrap();

    let content = bundle
        .core_content
        .as_ref()
        .and_then(|cc| cc.req_if_content.as_ref())
        .expect("REQ-IF-CONTENT must be present");
    assert_eq!(
        content.data_types_trailing_comments,
        vec![" trailing comment in DATATYPES ".to_string()]
    );

    let out = ReqIfUnparser::unparse(&bundle, FormatMode::Passthrough).unwrap();
    assert_eq!(out, src);
}

/// A comment appearing AFTER the last `<ATTRIBUTE-VALUE-*>` inside
/// `<SPEC-OBJECT>/<VALUES>` and before the closing `</VALUES>` lands in
/// `SpecObject::values_trailing_comments`, and the unparser emits it at the
/// 12-space inner-element indent before `</VALUES>`.
#[test]
fn spec_object_values_with_trailing_comment_round_trips() {
    let src = r#"<?xml version="1.0" encoding="UTF-8"?>
<REQ-IF xmlns="http://www.omg.org/spec/ReqIF/20110401/reqif.xsd">
  <CORE-CONTENT>
    <REQ-IF-CONTENT>
      <SPEC-OBJECTS>
        <SPEC-OBJECT IDENTIFIER="SO-1">
          <TYPE>
            <SPEC-OBJECT-TYPE-REF>SOT-1</SPEC-OBJECT-TYPE-REF>
          </TYPE>
          <VALUES>
            <ATTRIBUTE-VALUE-STRING THE-VALUE="hello">
              <DEFINITION>
                <ATTRIBUTE-DEFINITION-STRING-REF>AD-1</ATTRIBUTE-DEFINITION-STRING-REF>
              </DEFINITION>
            </ATTRIBUTE-VALUE-STRING>
            <!-- trailing comment inside VALUES -->
          </VALUES>
        </SPEC-OBJECT>
      </SPEC-OBJECTS>
    </REQ-IF-CONTENT>
  </CORE-CONTENT>
</REQ-IF>
"#;
    let bundle = ReqIfParser::parse_str(src).unwrap();

    let content = bundle
        .core_content
        .as_ref()
        .and_then(|cc| cc.req_if_content.as_ref())
        .expect("REQ-IF-CONTENT must be present");
    let so = &content.spec_objects.as_ref().unwrap()[0];
    assert_eq!(
        so.values_trailing_comments,
        vec![" trailing comment inside VALUES ".to_string()]
    );

    let out = ReqIfUnparser::unparse(&bundle, FormatMode::Passthrough).unwrap();
    assert_eq!(out, src);
}
