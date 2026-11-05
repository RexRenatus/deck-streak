//! The renderer's own behaviour (SPEC-603 A4, A5 and A6, ADR-624 D3 and D4): a hint opens as a
//! `details` element, every issue class has `DeckStreak`'s own text, and `render` never panics
//! over a fixed-seed population of templates and fields.

#![allow(clippy::print_stdout)]

use std::panic::{AssertUnwindSafe, catch_unwind};

use deck_streak_render::template::{Token, parse, tokenize};
use deck_streak_render::{
    CardContext, CardTemplate, MAX_DEPTH, MAX_TEMPLATE_BYTES, NoteContent, NoteType, NoteTypeKind,
    RenderIssue, check_note_type, render,
};

/// A standard note type with the fields `Prompt` and `Answer` and one template.
fn prompt_answer(front: &str, back: &str) -> NoteType {
    NoteType {
        name: "Basic".to_owned(),
        kind: NoteTypeKind::Standard,
        fields: vec!["Prompt".to_owned(), "Answer".to_owned()],
        templates: vec![CardTemplate {
            name: "Forward".to_owned(),
            front: front.to_owned(),
            back: back.to_owned(),
        }],
    }
}

/// The first card of a note with these two field values.
fn draw(
    note_type: &NoteType,
    prompt: &str,
    answer: &str,
) -> Result<deck_streak_render::Rendered, RenderIssue> {
    let fields = [prompt.to_owned(), answer.to_owned()];
    render(
        note_type,
        &NoteContent {
            fields: &fields,
            tags: &[],
        },
        &CardContext {
            ordinal: 0,
            deck_path: "Languages::Spanish",
            flag: 0,
            card_id: 7,
        },
    )
}

#[test]
fn hint_is_details() {
    let note_type = prompt_answer("{{Prompt}} {{hint:Answer}}", "{{FrontSide}}");
    let card = draw(&note_type, "la casa", "a <b>building</b>").expect("a card");
    assert_eq!(
        card.question,
        "la casa <details class=\"hint\"><summary>Answer</summary>a <b>building</b></details>"
    );
    assert_eq!(card.answer, card.question, "the answer shows the front");
    assert!(!card.question.contains("<script"), "a hint runs no script");
    assert!(!card.question.contains("onclick"), "a hint has no handler");
    let blank = draw(&note_type, "la casa", "  ").expect("a card");
    assert_eq!(
        blank.question, "la casa   ",
        "an empty hint is left as written"
    );
}

