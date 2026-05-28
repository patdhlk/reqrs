// Helpers below are consumed by downstream parser tasks; suppress dead-code
// while the wiring is still being built up.
#![allow(dead_code)]

use crate::error::ReqIfError;
use quick_xml::Reader;
use quick_xml::events::{BytesEnd, BytesStart, Event};

pub(crate) struct ReqIfReader<'a> {
    inner: Reader<&'a [u8]>,
    buf: Vec<u8>,
}

impl<'a> ReqIfReader<'a> {
    pub fn new(src: &'a [u8]) -> Self {
        let mut inner = Reader::from_reader(src);
        inner.config_mut().trim_text(false);
        Self {
            inner,
            buf: Vec::with_capacity(1024),
        }
    }

    pub fn buffer_position(&self) -> usize {
        self.inner.buffer_position() as usize
    }

    pub fn read_event(&mut self) -> Result<Event<'_>, ReqIfError> {
        self.buf.clear();
        self.inner
            .read_event_into(&mut self.buf)
            .map_err(|e| ReqIfError::Xml {
                pos: self.inner.buffer_position() as usize,
                msg: e.to_string(),
            })
    }

    /// Returns the text content of the current element (assumes the
    /// next event is Text or End). Reads until matching end tag.
    pub fn read_text_to_end(&mut self, end: &BytesEnd<'_>) -> Result<String, ReqIfError> {
        let name = end.name().as_ref().to_vec();
        let mut out = String::new();
        loop {
            self.buf.clear();
            match self.inner.read_event_into(&mut self.buf) {
                Ok(Event::Text(t)) => {
                    let s = t.unescape().map_err(|e| ReqIfError::Xml {
                        pos: self.inner.buffer_position() as usize,
                        msg: e.to_string(),
                    })?;
                    out.push_str(&s);
                }
                Ok(Event::CData(c)) => out.push_str(&String::from_utf8_lossy(c.as_ref())),
                Ok(Event::End(e)) if e.name().as_ref() == name => return Ok(out),
                Ok(Event::Eof) => {
                    return Err(ReqIfError::Xml {
                        pos: self.inner.buffer_position() as usize,
                        msg: format!("unexpected EOF inside <{}>", String::from_utf8_lossy(&name)),
                    });
                }
                Ok(_) => continue,
                Err(e) => {
                    return Err(ReqIfError::Xml {
                        pos: self.inner.buffer_position() as usize,
                        msg: e.to_string(),
                    });
                }
            }
        }
    }

    /// Skip events until the matching end tag for `start`.
    pub fn skip_to_end(&mut self, start: &BytesStart<'_>) -> Result<(), ReqIfError> {
        let name = start.name().as_ref().to_vec();
        let mut depth = 1usize;
        loop {
            self.buf.clear();
            match self
                .inner
                .read_event_into(&mut self.buf)
                .map_err(|e| ReqIfError::Xml {
                    pos: self.inner.buffer_position() as usize,
                    msg: e.to_string(),
                })? {
                Event::Start(s) if s.name().as_ref() == name => depth += 1,
                Event::End(e) if e.name().as_ref() == name => {
                    depth -= 1;
                    if depth == 0 {
                        return Ok(());
                    }
                }
                Event::Eof => {
                    return Err(ReqIfError::Xml {
                        pos: self.inner.buffer_position() as usize,
                        msg: format!("unexpected EOF inside <{}>", String::from_utf8_lossy(&name)),
                    });
                }
                _ => continue,
            }
        }
    }
}

/// Look up a required attribute on a start event, returning a descriptive error otherwise.
pub(crate) fn required_attr(start: &BytesStart<'_>, name: &str) -> Result<String, ReqIfError> {
    for attr in start.attributes().flatten() {
        if attr.key.as_ref() == name.as_bytes() {
            return Ok(String::from_utf8_lossy(&attr.value).into_owned());
        }
    }
    Err(ReqIfError::MissingAttribute {
        tag: String::from_utf8_lossy(start.name().as_ref()).into_owned(),
        attr: name.into(),
    })
}

/// Look up an optional attribute on a start event.
pub(crate) fn optional_attr(start: &BytesStart<'_>, name: &str) -> Option<String> {
    start
        .attributes()
        .flatten()
        .find(|a| a.key.as_ref() == name.as_bytes())
        .map(|a| String::from_utf8_lossy(&a.value).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn required_attr_returns_value() {
        let xml = br#"<TAG ID="abc"/>"#;
        let mut r = ReqIfReader::new(xml);
        match r.read_event().unwrap() {
            Event::Empty(s) => {
                assert_eq!(required_attr(&s, "ID").unwrap(), "abc");
            }
            other => panic!("expected Empty, got {other:?}"),
        }
    }

    #[test]
    fn required_attr_missing_returns_error() {
        let xml = br#"<TAG OTHER="x"/>"#;
        let mut r = ReqIfReader::new(xml);
        match r.read_event().unwrap() {
            Event::Empty(s) => {
                let err = required_attr(&s, "ID").unwrap_err();
                assert!(matches!(err, ReqIfError::MissingAttribute { .. }));
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn skip_to_end_handles_nesting() {
        let xml = br#"<A><A><B/></A><C/></A>"#;
        let mut r = ReqIfReader::new(xml);
        let start = match r.read_event().unwrap() {
            Event::Start(s) => s.into_owned(),
            _ => unreachable!(),
        };
        r.skip_to_end(&start).unwrap();
        // After skipping the outer A, the next event is Eof.
        assert!(matches!(r.read_event().unwrap(), Event::Eof));
    }
}
