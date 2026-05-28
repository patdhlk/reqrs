use pretty_assertions::assert_eq;
use reqrs::{model::ReqIfHeader, parse::header::parse_header, unparse::header::unparse_header};

const SAMPLE: &str = r#"  <THE-HEADER>
    <REQ-IF-HEADER IDENTIFIER="hdr-001">
      <COMMENT>generated</COMMENT>
      <CREATION-TIME>2021-01-02T03:04:05+00:00</CREATION-TIME>
      <REPOSITORY-ID>repo-1</REPOSITORY-ID>
      <REQ-IF-TOOL-ID>tool-x</REQ-IF-TOOL-ID>
      <REQ-IF-VERSION>1.0</REQ-IF-VERSION>
      <SOURCE-TOOL-ID>src-x</SOURCE-TOOL-ID>
      <TITLE>Title</TITLE>
    </REQ-IF-HEADER>
  </THE-HEADER>
"#;

#[test]
fn parse_then_unparse_round_trips_full_header() {
    let h: ReqIfHeader = parse_header(SAMPLE).unwrap();
    assert_eq!(h.identifier.as_str(), "hdr-001");
    assert_eq!(h.title.as_deref(), Some("Title"));

    let out = unparse_header(&h);
    assert_eq!(out, SAMPLE);
}

#[test]
fn parse_then_unparse_round_trips_minimal_header_with_empty_repository_id() {
    let src = r#"  <THE-HEADER>
    <REQ-IF-HEADER IDENTIFIER="h">
      <REPOSITORY-ID/>
    </REQ-IF-HEADER>
  </THE-HEADER>
"#;
    let h = parse_header(src).unwrap();
    let out = unparse_header(&h);
    assert_eq!(out, src);
}
