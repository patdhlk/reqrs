use chrono::{DateTime, FixedOffset};

use crate::error::ReqIfError;
use crate::helpers::datetime;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReqIfHeader {
    pub identifier: String,
    pub comment: Option<String>,
    pub creation_time: Option<String>,
    pub repository_id: Option<RepositoryId>,
    pub req_if_tool_id: Option<String>,
    pub req_if_version: Option<String>,
    pub source_tool_id: Option<String>,
    pub title: Option<String>,
}

impl ReqIfHeader {
    /// Lazily parse `creation_time` as a typed [`DateTime`].
    ///
    /// Returns `None` when the source had no `<CREATION-TIME>` element.
    /// Returns `Some(Err(_))` when the field is present but malformed; the
    /// raw string is preserved in [`Self::creation_time`] regardless so
    /// round-trip fidelity is unaffected.
    pub fn creation_time_parsed(&self) -> Option<Result<DateTime<FixedOffset>, ReqIfError>> {
        self.creation_time.as_deref().map(datetime::parse)
    }
}

/// `<REPOSITORY-ID>` may be present as a text value or as a self-closed empty tag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepositoryId {
    Text(String),
    Empty,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header_with_creation_time(s: Option<&str>) -> ReqIfHeader {
        ReqIfHeader {
            identifier: "H-1".into(),
            comment: None,
            creation_time: s.map(str::to_string),
            repository_id: None,
            req_if_tool_id: None,
            req_if_version: None,
            source_tool_id: None,
            title: None,
        }
    }

    #[test]
    fn creation_time_parsed_returns_none_when_absent() {
        let h = header_with_creation_time(None);
        assert!(h.creation_time_parsed().is_none());
    }

    #[test]
    fn creation_time_parsed_returns_typed_datetime_on_well_formed_input() {
        let h = header_with_creation_time(Some("2021-07-01T01:12:06.749Z"));
        let parsed = h.creation_time_parsed().unwrap().unwrap();
        assert_eq!(parsed.timestamp_millis(), 1625101926749);
    }

    #[test]
    fn creation_time_parsed_returns_some_err_on_garbage() {
        let h = header_with_creation_time(Some("not a date"));
        let err = h.creation_time_parsed().unwrap().unwrap_err();
        assert!(matches!(err, ReqIfError::Datetime { .. }));
    }

    #[test]
    fn creation_time_parsed_does_not_mutate_raw_field() {
        let h = header_with_creation_time(Some("2021-07-01T01:12:06.749Z"));
        let _ = h.creation_time_parsed();
        assert_eq!(h.creation_time.as_deref(), Some("2021-07-01T01:12:06.749Z"));
    }
}
