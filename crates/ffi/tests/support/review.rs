//! The review fixture (SPEC-348 R8): a collection whose deck `Review` holds a text card, an image
//! card, a sound card and a speech card, all new on the default preset, beside a media folder that
//! holds the image and the sound.
//!
//! One builder, written with the engine's own API before the adapter runs. The `review-fixture`
//! example includes this file alone to write the fixture where the Apple job uploads it, and the
//! adapter's tests include it to build the same collection in a scratch directory of their own.
//! The image and the sound are built here from their bytes, never committed as binaries. The four
//! cards' ids are fixed, so the intervals the engine gives the first card do not change from run to
//! run (`CARD_IDS` says what else they depend on). It returns every failure as an error rather than
//! panicking, because the example is not a test.

#![allow(
    dead_code,
    reason = "the example and each test that includes this file read the part they need"
)]

use std::error::Error;
use std::path::{Path, PathBuf};

use anki::collection::CollectionBuilder;

/// The deck that holds the fixture's four cards.
pub const DECK: &str = "Review";
/// The image's file name in the media folder.
pub const IMAGE: &str = "dot.png";
/// The sound's file name in the media folder.
pub const SOUND: &str = "tone.wav";
/// The image's width and height, in pixels.
pub const SIDE: u32 = 64;
/// The image card's alternative text.
pub const ALT: &str = "a grey dot";

/// The text card's front: no media and no tag.
const TEXT_FRONT: &str = "a text card";
/// The image card's front.
const IMAGE_FRONT: &str = r#"<img src="dot.png" alt="a grey dot">"#;
/// The sound card's front.
const SOUND_FRONT: &str = "[sound:tone.wav]";
/// The speech card's front.
const SPEECH_FRONT: &str = "[anki:tts lang=en_US]a spoken card[/anki:tts]";
/// Every card's back. The stock Basic answer template opens with `{{FrontSide}}`.
const BACK: &str = "the back";
/// The four cards' ids, in the order the builder adds them: text, image, sound and speech. The
/// engine seeds a review interval's fuzz from the card's id, so a new card's Easy interval is the
/// same on every run only for the same id; the core's review-pairs test fixes its cards' ids for
/// that reason. An id the engine gives from its clock read Easy as `4d` on one run and `5d` on
/// another.
///
/// The text card, which the review screen shows first, carries `1_000_007`, measured through the
/// adapter: its four intervals on the default preset are A1's `<1m`, `<6m`, `<10m` and `4d`. The
/// core's `1_000_002` reads `5d` here. The engine's load balancer, which picks the day within the
/// fuzz's range, counts each card's `due` as a day's load, and a new card's `due` is its position:
/// this deck's sound and speech cards sit at 3 and 4, inside the Easy range of 3 to 5 days, where
/// the core's two-card collection has none. The word therefore also moves with the days since the
/// collection was made: `1_000_007` reads `4d` on the day the fixture is built and on the next, and
/// `3d` two days on. It is the first id from `1_000_006` up that reads `4d` on the day the fixture
/// is built.
const CARD_IDS: [i64; 4] = [1_000_007, 1_000_008, 1_000_009, 1_000_010];
/// The sound's sample rate: 16-bit mono samples a second.
const SAMPLE_RATE: u32 = 8_000;

/// The fixture's four cards' fixed ids, by what each shows.
pub struct Cards {
    /// The text card.
    pub text: i64,
    /// The card whose front shows the image.
    pub image: i64,
    /// The card whose front plays the sound.
    pub sound: i64,
    /// The card whose front speaks through a TTS tag.
    pub speech: i64,
}

/// A review fixture built and closed, ready for the adapter to open.
pub struct Review {
    /// The directory that holds the collection and its media folder.
    pub dir: PathBuf,
    /// The collection file, `collection.anki2` in `dir`.
    pub collection: PathBuf,
    /// The four cards' ids.
    pub cards: Cards,
}

/// Builds the review fixture in `dir`: `collection.anki2` and `collection.media/` with the image
/// and the sound.
///
/// # Errors
///
/// Any step the engine or the file system refuses: the directories, the two files, the
/// collection, the deck, the stock Basic note type, a note, or fixing a card's id.
pub fn build(dir: &Path) -> Result<Review, Box<dyn Error>> {
    let media = dir.join("collection.media");
    std::fs::create_dir_all(&media)?;
    std::fs::write(media.join(IMAGE), png()?)?;
    std::fs::write(media.join(SOUND), wav()?)?;
    let collection = dir.join("collection.anki2");
    let mut col = CollectionBuilder::new(&collection).build()?;
    let deck = col.get_or_create_normal_deck(DECK)?.id;
    let basic = col
        .get_notetype_by_name("Basic")?
        .ok_or("the engine created no stock Basic note type")?;
    let mut card = |front: &str, id: i64| -> Result<i64, Box<dyn Error>> {
        let mut note = basic.new_note();
        note.set_field(0, front)?;
        note.set_field(1, BACK)?;
        col.add_note(&mut note, deck)?;
        let fixed = col
            .storage
            .db()
            .execute("update cards set id = ? where nid = ?", [id, note.id.0])?;
        if fixed != 1 {
            return Err(format!("the note holds {fixed} card(s), not one").into());
        }
        Ok(id)
    };
    let [text, image, sound, speech] = CARD_IDS;
    let cards = Cards {
        text: card(TEXT_FRONT, text)?,
        image: card(IMAGE_FRONT, image)?,
        sound: card(SOUND_FRONT, sound)?,
        speech: card(SPEECH_FRONT, speech)?,
    };
    col.close(None)?;
    Ok(Review {
        dir: dir.to_path_buf(),
        collection,
        cards,
    })
}

