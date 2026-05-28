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

/// `<REPOSITORY-ID>` may be present as a text value or as a self-closed empty tag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepositoryId {
    Text(String),
    Empty,
}
