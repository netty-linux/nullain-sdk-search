use std::path::Path;

use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::{Schema, TextFieldIndexing, TextOptions, Value, FAST, STORED, STRING};
use tantivy::{doc, Index, IndexReader, IndexWriter, ReloadPolicy, TantivyDocument, Term};

use crate::document::Document;
use crate::error::SearchError;
use crate::query::SearchResult;

/// Bytes given to the tantivy `IndexWriter` heap. Small on purpose: v0.1
/// targets a single workspace's worth of text files, not large corpora.
const WRITER_HEAP_BYTES: usize = 50_000_000;

const SNIPPET_MAX_CHARS: usize = 200;

/// An in-memory, single-process full-text index over `(source, content)`
/// documents, ranked with tantivy's default BM25 scoring.
///
/// v0.1 scope: no persistence, no incremental/watch indexing, no vector
/// search — see the crate README for what's deliberately left out.
pub struct SearchIndex {
    index: Index,
    writer: IndexWriter,
    reader: IndexReader,
    source_field: tantivy::schema::Field,
    content_field: tantivy::schema::Field,
}

impl SearchIndex {
    /// Build a fresh, empty in-memory index.
    pub fn create_in_ram() -> Result<Self, SearchError> {
        let mut schema_builder = Schema::builder();
        let source_field = schema_builder.add_text_field("source", STRING | STORED | FAST);
        let content_indexing = TextFieldIndexing::default()
            .set_tokenizer("default")
            .set_index_option(tantivy::schema::IndexRecordOption::WithFreqsAndPositions);
        let content_options = TextOptions::default()
            .set_indexing_options(content_indexing)
            .set_stored();
        let content_field = schema_builder.add_text_field("content", content_options);
        let schema = schema_builder.build();

        let index = Index::create_in_ram(schema);
        let writer = index.writer(WRITER_HEAP_BYTES)?;
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::Manual)
            .try_into()?;

        Ok(Self {
            index,
            writer,
            reader,
            source_field,
            content_field,
        })
    }

    /// Ingest one document and make it immediately searchable.
    ///
    /// Re-indexing the same `source` replaces the previous document under
    /// it (delete-then-add), so callers can safely call this again for
    /// content that changed.
    pub fn add_document(&mut self, doc: Document) -> Result<(), SearchError> {
        let term = Term::from_field_text(self.source_field, &doc.source);
        self.writer.delete_term(term);
        self.writer.add_document(doc!(
            self.source_field => doc.source,
            self.content_field => doc.content,
        ))?;
        self.commit()
    }

    /// Recursively walk `dir`, indexing every readable UTF-8 text file
    /// found. Non-UTF-8 (binary) files and unreadable entries are skipped
    /// rather than treated as errors — this is a best-effort convenience
    /// scan, not a strict ingest. Returns the number of files indexed.
    pub fn add_directory(&mut self, dir: &Path) -> Result<usize, SearchError> {
        let mut count = 0usize;
        self.walk_and_index(dir, &mut count)?;
        self.commit()?;
        Ok(count)
    }

    fn walk_and_index(&mut self, dir: &Path, count: &mut usize) -> Result<(), SearchError> {
        let entries = std::fs::read_dir(dir).map_err(|source| SearchError::Io {
            path: dir.to_path_buf(),
            source,
        })?;

        for entry in entries {
            let entry = entry.map_err(|source| SearchError::Io {
                path: dir.to_path_buf(),
                source,
            })?;
            let path = entry.path();
            let file_type = entry.file_type().map_err(|source| SearchError::Io {
                path: path.clone(),
                source,
            })?;

            if file_type.is_dir() {
                self.walk_and_index(&path, count)?;
                continue;
            }

            if !file_type.is_file() {
                continue;
            }

            let Ok(bytes) = std::fs::read(&path) else {
                continue;
            };
            let Ok(content) = String::from_utf8(bytes) else {
                // Binary or non-UTF8 file: skip, don't error.
                continue;
            };

            let source = path.to_string_lossy().into_owned();
            let term = Term::from_field_text(self.source_field, &source);
            self.writer.delete_term(term);
            self.writer.add_document(doc!(
                self.source_field => source,
                self.content_field => content,
            ))?;
            *count += 1;
        }

        Ok(())
    }

    /// Flush pending writes and make them visible to `search`/`fetch`.
    pub fn commit(&mut self) -> Result<(), SearchError> {
        self.writer.commit()?;
        self.reader.reload()?;
        Ok(())
    }

    /// BM25 search over indexed content, returning up to `limit` results
    /// ordered by descending score.
    ///
    /// An empty or whitespace-only `text` is a caller error
    /// ([`SearchError::EmptyQuery`]); a well-formed query with zero
    /// matches returns `Ok(vec![])`, never an error.
    pub fn search(&self, text: &str, limit: usize) -> Result<Vec<SearchResult>, SearchError> {
        if text.trim().is_empty() {
            return Err(SearchError::EmptyQuery);
        }

        let searcher = self.reader.searcher();
        let query_parser = QueryParser::for_index(&self.index, vec![self.content_field]);
        let query = query_parser.parse_query(text)?;

        let top_docs = searcher.search(&query, &TopDocs::with_limit(limit))?;

        let mut snippet_generator =
            tantivy::snippet::SnippetGenerator::create(&searcher, &query, self.content_field)?;
        snippet_generator.set_max_num_chars(SNIPPET_MAX_CHARS);

        let mut results = Vec::with_capacity(top_docs.len());
        for (score, doc_address) in top_docs {
            let retrieved: TantivyDocument = searcher.doc(doc_address)?;
            let source = retrieved
                .get_first(self.source_field)
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let content_value = retrieved
                .get_first(self.content_field)
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            let snippet = snippet_generator.snippet_from_doc(&retrieved);
            let snippet_text = if snippet.is_empty() {
                content_value.chars().take(SNIPPET_MAX_CHARS).collect()
            } else {
                snippet.fragment().to_string()
            };

            results.push(SearchResult {
                source,
                snippet: snippet_text,
                score,
            });
        }

        Ok(results)
    }

    /// Return the full stored content previously indexed under `source`.
    pub fn fetch(&self, source: &str) -> Result<String, SearchError> {
        let searcher = self.reader.searcher();
        let term = Term::from_field_text(self.source_field, source);
        let term_query =
            tantivy::query::TermQuery::new(term, tantivy::schema::IndexRecordOption::Basic);

        let top_docs = searcher.search(&term_query, &TopDocs::with_limit(1))?;
        let Some((_, doc_address)) = top_docs.into_iter().next() else {
            return Err(SearchError::SourceNotFound(source.to_string()));
        };

        let retrieved: TantivyDocument = searcher.doc(doc_address)?;
        let content = retrieved
            .get_first(self.content_field)
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();

        Ok(content)
    }
}
