use std::path::PathBuf;

/// Errors produced by [`crate::SearchIndex`] operations.
#[derive(Debug, thiserror::Error)]
pub enum SearchError {
    #[error("search query must not be empty")]
    EmptyQuery,

    #[error("no document indexed under source {0:?}")]
    SourceNotFound(String),

    #[error("I/O error at {path:?}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("tantivy index error: {0}")]
    Index(#[from] tantivy::TantivyError),

    #[error("query parse error: {0}")]
    QueryParse(#[from] tantivy::query::QueryParserError),
}
