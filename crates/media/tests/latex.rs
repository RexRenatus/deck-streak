//! The engine's LaTeX image names (SPEC-513 R9; ADR-531 D8): the crate's SHA-1 gives the FIPS 180
//! examples and every padding boundary, `latex_images` names every image of the engine's LaTeX
//! golden but the one case ADR-531 D8 cites, hand cases hold the shapes the golden does not draw,
//! and the index lists a note's LaTeX images, PNG or SVG, as the engine's media check does.

#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use deck_streak_media::sha1::sha1_hex;
use deck_streak_media::{
    Keep, LatexFormat, MediaIndex, Source, StoredFile, SweepPolicy, latex_images, latex_name, plan,
};
use serde_json::Value;

/// The committed golden of the engine's LaTeX images.
const LATEX_IMAGES: &str = "anki.render.latex_images";

/// Messages and their SHA-1, each a unit repeated a number of times: the FIPS 180 examples (`abc`,
/// the empty message, the 448-bit and 896-bit messages, a million `a`), then lengths either side of
/// one block's and two blocks' padding. Each digest was computed by Python's `hashlib`.
const SHA1: [(&str, usize, &str); 12] = [
    ("abc", 1, "a9993e364706816aba3e25717850c26c9cd0d89d"),
    ("", 1, "da39a3ee5e6b4b0d3255bfef95601890afd80709"),
    (
        "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq",
        1,
        "84983e441c3bd26ebaae4aa1f95129e5e54670f1",
    ),
    (
        "abcdefghbcdefghicdefghijdefghijkefghijklfghijklmghijklmnhijklmnoijklmnopjklmnopqklmnopqrlmnopqrsmnopqrstnopqrstu",
        1,
        "a49b2446a02c645bf419f995b67091253a04a259",
    ),
    ("a", 1_000_000, "34aa973cd4c4daa4f61eeb2bdbad27316534016f"),
    ("x", 55, "cef734ba81a024479e09eb5a75b6ddae62e6abf1"),
    ("x", 56, "901305367c259952f4e7af8323f480d59f81335b"),
    ("x", 63, "0ddc4e0cccd9a12850deb5abb0853a4425559fec"),
    ("x", 64, "bb2fa3ee7afb9f54c6dfb5d021f14b1ffe40c163"),
    ("x", 65, "78c741ddc482e4cdf8c474a0876347a0905b6233"),
    ("x", 119, "4300320394f7ee239bcdce7d3b8bcee173a0cd5c"),
    ("x", 120, "ceb2821639c4b6dcb10bce0e522ca2e608ce056d"),
];

#[test]
fn sha1_gives_the_fips_examples_and_every_padding_boundary() {
    for (unit, times, digest) in SHA1 {
        let message = unit.repeat(times);
        assert_eq!(
            sha1_hex(message.as_bytes()),
            digest,
            "the SHA-1 of {times} x {unit:?} ({} bytes)",
            message.len()
        );
    }
    println!("examined {} message(s)", SHA1.len());
}

/// The case of the golden ADR-531 D8 cites: its expression holds `&copy;` and `&eacute;`, named
/// entities the engine decodes and the grammar's decoder does not know, so the expression is kept
/// as written and its image is named for that.
const CITED: usize = 14;

/// The golden's images for one case, as (expression, name) pairs.
fn golden_images(output: &Value) -> Vec<(String, String)> {
    output["images"]
        .as_array()
        .expect("an array of images")
        .iter()
        .map(|image| {
            (
                image["latex"].as_str().expect("an expression").to_owned(),
                image["name"].as_str().expect("a name").to_owned(),
            )
        })
        .collect()
}

/// The images `latex_images` names for one case of the golden, as (expression, name) pairs.
fn our_images(input: &Value) -> Vec<(String, String)> {
    latex_images(input["text"].as_str().expect("a text"), format(input))
        .into_iter()
        .map(|image| (image.expression, image.name))
        .collect()
}

/// The case's image type.
fn format(input: &Value) -> LatexFormat {
    if input["svg"].as_bool().expect("an image type") {
        LatexFormat::Svg
    } else {
        LatexFormat::Png
    }
}

#[test]
fn latex_images_name_every_image_of_the_engines_latex_golden_but_the_cited_case() {
    let golden = golden::read(&golden::committed(LATEX_IMAGES)).expect("the golden");
    assert_eq!(golden.cases.len(), 838, "the golden's cases");
    let mut equal = 0;
    let mut images = 0;
    let mut svg = 0;
    let mut differ = Vec::new();
    for (index, case) in golden.cases.iter().enumerate() {
        let theirs = golden_images(&case.output);
        let ours = our_images(&case.input);
        images += theirs.len();
        if format(&case.input) == LatexFormat::Svg {
            svg += 1;
        }
        if ours == theirs {
            equal += 1;
            continue;
        }
        differ.push(index);
        assert_eq!(
            index, CITED,
            "an uncited case differs: {ours:?} against {theirs:?}"
        );
        assert_eq!(
            ours.len(),
            theirs.len(),
            "case {index}: the same images are found"
        );
        for ((our_expression, our_name), (expression, name)) in ours.iter().zip(&theirs) {
            if our_name != name {
                assert!(
                    our_expression.contains('&') && !expression.contains('&'),
                    "case {index}: only an entity the grammar keeps differs: {our_expression:?}"
                );
            }
            assert_eq!(
                &latex_name(expression, format(&case.input)),
                name,
                "case {index}: the engine's own expression is named as the engine names it"
            );
        }
    }
    println!(
        "examined {} case(s) of {} ({images} image(s), {svg} SVG case(s)): {equal} equal, \
         {} cited to ADR-531 D8",
        golden.cases.len(),
        golden.function,
        differ.len()
    );
    assert_eq!(differ, [CITED], "the cited case differs, and no other");
}

