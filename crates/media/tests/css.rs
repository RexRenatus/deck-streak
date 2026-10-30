//! `scan_css` keeps every font case of the engine core's face suite (SPEC-513 A5): for each CSS
//! the suite's font tests give a note type, the fonts `scan_css` names are the fonts the face's
//! font pass inlines or empties, in the order written, and a URL the pass leaves as written for
//! its scheme, its `data:` form or its path is no name. The census below holds the suite at its 18
//! tests and finds every CSS line copied here in it, so a font case added or changed there fails
//! here until it is copied.

#![allow(clippy::expect_used, clippy::print_stdout)]

use std::fs;
use std::path::Path;

use deck_streak_media::scan_css;

/// The face suite whose font cases these are.
const FACE_SUITE: &str = "crates/engine-core/tests/face.rs";

/// The suite's test count when these cases were copied.
const FACE_TESTS: usize = 18;

/// The font table of the face's font pass (`crates/engine-core/src/media.rs`, `FONT_TYPES`).
const FONT_EXTENSIONS: [&str; 4] = ["ttf", "otf", "woff", "woff2"];

/// One font case: the test it comes from, its CSS line by line, the fonts the font pass reads in
/// the order written, and every name `scan_css` gives.
struct FontCase {
    test: &'static str,
    css: &'static [&'static str],
    fonts: &'static [&'static str],
    names: &'static [&'static str],
}

/// The CSS of the suite's `FONT_CSS`.
const FONT_CSS: &[&str] = &[
    "@font-face { font-family: p; src: url(\"_parity.ttf\"); }\n",
    "@font-face { font-family: s; src: URL( '_parity.ttf' ); }\n",
    ".card { font-family: p, s; }\n",
];

/// Every font case of the suite.
const CASES: [FontCase; 5] = [
    FontCase {
        test: "a_font_the_css_names_is_inlined_for_a_reader_that_asks",
        css: FONT_CSS,
        fonts: &["_parity.ttf", "_parity.ttf"],
        names: &["_parity.ttf", "_parity.ttf"],
    },
    FontCase {
        test: "a_refused_font_is_emptied_and_named_and_any_other_url_stays",
        css: &[
            "@font-face { font-family: big; src: url(\"big.ttf\"); }\n",
            "@font-face { font-family: absent; src: url('absent.woff'); }\n",
            "@font-face { font-family: again; src: url(big.ttf); }\n",
            "@font-face { font-family: remote; src: url(\"https://example.invalid/r.woff2\"); }\n",
            "@font-face { font-family: inline; src: url(\"data:font/ttf;base64,eA==\"); }\n",
            "@font-face { font-family: nested; src: url(\"fonts/n.otf\"); }\n",
            ".card { background: url(\"dot.png\"); }\n",
        ],
        fonts: &["big.ttf", "absent.woff", "big.ttf"],
        names: &["big.ttf", "absent.woff", "big.ttf", "dot.png"],
    },
    FontCase {
        test: "the_css_stays_as_written_for_a_reader_that_does_not_ask",
        css: FONT_CSS,
        fonts: &["_parity.ttf", "_parity.ttf"],
        names: &["_parity.ttf", "_parity.ttf"],
    },
    FontCase {
        test: "a_font_never_crowds_out_the_cards_own_media",
        css: &["@font-face { font-family: one; src: url(\"_one.ttf\"); }\n"],
        fonts: &["_one.ttf"],
        names: &["_one.ttf"],
    },
    FontCase {
        test: "every_font_type_is_inlined_under_its_own_type",
        css: &[
            "@font-face { font-family: a; src: url(\"_a.ttf\"); }\n",
            "@font-face { font-family: b; src: url(\"_b.OTF\"); }\n",
            "@font-face { font-family: c; src: url(\"_c.woff\"); }\n",
            "@font-face { font-family: d; src: url(\"_d.Woff2\"); }\n",
        ],
        fonts: &["_a.ttf", "_b.OTF", "_c.woff", "_d.Woff2"],
        names: &["_a.ttf", "_b.OTF", "_c.woff", "_d.Woff2"],
    },
];

/// Whether a name's extension is in the font table, in any ASCII case.
fn font(name: &str) -> bool {
    name.rsplit_once('.').is_some_and(|(_, extension)| {
        FONT_EXTENSIONS
            .iter()
            .any(|known| known.eq_ignore_ascii_case(extension))
    })
}

#[test]
fn scan_css_keeps_every_font_case_of_the_face_suite() {
    for case in &CASES {
        let css = case.css.concat();
        let names = scan_css(&css);
        assert_eq!(names, case.names.to_vec(), "{}: every name", case.test);
        let fonts: Vec<&str> = names
            .iter()
            .map(String::as_str)
            .filter(|name| font(name))
            .collect();
        assert_eq!(fonts, case.fonts.to_vec(), "{}: the fonts", case.test);
    }
    println!("examined {} font case(s)", CASES.len());
}

#[test]
fn the_face_suite_still_holds_its_tests_and_every_font_case_copied_here() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let suite = fs::read_to_string(root.join(FACE_SUITE)).expect("the face suite");
    let tests = suite
        .lines()
        .filter(|line| line.trim() == "#[test]")
        .count();
    assert_eq!(tests, FACE_TESTS, "{FACE_SUITE} holds its 18 tests");
    let mut lines = 0;
    for case in &CASES {
        assert!(
            suite.contains(&format!("fn {}()", case.test)),
            "{FACE_SUITE} defines {}",
            case.test
        );
        for line in case.css {
            assert!(
                suite.contains(&format!("{line:?}")),
                "{FACE_SUITE} writes {line:?}"
            );
            lines += 1;
        }
    }
    println!("examined {tests} test(s) and {lines} CSS line(s) of {FACE_SUITE}");
}

#[test]
fn a_url_that_names_no_plain_file_is_no_name() {
    let css = concat!(
        "a { b: url(\"https://example.invalid/x.png\"); }\n",
        "a { b: url(data:font/ttf;base64,eA==); }\n",
        "a { b: url('fonts/n.otf'); }\n",
        "a { b: url(\"..\"); }\n",
        "a { b: url(\"%ff.png\"); }\n",
        "a { b: url(  \"spaced%20name.png\"  ); }\n",
        "a { b: Url(plain.png); }\n",
        "a { b: url(unclosed.png",
    );
    assert_eq!(scan_css(css), vec!["spaced name.png", "plain.png"]);
}
