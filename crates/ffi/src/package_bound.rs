//! The package import's bound (SPEC-681 R2, R4 and R5; ADR-703 D3 and D4).
//!
//! Before the one import call reaches the engine, the native adapter opens the picked package and
//! reads every zip entry through its decompressor, and every entry that opens with the zstd frame
//! magic through a zstd stream decoder as well, each under a reader that fails once more than
//! [`BOUND`] bytes have come out of it. Each count is of the bytes as they decompress, so a size
//! the archive declares is never trusted. A member past the bound, or a package the reader cannot
//! read, refuses the call in words before the engine runs, so no row is read or written; a package
//! within the bound reaches the engine with the import's four options in its request.

use std::fs::File;
use std::io::{self, Read};

use zip::ZipArchive;

/// The bound every zip entry and every inner compressed member is held to: 1 GiB of decompressed
/// bytes, the bound the format crate's opener holds for a foreign file (SPEC-681 R4).
pub const BOUND: u64 = 1 << 30;

/// The words a package with a member past the bound reads as (SPEC-681 R5).
pub const PAST_BOUND: &str =
    "This package holds a file larger than the app imports (1 GiB), so nothing was imported.";

/// The words a file that is no package the reader can read reads as (SPEC-681 R5).
pub const UNREADABLE: &str =
    "This file is not an Anki deck package the app can read, so nothing was imported.";

/// The bytes a zstd frame opens with: its magic number, `0xFD2FB528`, in little-endian order.
const ZSTD_MAGIC: [u8; 4] = [0x28, 0xB5, 0x2F, 0xFD];

/// The key of field 1, length-delimited: the request's package path, and a `BackendError`'s
/// message.
const FIELD_1_BYTES: u8 = 0x0A;

/// The key of field 2, length-delimited: the request's options.
const FIELD_2_BYTES: u8 = 0x12;

/// `ImportAnkiPackageOptions` as the import sends it (SPEC-681 R2), each of the four fields
/// written: no note type the collection holds merged or changed (`merge_notetypes`, field 1,
/// false), notes and note types updated under the engine's default condition (`update_notes` and
/// `update_notetypes`, fields 2 and 3, `IF_NEWER`), and the package's scheduling kept
/// (`with_scheduling`, field 4, true).
const FOUR_OPTIONS: [u8; 8] = [0x08, 0, 0x10, 0, 0x18, 0, 0x20, 1];

/// Why the adapter refused a package.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refused {
    /// A zip entry or an inner compressed member decompressed past [`BOUND`].
    PastBound,
    /// The request names no package path, the file is no zip archive, or a member could not be
    /// read.
    Unreadable,
}

impl Refused {
    /// The refusal's words, as the app shows them (SPEC-681 R5).
    #[must_use]
    pub fn words(self) -> &'static str {
        match self {
            Self::PastBound => PAST_BOUND,
            Self::Unreadable => UNREADABLE,
        }
    }

    /// The refusal in the engine's error shape: an encoded `BackendError` whose message, field 1,
    /// is the refusal's words, its kind left at `INVALID_INPUT`, the default the wire omits, as
    /// the core's own refusals are (ADR-703 D4).
    #[must_use]
    pub fn backend_error(self) -> Vec<u8> {
        let words = self.words().as_bytes();
        let mut error = Vec::with_capacity(words.len() + 4);
        error.push(FIELD_1_BYTES);
        put_varint(&mut error, words.len());
        error.extend_from_slice(words);
        error
    }
}

/// The import request the engine runs, once the package has been read under the bound: the
/// package's path from the app's request, with the import's four options written into field 2
/// whatever the app sent there (SPEC-681 R2, R4).
///
/// # Errors
///
/// [`Refused::Unreadable`] when the request names no UTF-8 package path or the package cannot be
/// read, and [`Refused::PastBound`] when a zip entry or an inner compressed member decompresses
/// past [`BOUND`].
pub fn checked_request(input: &[u8]) -> Result<Vec<u8>, Refused> {
    let path = package_path(input).ok_or(Refused::Unreadable)?;
    read_bounded(path)?;
    let mut request = Vec::with_capacity(path.len() + FOUR_OPTIONS.len() + 12);
    request.push(FIELD_1_BYTES);
    put_varint(&mut request, path.len());
    request.extend_from_slice(path.as_bytes());
    request.push(FIELD_2_BYTES);
    put_varint(&mut request, FOUR_OPTIONS.len());
    request.extend_from_slice(&FOUR_OPTIONS);
    Ok(request)
}

/// The package path an `ImportAnkiPackageRequest` names, field 1 at its last occurrence as the
/// wire's merge rule reads it, or `None` when the request does not decode or names no UTF-8 path.
fn package_path(input: &[u8]) -> Option<&str> {
    let mut at = 0;
    let mut path = None;
    while at < input.len() {
        let key = varint(input, &mut at)?;
        match key & 0b111 {
            0 => {
                varint(input, &mut at)?;
            }
            1 => at = at.checked_add(8)?,
            2 => {
                let length = usize::try_from(varint(input, &mut at)?).ok()?;
                let end = at.checked_add(length)?;
                let field = input.get(at..end)?;
                if key >> 3 == 1 {
                    path = Some(field);
                }
                at = end;
            }
            5 => at = at.checked_add(4)?,
            _ => return None,
        }
    }
    if at != input.len() {
        return None;
    }
    std::str::from_utf8(path?).ok()
}

