//! The media rules a face's references pass (SPEC-348 R3; ADR-359 D1).
//!
//! A card's frame blocks every load and has no base URL, so a media file reaches it only inside the
//! document, as a `data:` URL the core writes. A reference is admitted only as a plain file name
//! with a type in one closed table, read through the caller's [`Reader`] under two caps. The caps
//! and the table are public constants: one copy both clients read.

use anki::text::replace_media_refs;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;

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

/// The closed font table: a font file's extension, compared ASCII-case-insensitively, and the
/// media type its `data:` URL names. Only the face's CSS pass reads it, and only for a reader that
/// asks (SPEC-393 R3; ADR-407 D2); [`TYPES`] and the text's rewrite never do.
pub const FONT_TYPES: [(&str, &str); 4] = [
    ("ttf", "font/ttf"),
    ("otf", "font/otf"),
    ("woff", "font/woff"),
    ("woff2", "font/woff2"),
];

/// The opening of a CSS `url(` reference, matched ASCII-case-insensitively.
const URL_OPEN: &str = "url(";

/// Where a face's media files are read from. The caller owns the store: the native adapter reads
/// the opened collection's media folder, a test reads memory.
pub trait Reader {
    /// The first `limit` bytes of the media file `name` at most, or `None` when there is no such
    /// file. The core asks for one byte past its cap and decides by the length it gets back.
    fn read(&self, name: &str, limit: u64) -> Option<Vec<u8>>;

    /// Whether the face's CSS has its fonts inlined as `data:` URLs read through this reader. A
    /// reader that does not ask gets the note type's CSS byte for byte, and is asked for no font.
    fn inlines_fonts(&self) -> bool {
        false
    }
}

/// Whether `name` is a plain file name: not empty, no separator or NUL, and not `.` or `..`.
#[must_use]
pub fn plain_name(name: &str) -> bool {
    let separator = name.contains(['/', '\\', '\0']);
    let dots = name == "." || name == "..";
    let empty = name.is_empty();
    !(separator || dots || empty)
}

/// `name` with its `%XX` escapes decoded, as a card writes a media file's name in an attribute. A
/// `%` not followed by two hex digits stands for itself. `None` when the decoded bytes are not
/// UTF-8, which refuses the reference: no file name the engine stores can spell them.
#[must_use]
pub fn decoded_name(name: &str) -> Option<String> {
    let mut decoded = Vec::with_capacity(name.len());
    let mut rest = name.as_bytes();
    while let Some((&byte, tail)) = rest.split_first() {
        if byte == b'%'
            && let [high, low, after @ ..] = tail
            && let (Some(high), Some(low)) = (hex(*high), hex(*low))
        {
            decoded.push(high * 16 + low);
            rest = after;
        } else {
            decoded.push(byte);
            rest = tail;
        }
    }
    String::from_utf8(decoded).ok()
}

/// One hex digit's value.
fn hex(digit: u8) -> Option<u8> {
    char::from(digit)
        .to_digit(16)
        .and_then(|value| u8::try_from(value).ok())
}

/// `argument` without one pair of matching quotes around it, when it has them.
fn unquoted(argument: &str) -> &str {
    ['"', '\'']
        .into_iter()
        .find_map(|quote| argument.strip_prefix(quote)?.strip_suffix(quote))
        .unwrap_or(argument)
}

