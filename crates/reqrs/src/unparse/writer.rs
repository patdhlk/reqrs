use crate::error::ReqIfError;
use std::fmt::Write as _;

/// Format mode for the unparser.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatMode {
    /// Preserve self-closing decisions captured during parse; no whitespace reflow.
    Passthrough,
    /// Canonical indentation; ignores per-element self-closing flags.
    Canonical,
}

/// Append an XML-escaped representation of `s` to `out`.
pub(crate) fn escape_attr(out: &mut String, s: &str) {
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            // Attribute-value normalization (XML 1.0 §3.3.3) folds literal
            // #x9/#xA/#xD to spaces on re-parse, so they only survive as
            // character references — the decimal form lxml emits and real
            // ReqIF exports (IBM Engineering / requisis) carry.
            '\t' => out.push_str("&#9;"),
            '\n' => out.push_str("&#10;"),
            '\r' => out.push_str("&#13;"),
            // XML 1.0 allows literal apostrophes; Python lib uses &apos; only in some
            // helpers — mirror lxml.etree.tostring which leaves them literal.
            _ => out.push(c),
        }
    }
}

/// Append an XML-escaped representation of `s` to `out`, for use inside element text content.
///
/// Per the XML 1.0 spec, only `&` and `<` are required to be escaped in element
/// text content; `>` is legal raw except in the `]]>` sequence inside CDATA,
/// which we never emit. We deliberately leave `>` unescaped here so round-trip
/// preserves the form most ReqIF tools (Polarion, Doors, ReqIF Studio, the
/// Eclipse RMF reference) emit — a literal `>` in titles like `"OEM -> SUP"`.
///
/// Quotes (`"` and `'`) are also legal raw inside text content; attribute
/// quoting is handled by [`escape_attr`].
pub(crate) fn escape_text(out: &mut String, s: &str) {
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            // End-of-line handling (XML 1.0 §2.11) folds a literal #xD to
            // #xA on re-parse; only the character-reference form survives.
            '\r' => out.push_str("&#13;"),
            _ => out.push(c),
        }
    }
}

/// Write a self-closing element with attributes sorted alphabetically by name.
pub(crate) fn write_self_closing(
    out: &mut String,
    indent: &str,
    tag: &str,
    attrs: &mut [(&str, String)],
) -> Result<(), ReqIfError> {
    attrs.sort_by(|a, b| a.0.cmp(b.0));
    write!(out, "{indent}<{tag}").map_err(|e| ReqIfError::Schema(e.to_string()))?;
    for (k, v) in attrs.iter() {
        out.push(' ');
        out.push_str(k);
        out.push_str("=\"");
        escape_attr(out, v);
        out.push('"');
    }
    out.push_str("/>\n");
    Ok(())
}

/// Write an open tag with sorted attributes.
pub(crate) fn write_open(
    out: &mut String,
    indent: &str,
    tag: &str,
    attrs: &mut [(&str, String)],
) -> Result<(), ReqIfError> {
    attrs.sort_by(|a, b| a.0.cmp(b.0));
    write!(out, "{indent}<{tag}").map_err(|e| ReqIfError::Schema(e.to_string()))?;
    for (k, v) in attrs.iter() {
        out.push(' ');
        out.push_str(k);
        out.push_str("=\"");
        escape_attr(out, v);
        out.push('"');
    }
    out.push_str(">\n");
    Ok(())
}

/// Write a closing tag.
pub(crate) fn write_close(out: &mut String, indent: &str, tag: &str) {
    out.push_str(indent);
    out.push_str("</");
    out.push_str(tag);
    out.push_str(">\n");
}