/// A 64 by 64 pixel greyscale PNG of a dark dot on a light ground. Its rows are stored
/// uncompressed in one final deflate block, with the Adler-32 and CRC-32 checksums the formats
/// require, so no encoder is needed.
fn png() -> Result<Vec<u8>, Box<dyn Error>> {
    let centre = SIDE / 2;
    let mut rows = Vec::new();
    for y in 0..SIDE {
        // Each row opens with its filter type: none.
        rows.push(0);
        for x in 0..SIDE {
            let (dx, dy) = (x.abs_diff(centre), y.abs_diff(centre));
            rows.push(if dx * dx + dy * dy < 400 { 0x40 } else { 0xE0 });
        }
    }
    let stored = u16::try_from(rows.len())?;
    // The zlib header (deflate, the smallest window that holds the rows), then one final stored
    // block: its length, the length's complement, the bytes, and the Adler-32 of the bytes.
    let mut zlib = vec![0x78, 0x01, 0x01];
    zlib.extend_from_slice(&stored.to_le_bytes());
    zlib.extend_from_slice(&(!stored).to_le_bytes());
    zlib.extend_from_slice(&rows);
    zlib.extend_from_slice(&adler32(&rows).to_be_bytes());
    // Width, height, bit depth 8, greyscale, deflate, adaptive filtering, no interlace.
    let mut header = Vec::new();
    header.extend_from_slice(&SIDE.to_be_bytes());
    header.extend_from_slice(&SIDE.to_be_bytes());
    header.extend_from_slice(&[8, 0, 0, 0, 0]);
    let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
    chunk(&mut png, *b"IHDR", &header)?;
    chunk(&mut png, *b"IDAT", &zlib)?;
    chunk(&mut png, *b"IEND", &[])?;
    Ok(png)
}

/// Appends one PNG chunk: its data's length, its type, the data, and the CRC-32 of type and data.
fn chunk(png: &mut Vec<u8>, kind: [u8; 4], data: &[u8]) -> Result<(), Box<dyn Error>> {
    png.extend_from_slice(&u32::try_from(data.len())?.to_be_bytes());
    let start = png.len();
    png.extend_from_slice(&kind);
    png.extend_from_slice(data);
    let crc = crc32(&png[start..]);
    png.extend_from_slice(&crc.to_be_bytes());
    Ok(())
}

/// The CRC-32 PNG uses (ISO 3309: the reflected polynomial `0xEDB8_8320`), a bit at a time.
fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = u32::MAX;
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

/// The Adler-32 zlib ends with (RFC 1950).
fn adler32(bytes: &[u8]) -> u32 {
    let (mut low, mut high) = (1_u32, 0_u32);
    for &byte in bytes {
        low = (low + u32::from(byte)) % 65_521;
        high = (high + low) % 65_521;
    }
    (high << 16) | low
}

/// A quarter of a second of a square-wave tone, 16-bit mono PCM, in a RIFF WAVE file.
fn wav() -> Result<Vec<u8>, Box<dyn Error>> {
    let samples: Vec<u8> = (0..SAMPLE_RATE / 4)
        .flat_map(|index| {
            let level: i16 = if (index / 9) % 2 == 0 { 8_000 } else { -8_000 };
            level.to_le_bytes()
        })
        .collect();
    let data = u32::try_from(samples.len())?;
    let mut wav = b"RIFF".to_vec();
    wav.extend_from_slice(&(36 + data).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    // The format chunk: its length, PCM, one channel, the rates, the block size and the depth.
    wav.extend_from_slice(&16_u32.to_le_bytes());
    wav.extend_from_slice(&1_u16.to_le_bytes());
    wav.extend_from_slice(&1_u16.to_le_bytes());
    wav.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    wav.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes());
    wav.extend_from_slice(&2_u16.to_le_bytes());
    wav.extend_from_slice(&16_u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data.to_le_bytes());
    wav.extend_from_slice(&samples);
    Ok(wav)
}
