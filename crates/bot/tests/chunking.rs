//! A long HTML message is sent as chunks that each parse on their own, none cut inside a tag or an
//! entity (SPEC-026 A3; R7).
//!
//! The checks read each chunk with this file's own reading of Telegram's HTML, never the chunker's:
//! the tags the Bot API supports, each closed in the order it was opened, and `<`, `>` and `&` only
//! as tags and entities. The visible text is what is left after entity parsing, measured in UTF-16
//! units as the Bot API measures it.

// An integration test is test code: its helpers panic on a failed check.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "support/fake_bot_api.rs"]
mod fake_bot_api;

use deck_streak_bot::Sent;
use deck_streak_bot::chunk::{chunks, visible_units};
use deck_streak_bot::transport::MAX_TEXT_UTF16;
use fake_bot_api::{FakeBotApi, OWNER, golden_send, messages_directory, payload};

/// The tags the Bot API's HTML supports.
const TAGS: [&str; 16] = [
    "b",
    "strong",
    "i",
    "em",
    "u",
    "ins",
    "s",
    "strike",
    "del",
    "tg-spoiler",
    "span",
    "a",
    "tg-emoji",
    "code",
    "pre",
    "blockquote",
];

/// `html`'s visible text, when it parses as Telegram's HTML; else why it does not.
fn parse(html: &str) -> Result<String, String> {
    let mut open: Vec<String> = Vec::new();
    let mut plain = String::new();
    let mut rest = html;
    while let Some(first) = rest.chars().next() {
        match first {
            '<' => {
                let end = rest.find('>').ok_or("a '<' that starts no tag")?;
                let tag = &rest[1..end];
                if let Some(name) = tag.strip_prefix('/') {
                    if open.pop().as_deref() != Some(name) {
                        return Err(format!("</{name}> closes no tag open innermost"));
                    }
                } else {
                    let name = tag.split_whitespace().next().unwrap_or_default();
                    if !TAGS.contains(&name) {
                        return Err(format!("<{name}> is not a tag the Bot API supports"));
                    }
                    open.push(name.to_owned());
                }
                rest = &rest[end + 1..];
            }
            '&' => {
                let end = rest.find(';').ok_or("a bare '&'")?;
                let shown = match &rest[..=end] {
                    "&lt;" => '<',
                    "&gt;" => '>',
                    "&amp;" => '&',
                    "&quot;" => '"',
                    other => return Err(format!("{other} is not an entity the Bot API reads")),
                };
                plain.push(shown);
                rest = &rest[end + 1..];
            }
            '>' => return Err("a bare '>'".to_owned()),
            other => {
                plain.push(other);
                rest = &rest[other.len_utf8()..];
            }
        }
    }
    if open.is_empty() {
        Ok(plain)
    } else {
        Err(format!("{open:?} never closed"))
    }
}

/// The UTF-16 units of `text`.
fn utf16(text: &str) -> usize {
    text.encode_utf16().count()
}

/// Prints how many items a check examined and refuses zero (the tdd pack's examined contract).
fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

#[tokio::test]
async fn a_long_html_message_is_sent_as_chunks_that_each_parse() {
    let source = std::fs::read_to_string(messages_directory().join("long-sample.source.txt"))
        .expect("the committed sample");
    let sample = source.strip_suffix('\n').unwrap_or(&source);
    let whole = parse(sample).expect("the sample itself parses");
    assert!(utf16(&whole) >= 6000, "a sample of {} units", utf16(&whole));

    let fake = FakeBotApi::start().await;
    let directory = tempfile::tempdir().expect("a temporary directory");
    let transport = fake.transport(directory.path());
    let sent = transport.send_html(OWNER, sample, None).await;
    assert!(matches!(sent, Sent::Delivered { .. }), "{sent:?}");

    let calls = fake.calls_of("sendMessage");
    assert!(
        examined("chunk(s) sent", calls.clone()).len() >= 2,
        "a text over the bound is split"
    );
    let mut joined = String::new();
    for (index, call) in calls.iter().enumerate() {
        let text = call.body["text"].as_str().expect("a text");
        let plain = parse(text).unwrap_or_else(|why| panic!("chunk {}: {why}", index + 1));
        assert!(
            utf16(&plain) <= MAX_TEXT_UTF16,
            "chunk {}: {} units",
            index + 1,
            utf16(&plain)
        );
        assert!(
            !plain.trim().is_empty(),
            "chunk {} shows nothing",
            index + 1
        );
        if index + 1 < calls.len() {
            assert!(
                plain.ends_with("\n\n"),
                "chunk {} is cut at a paragraph: {:?}",
                index + 1,
                &plain[plain.len().saturating_sub(40)..]
            );
        }
        joined.push_str(&plain);
        assert_eq!(
            payload(call),
            golden_send(&format!("long-sample.{}", index + 1)),
            "chunk {} is its committed golden",
            index + 1
        );
    }
    assert_eq!(joined, whole, "the chunks show the sample's text, once");

    // The cut fell inside the quotation and its bold: both closed there and opened again.
    let first = calls[0].body["text"].as_str().expect("a text");
    let second = calls[1].body["text"].as_str().expect("a text");
    assert!(
        first.ends_with("</b></blockquote>"),
        "{}",
        &first[first.len() - 40..]
    );
    assert!(
        second.starts_with("<blockquote expandable><b>"),
        "{}",
        &second[..40]
    );

    let goldens = examined(
        "chunk golden(s)",
        std::fs::read_dir(messages_directory())
            .expect("the messages directory")
            .filter_map(Result::ok)
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .filter(|name| name.starts_with("long-sample.") && name.ends_with(".msg.json"))
            .collect(),
    );
    assert_eq!(
        goldens.len(),
        calls.len(),
        "one golden per chunk, and no other"
    );
}

