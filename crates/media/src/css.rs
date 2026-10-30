//! The media names a note type's CSS writes (SPEC-513 R5; ADR-531 D5). STUB: every function keeps
//! its shape and answers wrongly, so the crate's tests are seen red first.

/// Every media name `css` writes in a `url(`. STUB: none.
#[must_use]
pub fn scan_css(_css: &str) -> Vec<String> {
    Vec::new()
}

/// `name` with its `%XX` escapes decoded. STUB: none.
#[must_use]
pub fn decoded_name(_name: &str) -> Option<String> {
    None
}