/// One case per issue class: the problem, the class it must give, and `DeckStreak`'s text.
fn issue_cases() -> Vec<(Result<(), RenderIssue>, RenderIssue, String)> {
    let first = |note_type: &NoteType, prompt: &str| draw(note_type, prompt, "x").map(|_| ());
    let deep = format!(
        "{}{{{{Prompt}}}}{}",
        "{{#Prompt}}".repeat(MAX_DEPTH + 1),
        "{{/Prompt}}".repeat(MAX_DEPTH + 1)
    );
    let large = "a".repeat(MAX_TEMPLATE_BYTES + 1);
    let mut twice = prompt_answer("{{Prompt}}", "{{Answer}}");
    twice.templates.push(twice.templates[0].clone());
    let mut cloze = prompt_answer("{{Prompt}}", "{{Prompt}}");
    cloze.kind = NoteTypeKind::Cloze;
    let fields = ["a".to_owned(), "b".to_owned()];
    let beyond = render(
        &prompt_answer("{{Prompt}}", "{{Answer}}"),
        &NoteContent {
            fields: &fields,
            tags: &[],
        },
        &CardContext {
            ordinal: 3,
            deck_path: "Default",
            flag: 0,
            card_id: 1,
        },
    )
    .map(|_| ());
    vec![
        (
            first(&prompt_answer("{{Nowhere}}", ""), "a"),
            RenderIssue::UnknownField { name: "Nowhere".to_owned() },
            "The template asks for a field named \"Nowhere\", and this note type has no such field.".to_owned(),
        ),
        (
            first(&prompt_answer("{{#Prompt}}a", ""), "a"),
            RenderIssue::UnclosedSection { name: "Prompt".to_owned() },
            "The section \"Prompt\" is opened in the template but never closed.".to_owned(),
        ),
        (
            first(&prompt_answer("{{Prompt}}{{/Answer}}", ""), "a"),
            RenderIssue::StrayClose { name: "Answer".to_owned() },
            "The template closes a section \"Answer\" that is not open.".to_owned(),
        ),
        (
            first(&prompt_answer("{{Prompt}}", "{{Answer}}"), ""),
            RenderIssue::EmptyFront,
            "Nothing to ask: this card's front uses no field that holds text.".to_owned(),
        ),
        (
            first(&prompt_answer(&deep, ""), "a"),
            RenderIssue::TooDeep,
            format!("The template nests sections more than {MAX_DEPTH} deep."),
        ),
        (
            first(&prompt_answer(&large, ""), "a"),
            RenderIssue::TooLarge,
            format!("The template is longer than {MAX_TEMPLATE_BYTES} bytes."),
        ),
        (
            beyond,
            RenderIssue::NoTemplate { ordinal: 3 },
            "This note type has no card template number 4.".to_owned(),
        ),
        (
            check_note_type(&prompt_answer("plain", "{{Answer}}")),
            RenderIssue::FrontNamesNoField { ordinal: 0 },
            "Card template 1 shows no field on its front.".to_owned(),
        ),
        (
            check_note_type(&twice),
            RenderIssue::DuplicateFront { ordinal: 1 },
            "Card template 2 has the same front as an earlier template.".to_owned(),
        ),
        (
            check_note_type(&cloze),
            RenderIssue::NoClozeFilter,
            "A cloze note type needs a cloze field on both the front and the back of its template.".to_owned(),
        ),
    ]
}

#[test]
fn error_face_is_ds_text() {
    let cases = issue_cases();
    for (got, class, text) in &cases {
        assert_eq!(got, &Err(class.clone()), "the class of {class:?}");
        assert_eq!(&class.message(), text, "the text of {class:?}");
        let escaped = text
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;");
        assert_eq!(
            class.face(),
            format!("<div class=\"ds-render-issue\">{escaped}</div>"),
            "the face of {class:?}"
        );
    }
    let named = RenderIssue::UnknownField {
        name: "<b>&".to_owned(),
    };
    assert_eq!(
        named.face(),
        "<div class=\"ds-render-issue\">The template asks for a field named &quot;&lt;b&gt;&amp;&quot;, and this note type has no such field.</div>",
        "a name is escaped in the face"
    );
    println!("examined {} issue class(es)", cases.len());
    assert_eq!(cases.len(), 10, "every class has its case");
}

/// A fixed-seed generator (splitmix64), so every run draws the same population.
struct Seeded(u64);

impl Seeded {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn below(&mut self, n: usize) -> usize {
        usize::try_from(self.next() % u64::try_from(n).unwrap_or(1)).unwrap_or(0)
    }

    fn text(&mut self, pieces: usize) -> String {
        let count = self.below(pieces + 1);
        (0..count)
            .map(|_| PIECES[self.below(PIECES.len())])
            .collect()
    }
}

/// The pieces a generated template or field is joined from: the language's tags and their
/// parts, field names, filter names, markup, entities, ruby text and cut-off forms.
const PIECES: &[&str] = &[
    "{{",
    "}}",
    "{",
    "}",
    "#",
    "^",
    "/",
    ":",
    " ",
    "\n",
    "\t",
    "Prompt",
    "Answer",
    "FrontSide",
    "Tags",
    "Type",
    "Deck",
    "Subdeck",
    "Card",
    "CardFlag",
    "CardID",
    "c1",
    "c2",
    "text",
    "hint",
    "furigana",
    "kana",
    "kanji",
    "type",
    "tts en_US",
    "cloze",
    "nc",
    "<!--",
    "-->",
    "<b>",
    "</b>",
    "<br>",
    "<div>",
    "</div>",
    "<style>",
    "</style>",
    "<script>",
    "</script>",
    "<",
    ">",
    "[",
    "]",
    "&amp;",
    "&",
    ";",
    "&#x263A;",
    "&#99999999;",
    "&#;",
    "&nbsp;",
    "é",
    "日本[にほん]",
    "[sound:a.mp3]",
    "{{=<% %>=}}",
    "<%",
    "%>",
    "::",
    "\u{a0}",
    "🇳🇴",
];