#[test]
fn a_text_that_fits_is_one_chunk_as_given() {
    assert_eq!(chunks("  <b>fits</b> ", 100), vec!["  <b>fits</b> "]);
    assert_eq!(chunks("abc", 3), vec!["abc"], "a text exactly at the bound");
}

#[test]
fn a_text_with_nothing_to_show_is_no_chunk() {
    assert!(chunks("", 10).is_empty());
    assert!(chunks(" \n <b> </b>\n", 10).is_empty());
    // One character to show is enough to send the text as it is.
    assert_eq!(chunks(" \n <b>x</b>\n", 10), vec![" \n <b>x</b>\n"]);
}

#[test]
fn a_cut_prefers_a_paragraph_then_a_line_then_a_word() {
    assert_eq!(
        chunks("one two\n\nthree four five six", 12),
        vec!["one two\n\n", "three four ", "five six"]
    );
    assert_eq!(
        chunks("one two\nthree four five six", 12),
        vec!["one two\n", "three four ", "five six"]
    );
    assert_eq!(chunks("one two three", 9), vec!["one two ", "three"]);
}

#[test]
fn a_word_longer_than_the_bound_is_cut_between_characters() {
    // A character outside the Basic Multilingual Plane counts two units and is never halved.
    assert_eq!(chunks("📚📚📚", 3), vec!["📚", "📚", "📚"]);
    // An entity is one character, never cut.
    assert_eq!(chunks("a&amp;bc", 2), vec!["a&amp;", "bc"]);
    assert_eq!(chunks("abcdef", 4), vec!["abcd", "ef"]);
}

#[test]
fn a_tag_open_at_a_cut_is_closed_and_opened_again_with_its_attributes() {
    let html = r#"<a href="https://x.example/?a=1&amp;b=2"><b>one two three</b></a>"#;
    assert_eq!(
        chunks(html, 8),
        vec![
            r#"<a href="https://x.example/?a=1&amp;b=2"><b>one two </b></a>"#,
            r#"<a href="https://x.example/?a=1&amp;b=2"><b>three</b></a>"#,
        ]
    );
    let nested = r#"<pre><code class="language-text">aaaa bbbb</code></pre>"#;
    assert_eq!(
        chunks(nested, 5),
        vec![
            r#"<pre><code class="language-text">aaaa </code></pre>"#,
            r#"<pre><code class="language-text">bbbb</code></pre>"#,
        ]
    );
}

#[test]
fn a_cut_never_leaves_an_empty_tag() {
    // A cut at a word break before a tag opens leaves the tag to the next chunk.
    assert_eq!(chunks("aaaa <b>bbbb</b>", 5), vec!["aaaa ", "<b>bbbb</b>"]);
    // A cut between characters right after a tag opens moves the tag to the next chunk.
    assert_eq!(chunks("aaaa<b>bbbb</b>", 4), vec!["aaaa", "<b>bbbb</b>"]);
    // A tag that closes right at the cut closes in the chunk before it.
    assert_eq!(chunks("<b>aaa </b>bbbb", 4), vec!["<b>aaa </b>", "bbbb"]);
    for (html, limit) in [
        ("aaaa <b>bbbb</b>", 5),
        ("aaaa<b>bbbb</b>", 4),
        ("<b>aaa </b>bbbb", 4),
    ] {
        for chunk in chunks(html, limit) {
            assert!(!chunk.contains("<b></b>"), "{html} at {limit}: {chunk}");
            parse(&chunk).unwrap_or_else(|why| panic!("{chunk}: {why}"));
        }
    }
}

#[test]
fn a_piece_over_the_bound_goes_alone() {
    // A bound below a character's own units still makes progress: the character goes alone.
    assert_eq!(chunks("📚a", 1), vec!["📚", "a"]);
    assert_eq!(chunks("a📚<b>c</b>", 1), vec!["a", "📚", "<b>c</b>"]);
}

#[test]
fn the_units_are_counted_after_entity_parsing() {
    // A tag's name may carry a hyphen, as Telegram's tg-spoiler and tg-emoji do.
    assert_eq!(visible_units("<tg-spoiler>ab</tg-spoiler>"), 2);
    assert_eq!(
        visible_units("&amp;<b>a</b>📚&#128218;&#x1F4DA;"),
        1 + 1 + 2 + 2 + 2
    );
    assert_eq!(visible_units("&lt;&gt;&quot;"), 3);
    // A `<` or an `&` that starts nothing is one character, so no text is lost.
    assert_eq!(visible_units("a < b & c"), 9);
    assert_eq!(visible_units(""), 0);
}

#[test]
fn a_paragraph_is_a_line_break_after_another() {
    // The paragraph after `aa` and the line after `bb` both fit the bound; the cut takes the
    // paragraph.
    assert_eq!(chunks("aa\n\nbb\ncc dd", 9), vec!["aa\n\n", "bb\ncc dd"]);
}

#[test]
fn a_tag_name_starts_with_a_letter() {
    // `<3>` opens no tag, so each of its characters counts.
    assert_eq!(visible_units("a <3> b"), 7);
}

#[test]
fn a_closing_tag_name_starts_with_a_letter() {
    // `</3>` closes no tag, so each of its characters counts.
    assert_eq!(visible_units("a</3>b"), 6);
}

#[test]
fn a_cut_takes_the_closing_tags_that_end_the_text() {
    // The character goes alone, over the bound, with the tag that closes it at the text's end.
    assert_eq!(chunks("<b>📚</b>", 1), vec!["<b>📚</b>"]);
}
