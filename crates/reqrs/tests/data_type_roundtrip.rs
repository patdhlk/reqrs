use pretty_assertions::assert_eq;
use reqrs::model::DataType;
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