/// Reads one protobuf varint at `at`, moving `at` past it.
fn varint(bytes: &[u8], at: &mut usize) -> Option<u64> {
    let mut value = 0_u64;
    for shift in (0..64).step_by(7) {
        let byte = *bytes.get(*at)?;
        *at = at.checked_add(1)?;
        value |= u64::from(byte & 0x7F) << shift;
        if byte & 0x80 == 0 {
            return Some(value);
        }
    }
    None
}

/// Appends `value` as a protobuf varint. The loop runs at most as many times as a `usize` varint
/// has bytes, and every input writes its last byte and returns inside that bound.
fn put_varint(out: &mut Vec<u8>, value: usize) {
    let mut value = value;
    for _ in 0..usize::BITS.div_ceil(7) {
        if value < 0x80 {
            let [low, ..] = value.to_le_bytes();
            out.push(low);
            return;
        }
        let [low, ..] = ((value & 0x7F) + 0x80).to_le_bytes();
        out.push(low);
        value >>= 7;
    }
}

/// Reads every member of the package at `path` under the bound.
fn read_bounded(path: &str) -> Result<(), Refused> {
    let file = File::open(path).map_err(|_| Refused::Unreadable)?;
    let mut archive = ZipArchive::new(file).map_err(|_| Refused::Unreadable)?;
    for index in 0..archive.len() {
        let entry = archive.by_index(index).map_err(|_| Refused::Unreadable)?;
        member(entry)?;
    }
    Ok(())
}

/// Reads one zip entry through its decompressor under the bound and, when its bytes open with the
/// zstd frame magic, through a zstd stream decoder under the bound as well, so a count past the
/// bound on either reader refuses the package.
fn member(entry: impl Read) -> Result<(), Refused> {
    let mut entry = Bounded::new(entry);
    let mut head = Vec::with_capacity(ZSTD_MAGIC.len());
    if (&mut entry).take(4).read_to_end(&mut head).is_err() {
        return Err(Refused::Unreadable);
    }
    if head != ZSTD_MAGIC {
        return drain(&mut entry);
    }
    let decoded = match zstd::stream::read::Decoder::new(head.as_slice().chain(&mut entry)) {
        Ok(decoder) => drain(&mut Bounded::new(decoder)),
        Err(_) => Err(Refused::Unreadable),
    };
    if entry.past {
        Err(Refused::PastBound)
    } else {
        decoded
    }
}

/// Reads `reader` to its end, refusing past the bound or at a read it cannot make.
fn drain(reader: &mut Bounded<impl Read>) -> Result<(), Refused> {
    match io::copy(reader, &mut io::sink()) {
        Ok(_) => Ok(()),
        Err(_) if reader.past => Err(Refused::PastBound),
        Err(_) => Err(Refused::Unreadable),
    }
}

/// A reader that counts the bytes it hands out and fails once the count would pass [`BOUND`],
/// remembering that it did.
struct Bounded<R> {
    /// The reader whose bytes are counted.
    inner: R,
    /// The bytes still within the bound.
    left: u64,
    /// Whether a read would have passed the bound.
    past: bool,
}

impl<R> Bounded<R> {
    /// `inner` under the whole bound.
    fn new(inner: R) -> Self {
        Self {
            inner,
            left: BOUND,
            past: false,
        }
    }
}

impl<R: Read> Read for Bounded<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let read = self.inner.read(buf)?;
        let counted = u64::try_from(read).unwrap_or(u64::MAX);
        if let Some(left) = self.left.checked_sub(counted) {
            self.left = left;
            Ok(read)
        } else {
            self.past = true;
            Err(io::Error::other(PAST_BOUND))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::put_varint;

    /// The bytes the varint writer appends for `value`.
    fn written(value: usize) -> Vec<u8> {
        let mut out = Vec::new();
        put_varint(&mut out, value);
        out
    }

    /// The varint writer's exact bytes at each width's edges, the largest input included.
    #[test]
    fn the_varint_writer_writes_exact_leb128_bytes() {
        assert_eq!(written(0), [0x00], "zero is one byte");
        assert_eq!(
            written(0x7F),
            [0x7F],
            "the largest one-byte value is one byte"
        );
        assert_eq!(written(0x80), [0x80, 0x01], "the smallest two-byte value");
        assert_eq!(written(0x3FFF), [0xFF, 0x7F], "the largest two-byte value");
        assert_eq!(
            written(0x4000),
            [0x80, 0x80, 0x01],
            "the smallest three-byte value"
        );
        assert_eq!(
            written(usize::MAX),
            [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x01],
            "the largest value takes the most bytes a usize varint takes"
        );
    }
}
