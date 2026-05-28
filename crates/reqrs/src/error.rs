use std::fmt;

#[derive(thiserror::Error, Debug)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaWarning {
    pub message: String,
    pub context: Option<String>,
}

impl fmt::Display for SchemaWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.context {
            Some(ctx) => write!(f, "{} (while {})", self.message, ctx),
            None => f.write_str(&self.message),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_warning_display_with_context() {
        let w = SchemaWarning {
            message: "missing TYPE".into(),
            context: Some("parsing SPEC-OBJECT REQ-001".into()),
        };
        assert_eq!(
            format!("{w}"),
            "missing TYPE (while parsing SPEC-OBJECT REQ-001)"
        );
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
