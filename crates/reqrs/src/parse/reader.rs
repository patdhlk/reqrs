use crate::error::ReqIfError;
use quick_xml::Reader;
use quick_xml::events::{BytesEnd, BytesStart, Event};

pub(crate) struct ReqIfReader<'a> {
    inner: Reader<&'a [u8]>,
    /// Original input slice held separately. `inner.get_ref()` returns the
    /// *remaining* slice (it advances on every read), so it cannot be combined
    /// with `buffer_position()` to recover absolute byte ranges. This handle
    /// preserves the absolute view needed by `capture_inner_raw`.
    src: &'a [u8],
    buf: Vec<u8>,
}

impl<'a> ReqIfReader<'a> {
    pub fn new(src: &'a [u8]) -> Self {
        let mut inner = Reader::from_reader(src);
        inner.config_mut().trim_text(false);
        Self {
            inner,
            src,
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

    /// Capture every byte between the cursor (just after a `<TAG>` Start event)
    /// and the matching `</TAG>` (exclusive), as UTF-8.
    ///
    /// **Precondition:** The caller MUST have just consumed `Event::Start(_)` for
    /// `start`. Calling this after `Event::Empty(_)` is a logic error — the helper
    /// would walk forward through unrelated content trying to find a non-existent
    /// close tag. Self-closed elements have no inner content; there is nothing to
    /// capture.
    ///
    /// quick-xml does not expose any signal on `BytesStart` that distinguishes a
    /// `Start` from an `Empty` origin, so the precondition is documented rather
    /// than asserted. Threading the original `BytesStart` through the call (vs.
    /// passing a raw close-tag name) makes misuse far harder: the caller must
    /// already hold the event, which is only emitted on `Start` / `Empty`, and
    /// the type makes the close-tag name derivation an implementation detail.
    ///
    /// Nested elements of the same name are tracked by depth so the helper
    /// terminates at the correct close. The captured bytes are returned with
    /// original whitespace, escaping, and child markup preserved — this is what
    /// makes round-tripping XHTML and `DEFAULT-VALUE` blocks byte-exact.
    pub fn capture_inner_raw(&mut self, start: &BytesStart<'_>) -> Result<String, ReqIfError> {
        let end_name = start.name();
        let end_bytes = end_name.as_ref();
        let begin = self.inner.buffer_position() as usize;
        let mut depth = 1usize;
        loop {
            self.buf.clear();
            let pos_before = self.inner.buffer_position() as usize;
            match self
                .inner
                .read_event_into(&mut self.buf)
                .map_err(|e| ReqIfError::Xml {
                    pos: self.inner.buffer_position() as usize,
                    msg: e.to_string(),
                })? {
                Event::Start(s) if s.name().as_ref() == end_bytes => depth += 1,
                Event::End(e) if e.name().as_ref() == end_bytes => {
                    depth -= 1;
                    if depth == 0 {
                        // pos_before sits at "<" of the closing tag — quick-xml's offset
                        // points just before the markup-open angle when InsideMarkup. Capture
                        // [begin, pos_before) to exclude the close tag itself.
                        return Ok(
                            String::from_utf8_lossy(&self.src[begin..pos_before]).into_owned()
                        );
                    }
                }
                Event::Eof => {
                    return Err(ReqIfError::Xml {
                        pos: self.inner.buffer_position() as usize,
                        msg: format!(
                            "EOF capturing inside <{}>",
                            String::from_utf8_lossy(end_bytes)
                        ),
                    });
                }
                _ => continue,
            }
        }
    }

    /// Skip events until the matching end tag for `start`.
    #[allow(dead_code)]
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

    #[test]
    fn optional_attr_returns_value_when_present() {
        let xml = br#"<TAG NAME="x" OTHER="y"/>"#;
        let mut r = ReqIfReader::new(xml);
        let s = match r.read_event().unwrap() {
            Event::Empty(s) => s.into_owned(),
            _ => unreachable!(),
        };
        assert_eq!(optional_attr(&s, "NAME"), Some("x".to_string()));
        assert_eq!(optional_attr(&s, "MISSING"), None);
    }

    #[test]
    fn read_text_to_end_concatenates_text_and_cdata() {
        let xml = b"<OUTER>hello <![CDATA[<world>]]>!</OUTER>";
        let mut r = ReqIfReader::new(xml);
        let end = match r.read_event().unwrap() {
            Event::Start(s) => s.to_end().into_owned(),
            _ => unreachable!(),
        };
        let text = r.read_text_to_end(&end).unwrap();
        assert_eq!(text, "hello <world>!");
    }

    #[test]
    fn capture_inner_raw_preserves_bytes_inside_element() {
        let xml = b"<OUTER><THE-VALUE>hello <b>world</b></THE-VALUE></OUTER>";
        let mut r = ReqIfReader::new(xml);
        let start = loop {
            match r.read_event().unwrap() {
                Event::Start(s) if s.name().as_ref() == b"THE-VALUE" => break s.into_owned(),
                Event::Eof => panic!("no THE-VALUE start"),
                _ => continue,
            }
        };
        assert_eq!(r.capture_inner_raw(&start).unwrap(), "hello <b>world</b>");
    }

    #[test]
    fn capture_inner_raw_preserves_surrounding_whitespace() {
        // Mimics a `<DEFAULT-VALUE>` block as it appears in a real ReqIF file:
        // child element on its own indented line, with trailing whitespace before close.
        let xml = b"<OUTER>\n              <DEFAULT-VALUE>\n                <ATTRIBUTE-VALUE-STRING THE-VALUE=\"TBD\"/>\n              </DEFAULT-VALUE>\n            </OUTER>";
        let mut r = ReqIfReader::new(xml);
        let start = loop {
            match r.read_event().unwrap() {
                Event::Start(s) if s.name().as_ref() == b"DEFAULT-VALUE" => break s.into_owned(),
                Event::Eof => panic!("no DEFAULT-VALUE start"),
                _ => continue,
            }
        };
        assert_eq!(
            r.capture_inner_raw(&start).unwrap(),
            "\n                <ATTRIBUTE-VALUE-STRING THE-VALUE=\"TBD\"/>\n              "
        );
    }

    #[test]
    fn capture_inner_raw_handles_nested_same_name() {
        // Sanity: when an inner element shares the close-tag name, depth tracking
        // must skip over it without terminating early.
        let xml = b"<W><W>inner</W></W>";
        let mut r = ReqIfReader::new(xml);
        // Consume outer <W>.
        let start = match r.read_event().unwrap() {
            Event::Start(s) if s.name().as_ref() == b"W" => s.into_owned(),
            _ => unreachable!(),
        };
        assert_eq!(r.capture_inner_raw(&start).unwrap(), "<W>inner</W>");
    }

    #[test]
    fn buffer_position_advances_with_reads() {
        let xml = b"<A/><B/>";
        let mut r = ReqIfReader::new(xml);
        let _ = r.read_event().unwrap();
        let p1 = r.buffer_position();
        let _ = r.read_event().unwrap();
        let p2 = r.buffer_position();
        assert!(
            p2 > p1,
            "buffer_position should advance, got p1={p1} p2={p2}"
        );
    }
}
