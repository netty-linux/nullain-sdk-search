//! Pure-Rust full-text local search engine, tantivy-backed.
//!
//! Zero knowledge of Python — see `nullain-search-py` for the PyO3
//! bindings consumed by `nullain-agent-sdk`.

mod document;
mod error;
mod index;
mod query;

pub use document::Document;
pub use error::SearchError;
pub use index::SearchIndex;
pub use query::SearchResult;
