// pyo3's #[pymethods]/#[pyfunction] macro expansion triggers a
// clippy::useless_conversion false positive on every PyResult-returning
// method's generated trampoline (PyO3/pyo3#1813) — allowed crate-wide.
#![allow(clippy::useless_conversion)]

use nullain_search_core::{Document, SearchError, SearchIndex};
use pyo3::exceptions::{PyIOError, PyRuntimeError, PyValueError};
use pyo3::prelude::*;

struct SearchErrorWrapper(SearchError);

impl From<SearchErrorWrapper> for PyErr {
    fn from(wrapper: SearchErrorWrapper) -> Self {
        let err = wrapper.0;
        match err {
            SearchError::EmptyQuery => PyValueError::new_err(err.to_string()),
            SearchError::SourceNotFound(_) => PyValueError::new_err(err.to_string()),
            SearchError::Io { .. } => PyIOError::new_err(err.to_string()),
            SearchError::Index(_) | SearchError::QueryParse(_) => {
                PyRuntimeError::new_err(err.to_string())
            }
        }
    }
}

/// A single ranked search hit: source identifier, snippet, and BM25 score.
#[pyclass(name = "SearchHit", get_all)]
struct PySearchHit {
    source: String,
    snippet: String,
    score: f32,
}

#[pymethods]
impl PySearchHit {
    fn __repr__(&self) -> String {
        format!(
            "SearchHit(source={:?}, snippet={:?}, score={})",
            self.source, self.snippet, self.score
        )
    }
}

/// In-memory, single-process full-text index over local text content.
///
/// This class is a thin, synchronous wrapper around
/// `nullain-search-core::SearchIndex`. It intentionally does not implement
/// `nullain.ports.search.SearchProvider` itself (that Protocol is `async`
/// and lives in the sibling `nullain-agent-sdk` repo); the adapter there
/// wraps these blocking calls in `asyncio.to_thread`.
#[pyclass(name = "SearchIndex")]
struct PySearchIndex(SearchIndex);

#[pymethods]
impl PySearchIndex {
    #[new]
    fn new() -> PyResult<Self> {
        Ok(Self(
            SearchIndex::create_in_ram().map_err(SearchErrorWrapper)?,
        ))
    }

    /// index(content: str, *, source: str) -> None
    ///
    /// Ingest one document. Mirrors `SearchProvider.index`'s signature.
    #[pyo3(signature = (content, *, source))]
    fn index(&mut self, content: &str, source: &str) -> PyResult<()> {
        self.0
            .add_document(Document::new(source, content))
            .map_err(SearchErrorWrapper)?;
        Ok(())
    }

    /// index_directory(path: str) -> int
    ///
    /// Convenience beyond the Protocol: recursively index every readable
    /// UTF-8 text file under `path`. Binary/non-UTF8 files are skipped.
    /// Returns the number of files indexed.
    fn index_directory(&mut self, path: &str) -> PyResult<usize> {
        let count = self
            .0
            .add_directory(std::path::Path::new(path))
            .map_err(SearchErrorWrapper)?;
        Ok(count)
    }

    /// query(text: str, limit: int = 5) -> list[SearchHit]
    ///
    /// BM25 search. Raises ValueError for an empty/whitespace query;
    /// returns an empty list (never raises) for zero matches.
    #[pyo3(signature = (text, limit=5))]
    fn query(&self, text: &str, limit: usize) -> PyResult<Vec<PySearchHit>> {
        let results = self.0.search(text, limit).map_err(SearchErrorWrapper)?;
        Ok(results
            .into_iter()
            .map(|r| PySearchHit {
                source: r.source,
                snippet: r.snippet,
                score: r.score,
            })
            .collect())
    }

    /// fetch(source: str) -> str
    ///
    /// Return the full content previously indexed under `source`. Raises
    /// ValueError if `source` was never indexed.
    fn fetch(&self, source: &str) -> PyResult<String> {
        let content = self.0.fetch(source).map_err(SearchErrorWrapper)?;
        Ok(content)
    }
}

#[pymodule]
fn nullain_search(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PySearchIndex>()?;
    m.add_class::<PySearchHit>()?;
    Ok(())
}