#[test]
fn a_latex_answer_changed_by_hand_is_caught() {
    let mut golden = golden::read(&golden::committed(LATEX_IMAGES)).expect("the golden");
    golden.cases[0].output["images"][0]["name"] =
        Value::from("latex-0000000000000000000000000000000000000000.png");
    let case = &golden.cases[0];
    assert_ne!(
        our_images(&case.input),
        golden_images(&case.output),
        "a planted name is caught"
    );
}

/// Hand cases of the shapes the golden does not draw, each with the expressions the engine's
/// expressions give.
const HAND: [(&str, &[&str]); 9] = [
    ("[LaTeX]a[/LATEX] [$]B[/$]", &["a", "$B$"]),
    (
        "[latex]a<br/>b<br />c<DIV>d</div>e<BR>f[/latex]",
        &["a\nbc\nde\nf"],
    ),
    ("[latex]a<!-- <b>x</b> -->b<!-->c[/latex]", &["abc"]),
    (
        "[latex]a<style>.x{}</style>b<SCRIPT>s()</script>c[/latex]",
        &["abc"],
    ),
    ("[latex][/latex] [latex]x[/latex]", &["[/latex] [latex]x"]),
    ("[latex]x [$]y", &[]),
    (
        "[$]a[/$][$]a[/$][$$]b[/$$]",
        &["$a$", "$a$", "\\begin{displaymath}b\\end{displaymath}"],
    ),
    ("[latex]a &amp; b&nbsp;c[/latex]", &["a & b c"]),
    ("[latex]caf\u{e9}[/latex]", &["caf\u{e9}"]),
];

#[test]
fn latex_images_read_the_shapes_the_golden_does_not_draw() {
    for (text, expressions) in HAND {
        let images = latex_images(text, LatexFormat::Png);
        let found: Vec<&str> = images
            .iter()
            .map(|image| image.expression.as_str())
            .collect();
        assert_eq!(found, expressions.to_vec(), "{text:?}");
        for image in &images {
            assert_eq!(
                image.name,
                format!("latex-{}.png", sha1_hex(image.expression.as_bytes())),
                "{text:?}: the name is the expression's SHA-1"
            );
        }
    }
    let svg = latex_images("[$]y[/$]", LatexFormat::Svg);
    assert_eq!(
        svg.iter()
            .map(|image| image.name.as_str())
            .collect::<Vec<_>>(),
        ["latex-092e364e1d9d19ad5fffb0b46ef4cc7f2da02c1c.svg"],
        "an SVG note type's image"
    );
    println!("examined {} hand case(s) and 1 SVG case", HAND.len());
}

/// A collection of two notes: one of a note type that renders PNG, one of a note type that renders
/// SVG, and a store that holds the first note's image and, under the wrong type, the second's.
struct TwoNotes;

impl MediaIndex for TwoNotes {
    fn sources(&self) -> Vec<Source<'_>> {
        vec![
            Source::Html("[latex]x^{2}[/latex]"),
            Source::HtmlSvg("[$]y[/$]"),
        ]
    }

    fn stored(&self) -> Vec<StoredFile> {
        [
            "latex-1644e14eeb8d1a2fbc9fb06ff49b754832691025.png",
            "latex-092e364e1d9d19ad5fffb0b46ef4cc7f2da02c1c.png",
        ]
        .into_iter()
        .map(|name| StoredFile {
            name: name.to_owned(),
            added_ms: 0,
            imported: false,
        })
        .collect()
    }
}

#[test]
fn the_plan_lists_a_notes_latex_images_and_keeps_a_stored_one() {
    let planned = plan(
        &TwoNotes,
        SweepPolicy {
            now_ms: 1_000,
            grace_ms: 0,
        },
    );
    assert_eq!(
        planned.kept,
        [(
            "latex-1644e14eeb8d1a2fbc9fb06ff49b754832691025.png".to_owned(),
            Keep::Live
        )],
        "the PNG note's stored image is live"
    );
    assert_eq!(
        planned.delete,
        ["latex-092e364e1d9d19ad5fffb0b46ef4cc7f2da02c1c.png"],
        "the SVG note's image stored as PNG is named by nothing"
    );
    assert_eq!(
        planned.missing,
        ["latex-092e364e1d9d19ad5fffb0b46ef4cc7f2da02c1c.svg"],
        "the SVG note's image is missing"
    );
}
