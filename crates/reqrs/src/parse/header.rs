use crate::error::ReqIfError;
use crate::model::{RepositoryId, ReqIfHeader};
use crate::parse::reader::{ReqIfReader, required_attr};
use quick_xml::events::Event;

pub fn parse_header(xml: &str) -> Result<ReqIfHeader, ReqIfError> {
    let mut r = ReqIfReader::new(xml.as_bytes());
    parse_header_from_reader(&mut r)
}

/// Parse a `<REQ-IF-HEADER>` from a reader that is positioned anywhere before the
/// `<REQ-IF-HEADER>` start tag. Consumes events up through the matching `</REQ-IF-HEADER>`.
pub(crate) fn parse_header_from_reader(r: &mut ReqIfReader<'_>) -> Result<ReqIfHeader, ReqIfError> {
    // Skip until <REQ-IF-HEADER>.
    let identifier = loop {
        match r.read_event()? {
            Event::Start(s) if s.name().as_ref() == b"REQ-IF-HEADER" => {
                // Extract the IDENTIFIER attribute into an owned String before re-borrowing `r`.
                break required_attr(&s, "IDENTIFIER")?;
            }
            Event::Eof => {
                return Err(ReqIfError::MissingChild {
                    child: "REQ-IF-HEADER".into(),
                    parent: "THE-HEADER".into(),
                });
            }
            _ => continue,
        }
    };
    parse_reqif_header_body(r, identifier)
}

fn parse_reqif_header_body(
    r: &mut ReqIfReader<'_>,
    identifier: String,
) -> Result<ReqIfHeader, ReqIfError> {
    let mut header = ReqIfHeader {
        identifier,
        comment: None,
        creation_time: None,
        repository_id: None,
        req_if_tool_id: None,
        req_if_version: None,
        source_tool_id: None,
        title: None,
    };
    loop {
        match r.read_event()? {
            Event::Start(s) => {
                let name = s.name().as_ref().to_vec();
                let end = s.to_end().into_owned();
                let text = r.read_text_to_end(&end)?;
                let text = text.trim().to_owned();
                let value = if text.is_empty() { None } else { Some(text) };
                match name.as_slice() {
                    b"COMMENT" => header.comment = value,
                    b"CREATION-TIME" => header.creation_time = value,
                    b"REPOSITORY-ID" => header.repository_id = value.map(RepositoryId::Text),
                    b"REQ-IF-TOOL-ID" => header.req_if_tool_id = value,
                    b"REQ-IF-VERSION" => header.req_if_version = value,
                    b"SOURCE-TOOL-ID" => header.source_tool_id = value,
                    b"TITLE" => header.title = value,
                    _ => {}
                }
            }
            Event::Empty(s) if s.name().as_ref() == b"REPOSITORY-ID" => {
                header.repository_id = Some(RepositoryId::Empty);
            }
            Event::End(e) if e.name().as_ref() == b"REQ-IF-HEADER" => return Ok(header),
            Event::Eof => {
                return Err(ReqIfError::Xml {
                    pos: r.buffer_position(),
                    msg: "EOF inside <REQ-IF-HEADER>".into(),
                });
            }
            _ => continue,
        }
    }
}
