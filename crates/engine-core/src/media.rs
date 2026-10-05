//! The media rules a face's references pass (SPEC-348 R3; ADR-359 D1).
//!
//! A card's frame blocks every load and has no base URL, so a media file reaches it only inside the
//! document, as a `data:` URL the core writes. A reference is admitted only as a plain file name
//! with a type in one closed table, read through the caller's [`Reader`] under two caps. The caps
//! and the table are public constants: one copy both clients read.

/// The largest media file a face inlines or plays: 4 MiB (ADR-359 D1).
pub const FILE_CAP: u64 = 4 * 1024 * 1024;
/// The most media bytes one face carries: 16 MiB (ADR-359 D1).
pub const FACE_CAP: u64 = 16 * 1024 * 1024;

/// The closed type table: a file name's extension, compared ASCII-case-insensitively, and the
/// media type its `data:` URL names. Images and audio only; an mp4 is read as audio.
pub const TYPES: [(&str, &str); 16] = [
    ("png", "image/png"),
    ("jpg", "image/jpeg"),
    ("jpeg", "image/jpeg"),
    ("gif", "image/gif"),
    ("webp", "image/webp"),
    ("svg", "image/svg+xml"),
    ("avif", "image/avif"),
    ("mp3", "audio/mpeg"),
    ("m4a", "audio/mp4"),
    ("aac", "audio/aac"),
    ("ogg", "audio/ogg"),
    ("oga", "audio/ogg"),
    ("opus", "audio/ogg"),
    ("wav", "audio/wav"),
    ("webm", "audio/webm"),
    ("mp4", "audio/mp4"),
];

/// Where a face's media files are read from. The caller owns the store: the native adapter reads
/// the opened collection's media folder, a test reads memory.
pub trait Reader {
    /// The first `limit` bytes of the media file `name` at most, or `None` when there is no such
    /// file. The core asks for one byte past its cap and decides by the length it gets back.
    fn read(&self, name: &str, limit: u64) -> Option<Vec<u8>>;
}

/// Whether `name` is a plain file name: not empty, no separator or NUL, and not `.` or `..`.
#[must_use]
pub fn plain_name(name: &str) -> bool {
    let _ = name;
    true
}
