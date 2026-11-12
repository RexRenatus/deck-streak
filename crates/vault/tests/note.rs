//! A reading's note as the adapter writes and reads it (SPEC-042 R5, R6, R10): its topic key and
//! body hash read back as written, a frontmatter value that could leave its line and a body that is
//! empty or holds a box line are refused, and only a note that still ends with its two box lines
//! has a body to read or replace.

use deck_streak_kernel::StudyDay;
use deck_streak_vault::{BodyHash, TopicKey, VaultError, note};

const DIGEST: &str = "5d41402abc4b2a76b9719d911017c592aaf1d7f2c3b4e5a69788796a5b4c3d2e";
const BODY: &str = "# Hearsay\n\nAn out-of-court statement offered for its truth.";

#[test]
fn a_topic_key_reads_back_as_the_caller_named_it() {
    let key = TopicKey::new("law/evidence").expect("a topic key");

    assert_eq!(key.as_str(), "law/evidence");
    assert_eq!(key.to_string(), "law/evidence");
    assert_eq!(key.file_name(), "law-evidence.md");
}

#[test]
fn a_body_hash_prints_and_reads_as_64_lowercase_hexadecimal_digits() {
    // FIPS 180-4's example: the SHA-256 of `abc`.
    let abc = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
    let hash = BodyHash::of("abc");

    assert_eq!(hash.to_string(), abc);
    assert_eq!(format!("{hash:?}"), format!("BodyHash({abc})"));
    assert_eq!(abc.parse::<BodyHash>(), Ok(hash));
}

#[test]
fn a_new_reading_is_rendered_with_the_predecessors_keys_its_body_and_two_unticked_boxes() {
    let topic = TopicKey::new("law/evidence").expect("a topic key");
    let day = StudyDay::from_epoch_day(20_000);

    let rendered = note::render(&topic, day, DIGEST, BODY).expect("a reading renders");

    assert_eq!(
        rendered,
        format!(
            "---\ntype: reading\ntopic: law/evidence\ndate: {day}\nfirst_generated: {day}\n\
             last_rolled: {day}\nrolls: 0\ndigest: {DIGEST}\ntags: [reading, law/evidence]\n\
             ai_generated: true\n---\n{BODY}\n\n- [ ] Studied\n- [ ] I read it\n"
        )
    );
}

#[test]
fn a_digest_that_leaves_its_line_or_a_body_that_is_empty_or_boxed_is_refused() {
    let topic = TopicKey::new("law/evidence").expect("a topic key");
    let day = StudyDay::from_epoch_day(20_000);
    for digest in ["one\ntwo", "one\rtwo", "one]two"] {
        let refused = note::render(&topic, day, digest, BODY);
        assert!(
            matches!(
                refused,
                Err(VaultError::InvalidFrontmatterValue { field: "digest" })
            ),
            "the digest {digest:?} rendered as {refused:?}"
        );
    }
    for body in ["", " \n\t\u{1c}"] {
        let refused = note::render(&topic, day, DIGEST, body);
        assert!(
            matches!(refused, Err(VaultError::InvalidBody("is empty"))),
            "the body {body:?} rendered as {refused:?}"
        );
    }
    for body in [
        "A primer.\n- [ ] Studied\nMore.",
        "A primer.\r\n- [x] I read it\r\n",
    ] {
        let refused = note::render(&topic, day, DIGEST, body);
        assert!(
            matches!(
                refused,
                Err(VaultError::InvalidBody("holds a box line of its own"))
            ),
            "the body {body:?} rendered as {refused:?}"
        );
    }
}

#[test]
fn only_a_note_that_ends_with_its_two_box_lines_has_a_body_to_read_or_replace() {
    // The shortest frontmatter: the body is read and replaced between it and the boxes.
    let short = "---\n---\nA body.\n\n- [ ] Studied\n- [x] I read it\n";
    assert_eq!(note::body(short), Some("A body."));
    assert_eq!(
        note::with_body(short, "Another body."),
        Some("---\n---\nAnother body.\n\n- [ ] Studied\n- [x] I read it\n".to_owned())
    );
    // The box lines are gone: there is nothing to read or replace, however long the note is.
    let boxless =
        "---\ntype: reading\n---\nA body long enough to be longer than the two box lines are.\n";
    assert_eq!(note::body(boxless), None);
    assert_eq!(note::with_body(boxless, "Another body."), None);
    // The blank line before the boxes is the closing delimiter's own line ending: the note holds
    // no body, and none is read.
    let bodiless =
        "---\ntype: reading\ntopic: law/evidence\n---\n\n- [ ] Studied\n- [ ] I read it\n";
    assert_eq!(note::body(bodiless), None);
    assert_eq!(note::with_body(bodiless, "Another body."), None);
}
