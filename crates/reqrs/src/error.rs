use std::fmt;

#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
pub enum ReqIfError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("XML parse error at byte {pos}: {msg}")]
    Xml { pos: usize, msg: String },

    #[error("malformed datetime {value:?}")]
    Datetime { value: String },

    #[error("unexpected tag <{tag}> inside <{parent}>")]
    UnexpectedTag { tag: String, parent: String },

    #[error("missing required attribute {attr} on <{tag}>")]
    MissingAttribute { tag: String, attr: String },

    #[error("missing required child <{child}> inside <{parent}>")]
    MissingChild { child: String, parent: String },

    #[error("zip error: {0}")]
    Zip(#[from] zip::result::ZipError),

    #[error("schema error: {0}")]
    Schema(String),
}

/// Stable identifier for an [`IssueKind`].
///
/// Codes are part of the public API and are stable across minor releases
/// once published. Toolbuilders can match on `kind` for programmatic logic
/// and reference the code in user-facing output for cross-tool linking.
///
/// Code format: a two-letter family (`RQ` for reqrs parser-emitted issues)
/// followed by a zero-padded sequence number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IssueId(&'static str);

impl IssueId {
    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

impl fmt::Display for IssueId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

/// Domain locator for an [`Issue`].
///
/// Locators are domain-shaped rather than purely byte-shaped. Parser-emitted
/// issues use [`Location::Xml`]; semantic-check issues (duplicate IDs,
/// dangling refs) will use [`Location::Need`]; structural-traversal issues
/// (hierarchy comparison, diff) will use [`Location::Hierarchy`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Location {
    /// A byte position in the source XML document.
    Xml { byte_offset: u64 },

    /// A specific need (spec object, spec type, data type, …) by its
    /// IDENTIFIER, optionally narrowed to a named field within the need.
    Need {
        id: String,
        field: Option<String>,
    },

    /// A path through the specification hierarchy, expressed as a sequence
    /// of node identifiers from the root specification down to the target.
    Hierarchy { path: Vec<String> },
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Location::Xml { byte_offset } => write!(f, "byte {byte_offset}"),
            Location::Need { id, field: None } => write!(f, "need {id:?}"),
            Location::Need {
                id,
                field: Some(field),
            } => write!(f, "need {id:?} field {field:?}"),
            Location::Hierarchy { path } => write!(f, "hierarchy {}", path.join("/")),
        }
    }
}

/// Classified kind of an [`Issue`]. Non-exhaustive so new kinds can be
/// added in minor releases without breaking downstream consumers.
///
/// Each variant has a stable [`IssueId`] code accessible via
/// [`IssueKind::code`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum IssueKind {
    /// An unrecognised element appeared as a child of a known element. The
    /// parser skips the subtree (recording this issue) so that vendor
    /// extensions and forward-compatible documents still round-trip.
    UnknownElement { tag: String, parent: String },

    /// An element expected to wrap children was found self-closed (and the
    /// element form is not a documented legal short-form). Currently emitted
    /// for `<THE-HEADER/>` (Python-parity edge case); reserved for future
    /// parse-time "required child missing" issues.
    ExpectedNonEmptyElement { tag: String, parent: String },
}

impl IssueKind {
    /// Stable [`IssueId`] for this kind. Part of the public API.
    pub fn code(&self) -> IssueId {
        match self {
            IssueKind::UnknownElement { .. } => IssueId("RQ001"),
            IssueKind::ExpectedNonEmptyElement { .. } => IssueId("RQ002"),
        }
    }
}

impl fmt::Display for IssueKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IssueKind::UnknownElement { tag, parent } => {
                write!(f, "unknown element <{tag}> inside <{parent}>")
            }
            IssueKind::ExpectedNonEmptyElement { tag, parent } => {
                write!(
                    f,
                    "<{tag}/> is self-closed; expected open element inside <{parent}>"
                )
            }
        }
    }
}

/// A non-fatal issue surfaced by the parser, validator, or other library
/// operation. Aggregated on [`crate::model::ReqIfBundle::exceptions`] by
/// the parser; returned by [`crate::commands::validate`] as a `Vec<Issue>`
/// (in the redesigned library API).
///
/// Issues carry a structured [`IssueKind`] for programmatic matching, an
/// optional [`Location`] for tools that want to surface the position back
/// to the user (LSP, CI annotators), and an optional free-form `context`
/// string for additional narrative ("while parsing REQ-IF root").
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Issue {
    pub kind: IssueKind,
    pub location: Option<Location>,
    pub context: Option<String>,
}

