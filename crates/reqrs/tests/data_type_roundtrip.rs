use pretty_assertions::assert_eq;
use reqrs::DataTypeId;
use reqrs::model::{DataType, DataTypeBoolean, DataTypeCommon};
use reqrs::parse::data_type::parse_data_type;
use reqrs::unparse::data_type::unparse_data_type;

fn round_trip(xml: &str) {
    let dt: DataType = parse_data_type(xml).unwrap();
    let out = unparse_data_type(&dt);
    assert_eq!(out, xml, "round-trip mismatch");
}

#[test]
fn string_self_closed() {
    round_trip(
        "        <DATATYPE-DEFINITION-STRING IDENTIFIER=\"DT-STR\" LONG-NAME=\"S\" MAX-LENGTH=\"1024\"/>\n",
    );
}

#[test]
fn boolean_self_closed() {
    round_trip("        <DATATYPE-DEFINITION-BOOLEAN IDENTIFIER=\"DT-BOOL\" LONG-NAME=\"B\"/>\n");
}

#[test]
fn integer_self_closed_with_min_max() {
    round_trip(
        "        <DATATYPE-DEFINITION-INTEGER IDENTIFIER=\"DT-INT\" MAX=\"100\" MIN=\"0\"/>\n",
    );
}

#[test]
fn real_emits_open_close() {
    round_trip(
        "        <DATATYPE-DEFINITION-REAL ACCURACY=\"10\" IDENTIFIER=\"DT-REAL\" MAX=\"1234.5\" MIN=\"-1234.5\"/>\n",
    );
}

#[test]
fn xhtml_self_closed() {
    round_trip("        <DATATYPE-DEFINITION-XHTML IDENTIFIER=\"DT-XHTML\"/>\n");
}

#[test]
fn date_self_closed() {
    round_trip("        <DATATYPE-DEFINITION-DATE IDENTIFIER=\"DT-DATE\"/>\n");
}

#[test]
fn enumeration_with_self_closed_specified_values() {
    let xml = r#"        <DATATYPE-DEFINITION-ENUMERATION IDENTIFIER="DT-E2">
          <SPECIFIED-VALUES/>
        </DATATYPE-DEFINITION-ENUMERATION>
"#;
    round_trip(xml);
}

#[test]
fn enumeration_with_values() {
    let xml = r#"        <DATATYPE-DEFINITION-ENUMERATION IDENTIFIER="DT-E" LONG-NAME="E">
          <SPECIFIED-VALUES>
            <ENUM-VALUE IDENTIFIER="E-1" LONG-NAME="One">
              <PROPERTIES>
                <EMBEDDED-VALUE KEY="0"/>
              </PROPERTIES>
            </ENUM-VALUE>
          </SPECIFIED-VALUES>
        </DATATYPE-DEFINITION-ENUMERATION>
"#;
    round_trip(xml);
}

#[test]
fn data_type_with_comment_before_emits_above_element() {
    // The standalone `parse_data_type` skips events before the first start
    // (so a leading `<!--` outside the element is not visible to it), but a
    // DataType value constructed with a non-empty `comments_before` must emit
    // those comment lines at the 8-space DATATYPE-DEFINITION-* indent
    // immediately above the outer tag. Mirrors the Polarion fixture's
    // `<!-- "Boolean" data type definition -->` line.
    let dt = DataType::Boolean(DataTypeBoolean {
        identifier: DataTypeId::new("DT-BOOL"),
        common: DataTypeCommon {
            description: None,
            last_change: None,
            long_name: Some("Boolean".into()),
            was_self_closing: true,
            comments_before: vec![" Boolean data type ".into()],
        },
    });
    let out = unparse_data_type(&dt);
    let expected = "        <!-- Boolean data type -->\n        <DATATYPE-DEFINITION-BOOLEAN IDENTIFIER=\"DT-BOOL\" LONG-NAME=\"Boolean\"/>\n";
    assert_eq!(out, expected);
}
