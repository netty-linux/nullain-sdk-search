# nullain-sdk-search

Local full-text search engine for `nullain-agent-sdk`: a pure-Rust core
(`nullain-search-core`, tantivy-backed BM25 search) plus thin PyO3
bindings (`nullain-search-py`, importable as `nullain_search`).

v0.1 scope: index local text files, BM25 search, fetch by source. No web
search, no vector/semantic search, no file watching or incremental
indexing — see `nullain-search-core/src/index.rs` for what's covered.

This repo does **not** implement `nullain.ports.search.SearchProvider`
itself — that async Protocol lives in the sibling `nullain-agent-sdk` repo.
A future `RustSearchAdapter` there wraps this package's synchronous calls
in `asyncio.to_thread`.

## Build locally

Requires Rust (stable) and Python 3.12+.

```sh
cd nullain-search-py
pip install maturin
maturin develop   # builds the extension and installs it into your active venv
```

Then in Python:

```python
import nullain_search

index = nullain_search.SearchIndex()
index.index("the quick brown fox", source="a.txt")
index.index_directory("./some/workspace")  # convenience: recursive text-file scan

hits = index.query("fox", limit=5)
for hit in hits:
    print(hit.source, hit.score, hit.snippet)

print(index.fetch("a.txt"))
```

## Tests

```sh
cargo test --workspace       # core + binding crate unit/integration tests
cargo clippy --workspace -- -D warnings
cargo fmt --check
```

## Layout

- `nullain-search-core/` — pure Rust, zero Python dependency. The BM25
  indexing/search engine.
- `nullain-search-py/` — PyO3 bindings, published as the `nullain-search`
  wheel (`abi3-py312`, one wheel per platform regardless of Python 3.12+
  patch version).
