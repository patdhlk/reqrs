use crate::model::{RepositoryId, ReqIfHeader};
use crate::unparse::writer::{escape_attr, write_text_element};

pub fn unparse_header(h: &ReqIfHeader) -> String {
    let mut out = String::new();
    out.push_str("  <THE-HEADER>\n");
    out.push_str("    <REQ-IF-HEADER IDENTIFIER=\"");
    escape_attr(&mut out, &h.identifier);
    out.push_str("\">\n");

    write_text_element(&mut out, "      ", "COMMENT", h.comment.as_deref());
    write_text_element(
        &mut out,
        "      ",
        "CREATION-TIME",
        h.creation_time.as_deref(),
    );
    match &h.repository_id {
        Some(RepositoryId::Text(t)) => {
            write_text_element(&mut out, "      ", "REPOSITORY-ID", Some(t))
        }
        Some(RepositoryId::Empty) => out.push_str("      <REPOSITORY-ID/>\n"),
        None => {}
    }
    write_text_element(
        &mut out,
        "      ",
        "REQ-IF-TOOL-ID",
        h.req_if_tool_id.as_deref(),
    );
    write_text_element(
        &mut out,
        "      ",
        "REQ-IF-VERSION",
        h.req_if_version.as_deref(),
    );
    write_text_element(
        &mut out,
        "      ",
        "SOURCE-TOOL-ID",
        h.source_tool_id.as_deref(),
    );
    write_text_element(&mut out, "      ", "TITLE", h.title.as_deref());

    out.push_str("    </REQ-IF-HEADER>\n");
    out.push_str("  </THE-HEADER>\n");
    out
}