#[test]
fn render_total_proptest() {
    let mut seed = Seeded(603);
    let mut examined = 0;
    let mut panicked = Vec::new();
    for index in 0..10_000 {
        let templates = (0..=seed.below(2))
            .map(|t| CardTemplate {
                name: format!("T{t}"),
                front: seed.text(40),
                back: seed.text(40),
            })
            .collect();
        let note_type = NoteType {
            name: seed.text(3),
            kind: if seed.below(4) == 0 {
                NoteTypeKind::Cloze
            } else {
                NoteTypeKind::Standard
            },
            fields: vec!["Prompt".to_owned(), "Answer".to_owned(), seed.text(2)],
            templates,
        };
        let fields: Vec<String> = (0..seed.below(4)).map(|_| seed.text(8)).collect();
        let tags: Vec<String> = (0..seed.below(3)).map(|_| seed.text(2)).collect();
        let deck = seed.text(3);
        let ordinal = u16::try_from(seed.below(3)).unwrap_or(0);
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            let _ = check_note_type(&note_type);
            let note = NoteContent {
                fields: &fields,
                tags: &tags,
            };
            let card = CardContext {
                ordinal,
                deck_path: &deck,
                flag: 3,
                card_id: -1,
            };
            let _ = render(&note_type, &note, &card);
            for template in &note_type.templates {
                let _ = tokenize(&template.front);
                let _ = parse(&template.back);
            }
        }));
        examined += 1;
        if outcome.is_err() {
            panicked.push(index);
        }
    }
    println!("examined {examined}");
    assert!(
        panicked.is_empty(),
        "render panicked on generated case(s) {panicked:?}"
    );
    assert_eq!(
        examined, 10_000,
        "every generated case reached the renderer"
    );
}

/// The question of the first card whose one template has this front, with `Prompt` set to
/// `prompt` and `Answer` empty.
fn front_of(front: &str, prompt: &str) -> Result<String, RenderIssue> {
    draw(&prompt_answer(front, ""), prompt, "").map(|card| card.question)
}

#[test]
fn text_filter_drops_comments_styles_scripts_and_tags() {
    let cases = [
        ("a<!--x-->b", "ab"),
        ("a<style>p{}</style>b", "ab"),
        ("a<STYLE>p{}</Style>b", "ab"),
        ("a<SCRIPT>go()</script>b", "ab"),
        ("a<i>b</i>c", "abc"),
        ("&#65;&amp;", "A&"),
        ("&#+65; stays", "&#+65; stays"),
    ];
    for (field, shown) in cases {
        assert_eq!(
            front_of("{{text:Prompt}}", field),
            Ok(shown.to_owned()),
            "the text filter over {field:?}"
        );
    }
}

#[test]
fn ruby_filters_split_each_base_from_its_reading() {
    let front = "{{furigana:Prompt}}|{{kana:Prompt}}|{{kanji:Prompt}}";
    let cases = [
        ("ab[cd]e", "<ruby><rb>ab</rb><rt>cd</rt></ruby>e|cde|abe"),
        (" a[cd]e", "<ruby><rb>a</rb><rt>cd</rt></ruby>e|cde|ae"),
        ("a b[c]", "a<ruby><rb>b</rb><rt>c</rt></ruby>|ac|ab"),
        (
            "x[sound:y.mp3]z",
            "x[sound:y.mp3]z|x[sound:y.mp3]z|x[sound:y.mp3]z",
        ),
        ("[a]", "[a]|[a]|[a]"),
    ];
    for (field, shown) in cases {
        assert_eq!(
            front_of(front, field),
            Ok(shown.to_owned()),
            "the ruby filters over {field:?}"
        );
    }
}

