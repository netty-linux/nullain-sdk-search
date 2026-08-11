/// A single unit of content to ingest into a [`crate::SearchIndex`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    /// Stable identifier for this document (e.g. a file path or URL) —
    /// how a later `search`/`fetch` result refers back to it.
    pub source: String,
    /// Raw text content to index and store.
    pub content: String,
}

impl Document {
    pub fn new(source: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            content: content.into(),
        }
    }
}
