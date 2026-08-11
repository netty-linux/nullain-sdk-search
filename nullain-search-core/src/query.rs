/// One ranked search hit.
#[derive(Debug, Clone, PartialEq)]
pub struct SearchResult {
    /// The document's stable source identifier.
    pub source: String,
    /// A short excerpt of the matching content, with match highlighting
    /// markers stripped (plain text).
    pub snippet: String,
    /// BM25 relevance score (higher is more relevant).
    pub score: f32,
}
