//! XHTML whitespace reflow helpers.
//!
//! ReqIF XHTML attribute values appear inside `<THE-VALUE>` blocks, typically
//! indented to 16 spaces. The parser captures the inner content verbatim
//! (preserving source indentation for byte-exact round-trip under
//! [`crate::unparse::FormatMode::Passthrough`]); these helpers re-canonicalize
//! the indentation when emitting under [`crate::unparse::FormatMode::Canonical`].
//!
//! Mirrors the Python `reqif.helpers.string.xhtml_indent` module.

/// Strip the canonical 16-space indent and outer whitespace from an XHTML body.
///
/// This is the inverse of [`indent_16`]: parser-side normalization that
/// removes the source's formatting decisions. Only *runs* of exactly 16
/// spaces are stripped (matching the Python `r" {16}"` regex semantics); any
/// remaining leading whitespace below that 16-space threshold is preserved.
pub fn unindent_16(s: &str) -> String {
    s.replace("                ", "").trim().to_string()
}

/// Re-indent an XHTML body with a 16-space margin (canonical Python output).
///
/// The body is trimmed, every existing newline is re-indented to 16 spaces,
/// and a leading newline plus 16-space margin is prepended so the first line
/// aligns under `<THE-VALUE>`. A trailing newline plus 14-space margin lets
/// the closing `</THE-VALUE>` sit at column 14.
pub fn indent_16(s: &str) -> String {
    let body = s.trim();
    // Each existing newline becomes "\n" + 16 spaces; prepend the same so
    // the first line aligns under <THE-VALUE>.
    let padded = body.replace('\n', "\n                ");
    format!("\n                {padded}\n              ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn unindent_strips_16_space_runs_and_trims() {
        let input = "                <p>Hello</p>";
        assert_eq!(unindent_16(input), "<p>Hello</p>");
    }

    #[test]
    fn indent_wraps_single_line_with_16_space_margin() {
        let input = "<p>Hello</p>";
        let out = indent_16(input);
        assert_eq!(out, "\n                <p>Hello</p>\n              ");
    }

    #[test]
    fn round_trip_through_unindent_then_indent_normalizes() {
        let weirdly_indented = "          <p>Hello</p>\n              <p>World</p>";
        let canonical = indent_16(&unindent_16(weirdly_indented));
        assert_eq!(
            canonical,
            "\n                <p>Hello</p>\n                              <p>World</p>\n              "
        );
        // Note: lines that had less than 16 spaces of indent keep their
        // remaining whitespace post-unindent. The Python helper has the same
        // characteristic — it strips *runs* of 16, not arbitrary leading space.
    }

    #[test]
    fn already_canonical_body_round_trips_to_same_canonical_form() {
        // The shape produced for a single-line <p> body when emitted by the
        // unparser; feeding it back through unindent_16 + indent_16 should
        // yield the identical string.
        let canonical = "\n                <xhtml:p>aligned</xhtml:p>\n              ";
        let reflowed = indent_16(&unindent_16(canonical));
        assert_eq!(reflowed, canonical);
    }

    #[test]
    fn unindent_then_indent_drops_outer_blank_lines() {
        // Multiple blank lines around an inner body collapse on trim().
        let input = "\n\n                <xhtml:p>x</xhtml:p>\n\n              ";
        let out = indent_16(&unindent_16(input));
        assert_eq!(
            out,
            "\n                <xhtml:p>x</xhtml:p>\n              "
        );
    }
}
