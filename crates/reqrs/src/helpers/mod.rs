//! Convenience helpers layered on top of the byte-fidelity model.
//!
//! The model intentionally stores certain fields (datetimes, etc.) as raw
//! strings so that round-trip emits the exact original bytes. Code that needs
//! typed values for filtering, comparison, or arithmetic should reach into
//! these helpers rather than touching the raw strings directly.

pub mod datetime;
pub mod xhtml_indent;