impl Issue {
    /// Construct an [`Issue`] from a [`IssueKind`]. Location and context
    /// default to `None`; use [`Issue::with_location`] and
    /// [`Issue::with_context`] to attach them.
    pub fn new(kind: IssueKind) -> Self {
        Self {
            kind,
            location: None,
            context: None,
        }
    }

    pub fn with_location(mut self, location: Location) -> Self {
        self.location = Some(location);
        self
    }

    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }

    /// Convenience accessor for the stable code of this issue's kind.
    pub fn code(&self) -> IssueId {
        self.kind.code()
    }
}

impl fmt::Display for Issue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kind)?;
        if let Some(ctx) = &self.context {
            write!(f, " (while {ctx})")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issue_display_without_context_matches_kind() {
        let i = Issue::new(IssueKind::UnknownElement {
            tag: "FOO".into(),
            parent: "REQ-IF".into(),
        });
        assert_eq!(format!("{i}"), "unknown element <FOO> inside <REQ-IF>");
    }

    #[test]
    fn issue_display_with_context_appends_while_clause() {
        let i = Issue::new(IssueKind::UnknownElement {
            tag: "FOO".into(),
            parent: "REQ-IF".into(),
        })
        .with_context("parsing REQ-IF root");
        assert_eq!(
            format!("{i}"),
            "unknown element <FOO> inside <REQ-IF> (while parsing REQ-IF root)"
        );
    }

    #[test]
    fn expected_non_empty_element_display_names_parent() {
        let k = IssueKind::ExpectedNonEmptyElement {
            tag: "THE-HEADER".into(),
            parent: "REQ-IF".into(),
        };
        assert_eq!(
            format!("{k}"),
            "<THE-HEADER/> is self-closed; expected open element inside <REQ-IF>"
        );
    }

    #[test]
    fn stable_codes_match_documented_assignments() {
        assert_eq!(
            IssueKind::UnknownElement {
                tag: "X".into(),
                parent: "Y".into()
            }
            .code()
            .as_str(),
            "RQ001"
        );
        assert_eq!(
            IssueKind::ExpectedNonEmptyElement {
                tag: "X".into(),
                parent: "Y".into()
            }
            .code()
            .as_str(),
            "RQ002"
        );
    }

    #[test]
    fn issue_code_delegates_to_kind() {
        let i = Issue::new(IssueKind::UnknownElement {
            tag: "FOO".into(),
            parent: "BAR".into(),
        });
        assert_eq!(i.code().as_str(), "RQ001");
    }

    #[test]
    fn location_xml_display_uses_byte_keyword() {
        let l = Location::Xml { byte_offset: 1234 };
        assert_eq!(format!("{l}"), "byte 1234");
    }

    #[test]
    fn location_need_display_with_field() {
        let l = Location::Need {
            id: "REQ-042".into(),
            field: Some("long_name".into()),
        };
        assert_eq!(format!("{l}"), r#"need "REQ-042" field "long_name""#);
    }

    #[test]
    fn location_need_display_without_field() {
        let l = Location::Need {
            id: "REQ-042".into(),
            field: None,
        };
        assert_eq!(format!("{l}"), r#"need "REQ-042""#);
    }

    #[test]
    fn location_hierarchy_display_joins_with_slash() {
        let l = Location::Hierarchy {
            path: vec!["SPEC-1".into(), "GRP-A".into(), "REQ-42".into()],
        };
        assert_eq!(format!("{l}"), "hierarchy SPEC-1/GRP-A/REQ-42");
    }

    #[test]
    fn with_location_attaches_locator() {
        let i = Issue::new(IssueKind::UnknownElement {
            tag: "X".into(),
            parent: "Y".into(),
        })
        .with_location(Location::Xml { byte_offset: 99 });
        assert_eq!(i.location, Some(Location::Xml { byte_offset: 99 }));
    }

    #[test]
    fn reqif_error_display_for_missing_attribute() {
        let e = ReqIfError::MissingAttribute {
            tag: "SPEC-OBJECT".into(),
            attr: "IDENTIFIER".into(),
        };
        assert_eq!(
            e.to_string(),
            "missing required attribute IDENTIFIER on <SPEC-OBJECT>"
        );
    }
}
