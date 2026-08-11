use nullain_search_core::{Document, SearchError, SearchIndex};

#[test]
fn add_document_and_search_round_trip() {
    let mut index = SearchIndex::create_in_ram().expect("create index");
    index
        .add_document(Document::new("a.txt", "the quick brown fox jumps"))
        .expect("index a.txt");
    index
        .add_document(Document::new("b.txt", "an unrelated document about cats"))
        .expect("index b.txt");

    let results = index.search("fox", 5).expect("search");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].source, "a.txt");
    assert!(results[0].score > 0.0);
    assert!(results[0].snippet.to_lowercase().contains("fox"));
}

#[test]
fn search_on_empty_index_returns_no_results() {
    let index = SearchIndex::create_in_ram().expect("create index");
    let results = index.search("anything", 5).expect("search");
    assert!(results.is_empty());
}

#[test]
fn empty_query_is_rejected() {
    let index = SearchIndex::create_in_ram().expect("create index");
    let err = index.search("", 5).unwrap_err();
    assert!(matches!(err, SearchError::EmptyQuery));
}

#[test]
fn whitespace_only_query_is_rejected() {
    let index = SearchIndex::create_in_ram().expect("create index");
    let err = index.search("   ", 5).unwrap_err();
    assert!(matches!(err, SearchError::EmptyQuery));
}

#[test]
fn fetch_missing_source_errors() {
    let index = SearchIndex::create_in_ram().expect("create index");
    let err = index.fetch("nope.txt").unwrap_err();
    assert!(matches!(err, SearchError::SourceNotFound(source) if source == "nope.txt"));
}

#[test]
fn fetch_returns_full_indexed_content() {
    let mut index = SearchIndex::create_in_ram().expect("create index");
    index
        .add_document(Document::new("a.txt", "full content goes here"))
        .expect("index a.txt");

    let content = index.fetch("a.txt").expect("fetch");
    assert_eq!(content, "full content goes here");
}

#[test]
fn add_directory_on_empty_dir_indexes_nothing() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut index = SearchIndex::create_in_ram().expect("create index");

    let count = index.add_directory(dir.path()).expect("add_directory");
    assert_eq!(count, 0);

    let results = index.search("anything", 5).expect("search");
    assert!(results.is_empty());
}

#[test]
fn add_directory_skips_binary_files_but_indexes_text_files() {
    let dir = tempfile::tempdir().expect("tempdir");

    std::fs::write(dir.path().join("notes.txt"), "hello searchable world").expect("write txt");
    std::fs::write(
        dir.path().join("image.bin"),
        [0xFFu8, 0x00, 0xFE, 0x01, 0x00],
    )
    .expect("write binary");

    let mut index = SearchIndex::create_in_ram().expect("create index");
    let count = index.add_directory(dir.path()).expect("add_directory");

    assert_eq!(count, 1, "only the text file should be indexed");

    let results = index.search("searchable", 5).expect("search");
    assert_eq!(results.len(), 1);
    assert!(results[0].source.ends_with("notes.txt"));
}

#[test]
fn add_directory_recurses_into_subdirectories() {
    let dir = tempfile::tempdir().expect("tempdir");
    let sub = dir.path().join("nested");
    std::fs::create_dir(&sub).expect("mkdir nested");
    std::fs::write(sub.join("deep.txt"), "buried treasure content").expect("write nested file");

    let mut index = SearchIndex::create_in_ram().expect("create index");
    let count = index.add_directory(dir.path()).expect("add_directory");
    assert_eq!(count, 1);

    let results = index.search("treasure", 5).expect("search");
    assert_eq!(results.len(), 1);
}

#[test]
fn reindexing_same_source_replaces_previous_content() {
    let mut index = SearchIndex::create_in_ram().expect("create index");
    index
        .add_document(Document::new("a.txt", "original content"))
        .expect("index original");
    index
        .add_document(Document::new("a.txt", "updated content"))
        .expect("reindex");

    let content = index.fetch("a.txt").expect("fetch");
    assert_eq!(content, "updated content");

    let results = index.search("original", 5).expect("search");
    assert!(results.is_empty(), "stale content must not be searchable");
}