/// The media type the closed `table` gives `name`'s extension, compared ASCII-case-insensitively.
fn media_type(name: &str, table: &[(&str, &'static str)]) -> Option<&'static str> {
    let (_, extension) = name.rsplit_once('.')?;
    table
        .iter()
        .find(|(known, _)| known.eq_ignore_ascii_case(extension))
        .map(|&(_, media_type)| media_type)
}

/// One face's media: the caller's reader, the bytes the face has taken so far against
/// [`FACE_CAP`], and the names it left out. Every use of a file counts toward the total, and a file
/// is read again at each use, so the bytes held while a face is completed stay within its cap.
pub(crate) struct Budget<'a> {
    reader: &'a dyn Reader,
    total: u64,
    omitted: Vec<String>,
}

impl<'a> Budget<'a> {
    /// A budget with nothing taken, reading through `reader`.
    pub(crate) fn new(reader: &'a dyn Reader) -> Self {
        Self {
            reader,
            total: 0,
            omitted: Vec::new(),
        }
    }

    /// `text` with every media reference the engine finds in it rewritten: to the `data:` URL of
    /// the file's bytes when the rules admit it, else to nothing.
    pub(crate) fn rewrite(&mut self, text: &str) -> String {
        replace_media_refs(text, |name| Some(self.attribute(name)))
            .unwrap_or_else(|| text.to_owned())
    }

    /// The value a media attribute that wrote `written` takes.
    fn attribute(&mut self, written: &str) -> String {
        match self.take(written, &TYPES, "") {
            Some((media_type, bytes)) => {
                format!("data:{media_type};base64,{}", STANDARD.encode(bytes))
            }
            None => String::new(),
        }
    }

    /// The bytes of the sound file `written` names, when the rules admit it and its type is audio.
    pub(crate) fn sound(&mut self, written: &str) -> Option<Vec<u8>> {
        self.take(written, &TYPES, "audio/").map(|(_, bytes)| bytes)
    }

    /// `css` with each `url(` that names a font rewritten (SPEC-393 R4): to the `data:` URL of the
    /// font's bytes when SPEC-348 R3's rules admit it under [`FONT_TYPES`], else to `url("")` with
    /// its written name joining the omitted ones. Every other `url(`, an unclosed one included,
    /// and every byte outside a rewritten one stay as written.
    pub(crate) fn fonts(&mut self, css: &str) -> String {
        let mut written = String::with_capacity(css.len());
        let mut rest = css;
        while let Some(start) = rest
            .as_bytes()
            .windows(URL_OPEN.len())
            .position(|window| window.eq_ignore_ascii_case(URL_OPEN.as_bytes()))
        {
            let open = start + URL_OPEN.len();
            let Some(length) = rest[open..].find(')') else {
                break;
            };
            let close = open + length + 1;
            written.push_str(&rest[..start]);
            match self.font(&rest[open..open + length]) {
                Some(url) => written.push_str(&url),
                None => written.push_str(&rest[start..close]),
            }
            rest = &rest[close..];
        }
        written.push_str(rest);
        written
    }

    /// The `url(...)` a CSS argument naming a font is written as, or `None` when the argument
    /// names no font and stays as written: it must decode to a plain name whose extension is in
    /// [`FONT_TYPES`].
    fn font(&mut self, argument: &str) -> Option<String> {
        let written = unquoted(argument.trim());
        let name = decoded_name(written)?;
        let font = plain_name(&name) && media_type(&name, &FONT_TYPES).is_some();
        if !font {
            return None;
        }
        Some(match self.take(written, &FONT_TYPES, "") {
            Some((media_type, bytes)) => {
                format!(
                    "url(\"data:{media_type};base64,{}\")",
                    STANDARD.encode(bytes)
                )
            }
            None => "url(\"\")".to_owned(),
        })
    }

    /// The names left out, each once, in the order the face first met them.
    pub(crate) fn into_omitted(self) -> Vec<String> {
        self.omitted
    }

    /// The type and bytes of the file `written` names when the rules admit it under `table` and its
    /// type starts with `kind`; else `None`, with `written` joining the omitted names once.
    fn take(
        &mut self,
        written: &str,
        table: &[(&str, &'static str)],
        kind: &str,
    ) -> Option<(&'static str, Vec<u8>)> {
        let admitted = self.admit(written, table, kind);
        if admitted.is_none() && !self.omitted.iter().any(|name| name == written) {
            self.omitted.push(written.to_owned());
        }
        admitted
    }

    /// The rules of SPEC-348 R3, in order: the decoded name is plain, its type is in `table`, the
    /// reader has it, it is within the file cap, and the face stays within its own.
    fn admit(
        &mut self,
        written: &str,
        table: &[(&str, &'static str)],
        kind: &str,
    ) -> Option<(&'static str, Vec<u8>)> {
        let name = decoded_name(written)?;
        if !plain_name(&name) {
            return None;
        }
        let media_type = media_type(&name, table).filter(|found| found.starts_with(kind))?;
        let bytes = self.reader.read(&name, FILE_CAP + 1)?;
        let len = u64::try_from(bytes.len()).ok()?;
        if len > FILE_CAP {
            return None;
        }
        if self.total + len > FACE_CAP {
            return None;
        }
        self.total += len;
        Some((media_type, bytes))
    }
}