#[test]
fn type_marker_keeps_a_cloze_or_nc_filter() {
    let cases = [
        ("{{Prompt}}{{type:Answer}}", "p[[type:Answer]]"),
        ("{{Prompt}}{{type:cloze:Answer}}", "p[[type:cloze:Answer]]"),
        ("{{Prompt}}{{type:nc:Answer}}", "p[[type:nc:Answer]]"),
        ("{{Prompt}}{{type:text:Answer}}", "p[[type:Answer]]"),
    ];
    for (front, shown) in cases {
        assert_eq!(
            front_of(front, "p"),
            Ok(shown.to_owned()),
            "the front {front:?}"
        );
    }
}

#[test]
fn cloze_number_sections_need_no_field() {
    let front = "{{Prompt}}{{#c1}}z{{/c1}}{{#c2}}x{{/c2}}{{^c2}}y{{/c2}}";
    assert_eq!(
        front_of(front, "p"),
        Ok("pzy".to_owned()),
        "c1 is the first card's number and c2 is no field"
    );
    for name in ["c", "cat", "Nope"] {
        let front = ["{{Prompt}}{{#", name, "}}x{{/", name, "}}"].concat();
        assert_eq!(
            front_of(&front, "p"),
            Err(RenderIssue::UnknownField {
                name: name.to_owned()
            }),
            "a section named {name:?} needs a field"
        );
    }
}

#[test]
fn empty_name_with_a_filter_draws_nothing() {
    assert_eq!(front_of("{{Prompt}}[{{text:}}]", "p"), Ok("p[]".to_owned()));
}

#[test]
fn line_break_before_text_is_not_blank() {
    assert_eq!(
        front_of("{{Prompt}}", "<br>x"),
        Ok("<br>x".to_owned()),
        "text after a line break shows"
    );
    assert_eq!(
        front_of("{{Prompt}}", "<br> <DIV></div>"),
        Err(RenderIssue::EmptyFront),
        "line breaks and empty div tags show nothing"
    );
}

#[test]
fn equal_fronts_that_show_the_card_name_pass_the_check() {
    let twice = |front: &str| NoteType {
        name: "Basic".to_owned(),
        kind: NoteTypeKind::Standard,
        fields: vec!["Prompt".to_owned(), "Answer".to_owned()],
        templates: ["Forward", "Again"]
            .into_iter()
            .map(|name| CardTemplate {
                name: name.to_owned(),
                front: front.to_owned(),
                back: "{{FrontSide}}".to_owned(),
            })
            .collect(),
    };
    assert_eq!(
        check_note_type(&twice("{{Card}}")),
        Ok(()),
        "the card name tells the fronts apart"
    );
    assert_eq!(
        check_note_type(&twice("{{Prompt}}")),
        Err(RenderIssue::DuplicateFront { ordinal: 1 }),
        "two fronts that read the same"
    );
}

#[test]
fn tokenize_takes_the_earlier_of_a_comment_and_a_tag() {
    assert_eq!(
        tokenize("<!--x-->{{Prompt}}"),
        vec![Token::Comment("x"), Token::Var("Prompt")]
    );
    assert_eq!(
        tokenize("{{Prompt}}<!--x-->"),
        vec![Token::Var("Prompt"), Token::Comment("x")]
    );
    assert_eq!(
        tokenize("<!--{{Nope}}-->"),
        vec![Token::Comment("{{Nope}}")]
    );
    assert_eq!(tokenize("{{a}}x"), vec![Token::Var("a"), Token::Text("x")]);
}

#[test]
fn tokenize_opens_a_section_only_past_one_character() {
    assert_eq!(
        tokenize("{{#a}}x{{/a}}"),
        vec![
            Token::Open {
                inverted: false,
                name: "a"
            },
            Token::Text("x"),
            Token::Close("a"),
        ]
    );
    assert_eq!(tokenize("{{#}}"), vec![Token::Var("#")]);
}

#[test]
fn parse_takes_a_template_of_exactly_the_byte_bound() {
    assert!(
        parse(&"x".repeat(MAX_TEMPLATE_BYTES)).is_ok(),
        "a template of exactly the bound parses"
    );
    assert_eq!(
        parse(&"x".repeat(MAX_TEMPLATE_BYTES + 1)).err(),
        Some(RenderIssue::TooLarge),
        "one byte over the bound is too large"
    );
}
