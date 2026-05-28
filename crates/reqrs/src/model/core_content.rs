//! `<CORE-CONTENT>` element model — single-child wrapper around
//! [`crate::model::ReqIfContent`].
//!
//! Mirrors `strict-doc-reqif/reqif/models/reqif_core_content.py`. The
//! `<CORE-CONTENT>` element is essentially a stylistic indirection in the
//! ReqIF schema: it carries no attributes of its own and exists solely to
//! wrap a single `<REQ-IF-CONTENT>` child.
//!
//! `req_if_content` is `Option<ReqIfContent>` to record whether the source had
//! a `<REQ-IF-CONTENT>` child at all. An empty `<CORE-CONTENT/>` is legal.

use crate::model::ReqIfContent;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CoreContent {
    pub req_if_content: Option<ReqIfContent>,
}
