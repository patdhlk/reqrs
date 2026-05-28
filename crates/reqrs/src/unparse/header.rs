use crate::model::{RepositoryId, ReqIfHeader};

pub fn unparse_header(h: &ReqIfHeader) -> String {
    let mut out = String::new();
    out.push_str("  <THE-HEADER>\n");
    out.push_str("    <REQ-IF-HEADER IDENTIFIER=\"");
    out.push_str(&h.identifier);
    out.push_str("\">\n");

    write_text_child(&mut out, "COMMENT", h.comment.as_deref());
    write_text_child(&mut out, "CREATION-TIME", h.creation_time.as_deref());
    match &h.repository_id {
        Some(RepositoryId::Text(t)) => write_text_child(&mut out, "REPOSITORY-ID", Some(t)),
        Some(RepositoryId::Empty) => out.push_str("      <REPOSITORY-ID/>\n"),
        None => {}
    }
    write_text_child(&mut out, "REQ-IF-TOOL-ID", h.req_if_tool_id.as_deref());
    write_text_child(&mut out, "REQ-IF-VERSION", h.req_if_version.as_deref());
    write_text_child(&mut out, "SOURCE-TOOL-ID", h.source_tool_id.as_deref());
    write_text_child(&mut out, "TITLE", h.title.as_deref());

    out.push_str("    </REQ-IF-HEADER>\n");
    out.push_str("  </THE-HEADER>\n");
    out
}

fn write_text_child(out: &mut String, tag: &str, value: Option<&str>) {
    if let Some(v) = value {
        out.push_str("      <");
        out.push_str(tag);
        out.push('>');
        out.push_str(v);
        out.push_str("</");
        out.push_str(tag);
        out.push_str(">\n");
    }
}
