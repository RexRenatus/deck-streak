//! A certified `ds-` name round-trips, a mismatched one is refused, and an imported name is kept
//! as it was written (SPEC-513 A2).

use deck_streak_media::{MediaHash, MediaName, NameRefusal};

/// The certified form of the SHA-256 of `abc`, written out here.
const ABC: &str = "ds-ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

#[test]
fn a_certified_name_round_trips() {
    for (text, extension) in [
        (ABC.to_owned(), None),
        (format!("{ABC}.png"), Some("png")),
        (format!("{ABC}.woff2"), Some("woff2")),
        (format!("{ABC}.abcdefghij"), Some("abcdefghij")),
    ] {
        let name = MediaName::parse(&text).expect("a certified name reads");
        let MediaName::Certified(certified) = &name else {
            panic!("{text} reads as certified");
        };
        assert_eq!(certified.hash(), MediaHash::of(b"abc"));
        assert_eq!(certified.extension(), extension);
        assert_eq!(
            name.to_string(),
            text,
            "the name writes back as it was read"
        );
        assert_eq!(MediaName::parse(&name.to_string()), Ok(name.clone()));
        assert_eq!(
            MediaName::for_bytes(b"abc", extension),
            Ok(name),
            "the name of the bytes is the name read"
        );
    }
}

#[test]
fn a_mismatched_certified_name_is_refused() {
    let name = MediaName::parse(&format!("{ABC}.png")).expect("a certified name reads");
    assert_eq!(name.check_bytes(b"abc"), Ok(()));
    assert_eq!(name.check_bytes(b"abd"), Err(NameRefusal::Mismatch));
    assert_eq!(
        name.check(MediaHash::of(b"")),
        Err(NameRefusal::Mismatch),
        "a name certifies one hash"
    );
    assert_eq!(name.claimed(), Some(MediaHash::of(b"abc")));
}

#[test]
fn a_name_that_looks_certified_but_is_not_is_refused() {
    let upper = format!("ds-{}", ABC[3..].to_uppercase());
    for text in [
        upper.as_str(),
        &format!("{ABC}."),
        &format!("{ABC}.PNG"),
        &format!("{ABC}.abcdefghijk"),
        &format!("{ABC}.tar.gz"),
        &format!("{ABC}-x.png"),
        &format!("{ABC}0"),
    ] {
        assert_eq!(
            MediaName::parse(text),
            Err(NameRefusal::Malformed),
            "{text} is refused"
        );
    }
    let hash = MediaHash::of(b"abc");
    for extension in ["", "PNG", "p.ng", "abcdefghijk", "pn g"] {
        assert_eq!(
            MediaName::certified(hash, Some(extension)),
            Err(NameRefusal::BadExtension),
            "the extension {extension:?} is refused"
        );
    }
}

#[test]
fn an_imported_name_is_kept_unchanged() {
    for text in [
        "ds_harbor.png",
        "ds-logo.png",
        "_ds_font.woff2",
        "paste-29e2dcfbb16f63bb0254df7585a15bb6fb5e927d.png",
        "ds-ba7816bf.png",
        "caf\u{e9} na\u{ef}ve.mp3",
        "ds%20amber.png",
    ] {
        let name = MediaName::parse(text).expect("a plain name reads");
        assert_eq!(name, MediaName::Imported(text.to_owned()));
        assert_eq!(name.to_string(), text, "an imported name is never renamed");
        assert_eq!(name.claimed(), None, "an imported name claims no hash");
        assert_eq!(name.check_bytes(b"any bytes"), Ok(()));
    }
}

#[test]
fn a_name_that_is_not_plain_is_refused() {
    for text in ["", ".", "..", "a/b.png", "a\\b.png", "a\0b.png", "/abs.png"] {
        assert_eq!(
            MediaName::parse(text),
            Err(NameRefusal::NotPlain),
            "{text:?} is refused"
        );
    }
}