/// Emit each captured inline `<!-- ... -->` comment on its own line at
/// `indent`, exactly recreating the source body between the `<!--` and `-->`
/// delimiters. Used by element unparsers that own a `comments_before` slot
/// (SpecType, SpecObject, AttributeValue) so their leading inter-sibling
/// comments survive round-trip.
///
/// No escaping is performed — XML 1.0 forbids `--` inside comments at the
/// parser level, so a value captured by quick-xml's `Event::Comment` is
/// already safe to re-emit verbatim.
pub(crate) fn emit_comments_before(out: &mut String, indent: &str, comments: &[String]) {
    for c in comments {
        out.push_str(indent);
        out.push_str("<!--");
        out.push_str(c);
        out.push_str("-->\n");
    }
}

/// Emit a text-content element on one line: `{indent}<TAG>{escaped_text}</TAG>\n`.
/// Emits nothing when `value` is `None`.
pub(crate) fn write_text_element(out: &mut String, indent: &str, tag: &str, value: Option<&str>) {
    let Some(v) = value else { return };
    out.push_str(indent);
    out.push('<');
    out.push_str(tag);
    out.push('>');
    escape_text(out, v);
    out.push_str("</");
    out.push_str(tag);
    out.push_str(">\n");
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn attributes_are_alphabetically_sorted() {
        let mut out = String::new();
        let mut attrs = vec![
            ("LONG-NAME", "Name".to_string()),
            ("IDENTIFIER", "ID-1".to_string()),
            ("DESC", "Hello".to_string()),
        ];
        write_self_closing(&mut out, "    ", "DATATYPE-DEFINITION-STRING", &mut attrs).unwrap();
        assert_eq!(
            out,
            "    <DATATYPE-DEFINITION-STRING DESC=\"Hello\" IDENTIFIER=\"ID-1\" LONG-NAME=\"Name\"/>\n"
        );
    }

    #[test]
    fn special_chars_are_escaped() {
        let mut out = String::new();
        let mut attrs = vec![("DESC", "a < b & \"c\"".to_string())];
        write_self_closing(&mut out, "", "T", &mut attrs).unwrap();
        assert_eq!(out, "<T DESC=\"a &lt; b &amp; &quot;c&quot;\"/>\n");
    }

    #[test]
    fn control_whitespace_in_attributes_is_escaped_as_char_refs() {
        // XML attribute-value normalization (XML 1.0 §3.3.3) turns literal
        // #x9/#xA/#xD in attribute values into spaces on every conforming
        // re-parse. To survive round-trip they must be written as character
        // references — the same form lxml emits and the form real ReqIF
        // exports (IBM Engineering / requisis ReqIF-Manager) use.
        let mut out = String::new();
        escape_attr(&mut out, "line1\nline2\ttab\rcr");
        assert_eq!(out, "line1&#10;line2&#9;tab&#13;cr");
    }

    #[test]
    fn escape_text_writes_carriage_return_as_char_ref() {
        // End-of-line handling (XML 1.0 §2.11) folds a literal #xD in text
        // content into #xA on re-parse; only the &#13; form survives.
        // Literal newlines and tabs in text content are untouched.
        let mut out = String::new();
        escape_text(&mut out, "a\rb\nc\td");
        assert_eq!(out, "a&#13;b\nc\td");
    }

    #[test]
    fn escape_text_handles_amp_and_lt_but_leaves_gt_and_quotes() {
        // Per the XML 1.0 spec, only `&` and `<` MUST be escaped in element
        // text content. We deliberately leave `>` as a literal to match the
        // form that real ReqIF tools (Polarion, Doors, ReqIF Studio, Eclipse
        // RMF) emit in titles like "OEM -> SUP".
        let mut out = String::new();
        escape_text(&mut out, "a & b < c > d \"e\" 'f'");
        assert_eq!(out, "a &amp; b &lt; c > d \"e\" 'f'");
    }

    #[test]
    fn write_open_and_close_pair_emits_balanced_tags() {
        let mut out = String::new();
        let mut attrs = vec![("IDENTIFIER", "ID-1".to_string())];
        write_open(&mut out, "  ", "TAG", &mut attrs).unwrap();
        write_close(&mut out, "  ", "TAG");
        assert_eq!(out, "  <TAG IDENTIFIER=\"ID-1\">\n  </TAG>\n");
    }
}
