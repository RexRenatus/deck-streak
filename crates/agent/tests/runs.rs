//! The `agent_runs` prune reads `created_at` through an index (SPEC-043 A19, A20, issue 363).
#![allow(clippy::expect_used)]

use deck_streak_kernel::Db;
use proc_macro2::{Delimiter, Group, Ident, Literal, TokenStream, TokenTree};

/// The index migration `004302` creates, named like the ledger's other `*_by_*` indexes.
const INDEX: &str = "agent_runs_by_created_at";

/// The one place this file writes the prune statement out: the tested statement and the text
/// of its `EXPLAIN QUERY PLAN` are both made from it, so neither can drift from the other.
macro_rules! prune_statement {
    () => {
        "DELETE FROM agent_runs WHERE created_at < ?1"
    };
}

/// The statement `AgentRuns::prune_before` runs, as `crates/agent/src/runs.rs` states it; the test
/// below reads the source and asserts it holds this exact text.
const PRUNE: &str = prune_statement!();

/// The table the prune deletes from, named apart so the fixtures below never write the statement.
const TABLE: &str = "agent_runs";

/// `text` lower-cased with every run of whitespace collapsed to one space, so a statement cannot
/// hide behind a different case or a wider gap.
fn normalized(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// `source` as rustc's lexer reads it (SPEC-043 R16b): no comment is a token, a doc comment is a
/// `doc` attribute, a literal of any kind is one token whatever it holds, and whitespace is what
/// separates tokens. A source that does not lex, an unclosed delimiter in it among others, is an
/// error, and the pin refuses it.
fn tokens_of(source: &str) -> Result<Vec<TokenTree>, String> {
    source
        .parse::<TokenStream>()
        .map(|stream| stream.into_iter().collect())
        .map_err(|error| error.to_string())
}

/// The tokens a delimited group holds.
fn tokens_in(group: &Group) -> Vec<TokenTree> {
    group.stream().into_iter().collect()
}

/// An identifier as rustc resolves it: `r#delete` is `delete`.
fn name_of(ident: &Ident) -> String {
    let name = ident.to_string();
    name.strip_prefix("r#")
        .map_or_else(|| name.clone(), str::to_owned)
}

/// The text rustc cooks from a string, byte-string or C-string literal, raw or not: escapes and
/// line continuations are applied, a raw form is read as written. `None` for any other literal.
fn text_of(literal: &Literal) -> Option<String> {
    match syn::Lit::new(literal.clone()) {
        syn::Lit::Str(text) => Some(text.value()),
        syn::Lit::ByteStr(bytes) => Some(String::from_utf8_lossy(&bytes.value()).into_owned()),
        syn::Lit::CStr(text) => Some(text.value().to_string_lossy().into_owned()),
        _ => None,
    }
}

/// Whether `token` is the punctuation `ch`.
fn is_punct(token: Option<&TokenTree>, ch: char) -> bool {
    matches!(token, Some(TokenTree::Punct(punct)) if punct.as_char() == ch)
}

/// Whether `group`, after the tokens `before`, is a doc attribute: `#[doc ..]` or `#![doc ..]`,
/// which every doc comment becomes. Its text is prose that runs nothing.
fn is_doc_attribute(before: &[TokenTree], group: &Group) -> bool {
    let hash = match before {
        [.., before_bang, _] if is_punct(before.last(), '!') => is_punct(Some(before_bang), '#'),
        _ => is_punct(before.last(), '#'),
    };
    group.delimiter() == Delimiter::Bracket
        && hash
        && matches!(tokens_in(group).first(), Some(TokenTree::Ident(doc)) if name_of(doc) == "doc")
}

/// Every text the code in `tokens` writes, at any depth: each identifier and each string-like
/// literal's cooked text. A comment is no token, and a doc attribute's text is not read.
fn code_texts(tokens: &[TokenTree], texts: &mut Vec<String>) {
    for (at, token) in tokens.iter().enumerate() {
        match token {
            TokenTree::Ident(ident) => texts.push(name_of(ident)),
            TokenTree::Literal(literal) => texts.extend(text_of(literal)),
            TokenTree::Group(group) if !is_doc_attribute(&tokens[..at], group) => {
                code_texts(&tokens_in(group), texts);
            }
            TokenTree::Group(_) | TokenTree::Punct(_) => {}
        }
    }
}

/// Whether the tokens `before` end in `sqlx::query!`, each identifier as rustc resolves it.
fn ends_in_the_query_macro(before: &[TokenTree]) -> bool {
    let is = |token: &TokenTree, text: &str| match token {
        TokenTree::Ident(ident) => name_of(ident) == text,
        TokenTree::Punct(punct) => punct.as_char().to_string() == text,
        TokenTree::Group(_) | TokenTree::Literal(_) => false,
    };
    before.len() >= 5
        && ["sqlx", ":", ":", "query", "!"]
            .iter()
            .zip(&before[before.len() - 5..])
            .all(|(text, token)| is(token, text))
}

/// Whether the macro arguments `group` open with the tested statement: a string literal whose
/// cooked text is the statement, alone or before a comma.
fn opens_with_the_statement(group: &Group) -> bool {
    match tokens_in(group).as_slice() {
        [TokenTree::Literal(statement), rest @ ..] => {
            matches!(syn::Lit::new(statement.clone()), syn::Lit::Str(text) if text.value() == PRUNE)
                && match rest.first() {
                    None => true,
                    Some(TokenTree::Punct(comma)) => comma.as_char() == ',',
                    Some(_) => false,
                }
        }
        _ => false,
    }
}

/// How many times `tokens`, at any depth, hand the tested statement to `sqlx::query!`: a copy in
/// a comment or inside any literal is no call, and a spelling rustc reads as the same call (spaces
/// in the path, any delimiter, any string form) is one.
fn statements_run_in(tokens: &[TokenTree]) -> usize {
    tokens
        .iter()
        .enumerate()
        .map(|(at, token)| match token {
            TokenTree::Group(group) => {
                statements_run_in(&tokens_in(group))
                    + usize::from(
                        ends_in_the_query_macro(&tokens[..at]) && opens_with_the_statement(group),
                    )
            }
            TokenTree::Ident(_) | TokenTree::Punct(_) | TokenTree::Literal(_) => 0,
        })
        .sum()
}

/// What is wrong with a prune's source (SPEC-043 R16b): an empty list when its tokens hand the
/// tested statement to `sqlx::query!` exactly once and its code writes the word delete exactly
/// once, as an identifier or in any literal's cooked text.
fn prune_pin_problems(source: &str) -> Vec<String> {
    let tokens = match tokens_of(source) {
        Ok(tokens) => tokens,
        Err(error) => return vec![format!("the source does not lex as Rust: {error}")],
    };
    let mut problems = Vec::new();
    let run = statements_run_in(&tokens);
    if run != 1 {
        problems.push(format!(
            "the code hands the statement to sqlx::query! {run} times, not once"
        ));
    }
    let mut texts = Vec::new();
    code_texts(&tokens, &mut texts);
    let keywords: usize = texts
        .iter()
        .map(|text| {
            text.split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
                .filter(|word| word.eq_ignore_ascii_case("delete"))
                .count()
        })
        .sum();
    if keywords != 1 {
        problems.push(format!(
            "the source writes the word delete {keywords} times, not once"
        ));
    }
    problems
}

/// What is wrong with this test file's own text (SPEC-043 A22): an empty list when exactly one
/// literal holds a delete from the run table in its cooked text, the macro's, so the plan's text is
/// derived from it and no changed copy can stand beside it.
fn own_statement_problems(test_source: &str) -> Vec<String> {
    let mut texts = Vec::new();
    if let Ok(tokens) = tokens_of(test_source) {
        code_texts(&tokens, &mut texts);
    }
    let statements: usize = texts
        .iter()
        .map(|text| {
            normalized(text)
                .matches(&format!("delete from {TABLE}"))
                .count()
        })
        .sum();
    if statements == 1 {
        Vec::new()
    } else {
        vec![format!(
            "the test file writes the statement {statements} times, not once"
        )]
    }
}

async fn open() -> (tempfile::TempDir, Db) {
    let dir = tempfile::tempdir().expect("a directory");
    let db = Db::open(&dir.path().join("t.db"))
        .await
        .expect("a database");
    (dir, db)
}

#[tokio::test]
async fn agent_runs_is_indexed_on_created_at() {
    let (_dir, db) = open().await;
    let columns: Vec<(String,)> =
        sqlx::query_as("SELECT name FROM pragma_index_info(?1) ORDER BY seqno")
            .bind(INDEX)
            .fetch_all(db.reader())
            .await
            .expect("the index's columns");
    assert_eq!(columns, [("created_at".to_owned(),)]);
    let table: Option<(String,)> =
        sqlx::query_as("SELECT tbl_name FROM sqlite_master WHERE type = 'index' AND name = ?1")
            .bind(INDEX)
            .fetch_optional(db.reader())
            .await
            .expect("the schema");
    assert_eq!(table, Some(("agent_runs".to_owned(),)));
}

#[tokio::test]
async fn the_prune_reads_agent_runs_through_the_created_at_index() {
    let (_dir, db) = open().await;
    let plan: Vec<(i64, i64, i64, String)> =
        sqlx::query_as(concat!("EXPLAIN QUERY PLAN ", prune_statement!()))
            .bind(0_i64)
            .fetch_all(db.reader())
            .await
            .expect("the plan");
    let details: Vec<&str> = plan.iter().map(|row| row.3.as_str()).collect();
    assert!(!details.is_empty(), "the planner returned no step");
    assert!(
        details
            .iter()
            .any(|d| d.contains("SEARCH agent_runs USING INDEX agent_runs_by_created_at")),
        "the prune does not use the index: {details:?}"
    );
    assert!(
        !details.iter().any(|d| d.starts_with("SCAN")),
        "the prune scans the table: {details:?}"
    );
    // The statement as the cooked text of the one literal handed to `sqlx::query!`, so an added
    // predicate (`... < ?1 OR ...`) is another statement, never a superstring that still matches.
    assert_eq!(
        prune_pin_problems(include_str!("../src/runs.rs")),
        Vec::<String>::new()
    );
    assert_eq!(
        own_statement_problems(include_str!("runs.rs")),
        Vec::<String>::new()
    );
}

/// A prune source as `runs.rs` states it: the one statement, quoted whole.
fn a_good_prune_source() -> String {
    format!("let done = sqlx::query!(\"{PRUNE}\", cutoff);")
}

#[test]
fn a_comment_quoting_the_prune_beside_a_prune_that_skips_the_index_is_refused() {
    assert_eq!(
        prune_pin_problems(&a_good_prune_source()),
        Vec::<String>::new()
    );
    // The quoted copy sits in a comment; the statement that runs wraps the column in an
    // expression, which the index cannot seek.
    let decoy = format!(
        "// \"{PRUNE}\"\nlet done = sqlx::query!(\"DELETE FROM {TABLE} WHERE created_at + 0 < ?1\", cutoff);"
    );
    let problems = prune_pin_problems(&decoy);
    assert!(
        !problems.is_empty(),
        "the decoy is not refused: {problems:?}"
    );
}

#[test]
fn a_second_delete_statement_is_refused_however_it_is_spelled() {
    assert_eq!(
        prune_pin_problems(&a_good_prune_source()),
        Vec::<String>::new()
    );
    let decoy = format!(
        "{}\nsqlx::query(\"delete  from {TABLE}\");",
        a_good_prune_source()
    );
    let problems = prune_pin_problems(&decoy);
    assert!(
        !problems.is_empty(),
        "the second statement is not refused: {problems:?}"
    );
}

#[test]
fn a_changed_copy_of_the_statement_in_the_plan_string_is_refused() {
    let one_literal = format!(
        "macro_rules! prune_statement {{ () => {{ \"DELETE FROM {TABLE} WHERE created_at < ?1\" }}; }}\n\
         sqlx::query_as(concat!(\"EXPLAIN QUERY PLAN \", prune_statement!()))"
    );
    assert_eq!(own_statement_problems(&one_literal), Vec::<String>::new());
    let decoy = format!(
        "{one_literal}\nsqlx::query_as(\"EXPLAIN QUERY PLAN DELETE FROM {TABLE} WHERE created_at <= ?1\")"
    );
    let problems = own_statement_problems(&decoy);
    assert!(
        !problems.is_empty(),
        "the changed copy is not refused: {problems:?}"
    );
}

#[test]
fn a_prune_spelled_around_the_keyword_scan_beside_a_quoted_copy_is_refused() {
    assert_eq!(
        prune_pin_problems(&a_good_prune_source()),
        Vec::<String>::new()
    );
    // Each decoy keeps the tested statement quoted once, in a comment or handed to a
    // `sqlx::query!` that never runs, and runs a prune the index cannot seek, spelled so that the
    // plain statement text never appears in it: the table is schema-qualified or quoted, split
    // across two literals, or an SQL comment sits between the keywords. Beside the copy that is
    // handed to `sqlx::query!`, only the word count sees the prune that runs.
    let copies = [
        format!("// \"{PRUNE}\""),
        format!("let _unused = sqlx::query!(\"{PRUNE}\", cutoff);"),
    ];
    let spellings = [
        format!("DELETE FROM main.{TABLE} WHERE created_at + 0 < ?1"),
        format!("DELETE FROM \\\"{TABLE}\\\" WHERE created_at + 0 < ?1"),
        "DELETE FROM agent\" + \"_runs WHERE created_at + 0 < ?1".to_owned(),
        format!("DELETE/**/FROM {TABLE} WHERE created_at + 0 < ?1"),
    ];
    for spelling in &spellings {
        for copy in &copies {
            let decoy = format!("{copy}\nlet done = sqlx::query!(\"{spelling}\", cutoff);");
            assert_eq!(
                decoy.matches(&format!("\"{PRUNE}\"")).count(),
                1,
                "the decoy does not quote the tested statement once: {decoy}"
            );
            let problems = prune_pin_problems(&decoy);
            assert!(
                !problems.is_empty(),
                "the decoy is not refused: {spelling}: {problems:?}"
            );
        }
        let beside_a_query = format!(
            "{}\nlet done = sqlx::query!(\"{spelling}\", cutoff);",
            copies[1]
        );
        assert_eq!(
            prune_pin_problems(&beside_a_query),
            ["the source writes the word delete 2 times, not once"],
            "the word count did not refuse the prune beside a copy handed to sqlx::query!: {spelling}"
        );
    }
}

#[test]
fn prose_that_names_the_delete_beside_the_prune_is_not_counted() {
    let good = a_good_prune_source();
    // A comment or doc line that names the prune in words is prose, whatever it holds: it runs
    // nothing, so the good source stays good beside it, a quote in it included.
    for prose in [
        "/// The delete reads `created_at` through its index.",
        "// One DELETE, so the index on created_at is used.",
        "// The \"prune\" is one delete, through the index on created_at.",
    ] {
        assert_eq!(
            prune_pin_problems(&format!("{prose}\n{good}")),
            Vec::<String>::new(),
            "prose was counted: {prose}"
        );
    }
    // A comment runs nothing, a copy of the statement in it included: beside the good prune it
    // changes nothing.
    let copy = format!("/// DELETE FROM {TABLE} WHERE created_at < ?1\n{good}");
    assert_eq!(
        prune_pin_problems(&copy),
        Vec::<String>::new(),
        "a commented copy of the statement was read as code"
    );
    // A comment that quotes the statement is not the statement that runs, so beside a changed
    // prune the code hands the tested statement to nothing.
    let decoy = format!(
        "// \"{PRUNE}\"\nlet done = sqlx::query!(\"DELETE FROM main.{TABLE} WHERE created_at + 0 < ?1\", cutoff);"
    );
    let problems = prune_pin_problems(&decoy);
    assert!(
        !problems.is_empty(),
        "a quoting comment was read as the prune: {problems:?}"
    );
}

/// Every form a Rust comment takes, each wrapping `text`: line, outer doc, inner doc, block, outer
/// doc block, inner doc block, a block nested in a block, and a block over several lines.
fn every_comment_form(text: &str) -> Vec<String> {
    vec![
        format!("// {text}"),
        format!("/// {text}"),
        format!("//! {text}"),
        format!("/* {text} */"),
        format!("/** {text} */"),
        format!("/*! {text} */"),
        format!("/* /* {text} */ */"),
        format!("/*\n {text}\n*/"),
    ]
}

#[test]
fn a_delete_word_or_the_statement_in_any_comment_form_is_not_counted() {
    let good = a_good_prune_source();
    // Two prose texts, one holding a quote, in each of the eight forms: none is read.
    let forms: Vec<String> = [
        "The delete reads created_at through its index.",
        "The \"prune\" is one delete, through the index on created_at.",
    ]
    .into_iter()
    .flat_map(every_comment_form)
    .collect();
    assert_eq!(forms.len(), 16, "the population changed");
    eprintln!("members: {} comment forms", forms.len());
    for comment in &forms {
        assert_eq!(
            prune_pin_problems(&format!("{comment}\n{good}")),
            Vec::<String>::new(),
            "a benign delete word in a comment was counted: {comment}"
        );
    }
    // The statement itself inside each form, bare or quoted, is prose too: no comment is code.
    for text in [PRUNE.to_owned(), format!("\"{PRUNE}\"")] {
        for comment in every_comment_form(&text) {
            assert_eq!(
                prune_pin_problems(&format!("{comment}\n{good}")),
                Vec::<String>::new(),
                "a commented copy of the statement was read as code: {comment}"
            );
        }
    }
    // Comment markers inside a string or a character literal open no comment: a prune written
    // after them on the same line is still read.
    for opener in ["\"// \"", "\"/* \"", "'\"'", "r#\"//\"#"] {
        let decoy = format!(
            "let _ = {opener}; let done = sqlx::query!(\"DELETE FROM main.{TABLE} WHERE created_at + 0 < ?1\", cutoff);\n{good}"
        );
        assert!(
            !prune_pin_problems(&decoy).is_empty(),
            "a comment marker in a literal hid a prune: {opener}"
        );
    }
}

/// Every place the tested statement can be written without being the statement that runs: each
/// comment form, bare and quoted, and each literal in code that `sqlx::query!` does not take.
fn every_copy_that_does_not_run() -> Vec<String> {
    let quoted = format!("\"{PRUNE}\"");
    let mut copies = every_comment_form(PRUNE);
    copies.extend(every_comment_form(&quoted));
    copies.extend([
        format!("const _PRUNE: &str = {quoted};"),
        format!("static _PRUNE: &str = {quoted};"),
        format!("#[doc = {quoted}]"),
        format!("let _prune = {quoted};"),
        format!("let _prune = concat!({quoted});"),
    ]);
    copies
}

#[test]
fn a_copy_of_the_statement_that_does_not_run_is_not_the_tested_statement() {
    assert_eq!(
        prune_pin_problems(&a_good_prune_source()),
        Vec::<String>::new()
    );
    // Prunes that run a statement the source does not write: one from another module, one from
    // a file, and one read by `query_file!`. Each can skip the index while a copy stays behind.
    let runs = [
        "let done = sqlx::query(crate::prune_sql::PRUNE).bind(cutoff);",
        "let done = sqlx::query(include_str!(\"../queries/prune.sql\")).bind(cutoff);",
        "let done = sqlx::query_file!(\"queries/prune.sql\", cutoff);",
    ];
    let copies = every_copy_that_does_not_run();
    assert_eq!(copies.len(), 21, "the population changed");
    let mut members = 0;
    let none = "the code hands the statement to sqlx::query! 0 times, not once".to_owned();
    for run in runs {
        for copy in &copies {
            let problems = prune_pin_problems(&format!("{copy}\n{run}"));
            assert!(
                problems.contains(&none),
                "a copy that does not run was read as the prune: {copy}: {problems:?}"
            );
            members += 1;
        }
    }
    assert_eq!(members, 63, "the population changed");
    eprintln!("members: {members} copies-by-runs");
}

#[test]
fn no_character_literal_hides_a_second_statement_in_the_string_after_it() {
    let good = a_good_prune_source();
    // A string after the literal holds comment markers around a second statement. A literal the
    // scan measures wrongly lets that string's quote open a string outside it, and the markers
    // then hide the statement from the word count.
    let hidden = format!(
        "sqlx::query(\"/* /* */ DELETE FROM main.{TABLE} WHERE created_at + 0 < ?1 -- */\");"
    );
    let literals = [
        "'\"'",
        "'\\''",
        "'\\\"'",
        "'\\\\'",
        "b'\\''",
        "b'\"'",
        "'\\x27'",
        "'\\x22'",
        "'\\u{27}'",
        "'\\u{22}'",
        "'\\n'",
        "'/'",
        "'*'",
    ];
    assert_eq!(literals.len(), 13, "the population changed");
    eprintln!("members: {} character literals", literals.len());
    for literal in literals {
        let before = format!("let _ = ({literal},'\"');\n{good}");
        assert_eq!(
            prune_pin_problems(&before),
            Vec::<String>::new(),
            "the literal alone was refused: {literal}"
        );
        assert_eq!(
            prune_pin_problems(&format!("{before}\n{hidden}")),
            ["the source writes the word delete 2 times, not once"],
            "a character literal hid a second statement: {literal}"
        );
    }
}

/// The prune the tests set beside a holder: the good one, the statement handed whole to
/// `sqlx::query!`, and one that runs a statement written in another module, so that only a copy
/// can make the source look like the prune.
fn the_two_prunes() -> [String; 2] {
    [
        a_good_prune_source(),
        "let done = sqlx::query(crate::prune_sql::INDEX_SKIPPING).bind(cutoff);".to_owned(),
    ]
}

/// The string-like literal kinds rustc lexes as one token whatever they hold, each an opener, a
/// closer and the type of a constant holding it: a string, raw strings with none to three hashes,
/// a byte string and a raw one, a C string and a raw one.
const LITERAL_KINDS: [(&str, &str, &str); 11] = [
    ("\"", "\"", "&str"),
    ("r\"", "\"", "&str"),
    ("r#\"", "\"#", "&str"),
    ("r##\"", "\"##", "&str"),
    ("r###\"", "\"###", "&str"),
    ("b\"", "\"", "&[u8]"),
    ("br\"", "\"", "&[u8]"),
    ("br#\"", "\"#", "&[u8]"),
    ("c\"", "\"", "&core::ffi::CStr"),
    ("cr\"", "\"", "&core::ffi::CStr"),
    ("cr#\"", "\"#", "&core::ffi::CStr"),
];

/// `text` written as the literal `open`..`close`: escaped where the kind escapes, as written where
/// it is raw, or `None` where a raw kind cannot hold `text` because `text` holds its closer.
fn literal_holding(open: &str, close: &str, text: &str) -> Option<String> {
    if open.trim_start_matches(['b', 'c']).starts_with('r') {
        (!text.contains(close)).then(|| format!("{open}{text}{close}"))
    } else {
        let escaped = text.replace('\\', "\\\\").replace('"', "\\\"");
        Some(format!("{open}{escaped}{close}"))
    }
}

/// Where a holder closes against the code after it: a holder on one line of its own; one that
/// spans lines and closes on the line before; one that spans lines and closes on the code's own
/// line. Each is the text before and after the held text, then what separates the holder from
/// the code.
const CLOSES: [(&str, &str, &str); 3] = [("", "", "\n"), ("\n", "\n", "\n"), ("\n", "\n", " ")];

/// Every comment kind rustc lexes, as an opener and a closer: line, a line of four slashes, outer
/// doc, inner doc, block, nested block, outer doc block and inner doc block. A line kind holds one
/// line and closes where the line ends.
const COMMENT_KINDS: [(&str, &str); 8] = [
    ("// ", ""),
    ("//// ", ""),
    ("/// ", ""),
    ("//! ", ""),
    ("/* ", " */"),
    ("/* /* ", " */ */"),
    ("/** ", " */"),
    ("/*! ", " */"),
];

/// A copy of the call, the way the prune writes it.
fn a_copy_of_the_call() -> String {
    format!("sqlx::query!(\"{PRUNE}\", cutoff)")
}

/// Every literal holding a copy of the call, of every kind that can hold it, as a statement, a
/// constant, a static or a doc attribute, closing in each place `CLOSES` names.
fn every_literal_holding_the_call() -> Vec<String> {
    let mut holders = Vec::new();
    for (open, close, kind) in LITERAL_KINDS {
        for (before, after, separator) in CLOSES {
            let text = format!("{before}{}{after}", a_copy_of_the_call());
            let Some(literal) = literal_holding(open, close, &text) else {
                continue;
            };
            holders.push(format!("let _ = {literal};{separator}"));
            holders.push(format!("const _COPY: {kind} = {literal};{separator}"));
            holders.push(format!("static _COPY: {kind} = {literal};{separator}"));
            if kind == "&str" {
                holders.push(format!(
                    "#[doc = {literal}]\nfn _documented() {{}}{separator}"
                ));
            }
        }
    }
    holders
}

#[test]
fn a_literal_of_any_kind_holding_the_call_is_not_the_call() {
    // Beside the good prune, a doc attribute is prose and changes nothing, and any other literal's
    // word delete is still counted: such a literal can be handed to `sqlx::query` and run, so the
    // source is refused on the count, never on the call. Beside a prune written elsewhere, the
    // literal is no call, so the code hands the statement to none.
    let holders = every_literal_holding_the_call();
    assert_eq!(holders.len(), 84, "the population changed");
    let [good, elsewhere] = the_two_prunes();
    let counted = ["the source writes the word delete 2 times, not once"];
    let mut members = 0;
    for holder in &holders {
        let expected: &[&str] = if holder.starts_with("#[doc") {
            &[]
        } else {
            &counted
        };
        assert_eq!(
            prune_pin_problems(&format!("{holder}{good}")),
            expected,
            "a literal holding the call was not counted, or was read as the call: {holder}"
        );
        let problems = prune_pin_problems(&format!("{holder}{elsewhere}"));
        assert!(
            problems.contains(
                &"the code hands the statement to sqlx::query! 0 times, not once".to_owned()
            ),
            "a literal holding the call was read as the call: {holder}: {problems:?}"
        );
        members += 2;
    }
    assert_eq!(members, 168, "the population changed");
    eprintln!("examined {members} literal holders");
}

/// Every comment holding a copy of the call or the bare statement, of every kind, closing in each
/// place `CLOSES` names that the kind can close.
fn every_comment_holding_the_statement() -> Vec<String> {
    let mut holders = Vec::new();
    for (open, close) in COMMENT_KINDS {
        for text in [a_copy_of_the_call(), PRUNE.to_owned()] {
            for (before, after, separator) in CLOSES {
                if close.is_empty() && !before.is_empty() {
                    continue;
                }
                let separator = if close.is_empty() { "\n" } else { separator };
                holders.push(format!("{open}{before}{text}{after}{close}{separator}"));
            }
        }
    }
    holders
}

#[test]
fn a_comment_of_any_kind_holding_the_statement_is_not_read() {
    // A comment runs nothing, whatever it holds: beside the good prune it changes nothing, and
    // beside a prune written elsewhere it is no call.
    let holders = every_comment_holding_the_statement();
    assert_eq!(holders.len(), 32, "the population changed");
    let [good, elsewhere] = the_two_prunes();
    let mut members = 0;
    for holder in &holders {
        assert_eq!(
            prune_pin_problems(&format!("{holder}{good}")),
            Vec::<String>::new(),
            "a comment was read as code: {holder}"
        );
        let problems = prune_pin_problems(&format!("{holder}{elsewhere}"));
        assert!(
            problems.contains(
                &"the code hands the statement to sqlx::query! 0 times, not once".to_owned()
            ),
            "a comment was read as the call: {holder}: {problems:?}"
        );
        members += 2;
    }
    assert_eq!(members, 64, "the population changed");
    eprintln!("examined {members} comment holders");
}

/// Rustc's whitespace, `Pattern_White_Space`: the eleven code points its lexer skips between
/// tokens. NBSP and U+3000 are not among them, and rustc refuses either between tokens.
const RUSTC_WHITESPACE: [char; 11] = [
    '\t', '\n', '\u{b}', '\u{c}', '\r', ' ', '\u{85}', '\u{200e}', '\u{200f}', '\u{2028}',
    '\u{2029}',
];

/// Every spelling of the good call that rustc reads as the same tokens: each whitespace code point
/// in each gap between the tokens the pin reads, then each path spelling with each literal
/// spelling of the statement (raw with none to three hashes, an escape, a line continuation).
fn every_spelling_of_the_call() -> Vec<String> {
    let mut spellings = Vec::new();
    let pieces = [
        "let done = sqlx".to_owned(),
        "::".to_owned(),
        "query".to_owned(),
        "!".to_owned(),
        "(".to_owned(),
        format!("\"{PRUNE}\""),
        ", cutoff);".to_owned(),
    ];
    for w in RUSTC_WHITESPACE {
        for gap in 1..pieces.len() {
            let mut spelling = pieces[..gap].concat();
            spelling.push(w);
            spelling.push_str(&pieces[gap..].concat());
            spellings.push(spelling);
        }
    }
    let paths = [
        "sqlx::query!(@)",
        "sqlx::query! (@)",
        "sqlx :: query!(@)",
        "sqlx :: query ! (@)",
        "sqlx::query!(\n    @)",
        "sqlx::query![@]",
        "sqlx::query!{@}",
        "r#sqlx::r#query!(@)",
    ];
    let literals = [
        format!("\"{PRUNE}\""),
        format!("r\"{PRUNE}\""),
        format!("r#\"{PRUNE}\"#"),
        format!("r##\"{PRUNE}\"##"),
        format!("r###\"{PRUNE}\"###"),
        format!("\"{}\"", PRUNE.replacen('D', "\\x44", 1)),
        format!("\"{}\"", PRUNE.replacen('D', "\\u{44}", 1)),
        format!("\"{}\"", PRUNE.replacen(" WHERE", " \\\n    WHERE", 1)),
    ];
    for path in paths {
        for literal in &literals {
            let call = path.replacen('@', &format!("{literal}, cutoff"), 1);
            spellings.push(format!("let done = {call};"));
        }
    }
    spellings
}

#[test]
fn every_spelling_of_the_call_rustc_reads_as_the_call_is_the_call() {
    let spellings = every_spelling_of_the_call();
    assert_eq!(spellings.len(), 130, "the population changed");
    for spelling in &spellings {
        assert_eq!(
            prune_pin_problems(spelling),
            Vec::<String>::new(),
            "a spelling rustc reads as the call was refused: {spelling:?}"
        );
    }
    eprintln!("examined {} spellings", spellings.len());
}

#[test]
fn a_prune_spelled_by_an_escape_a_continuation_or_tokens_beside_a_dead_copy_is_refused() {
    // Each prune runs a statement the index cannot seek, its keyword spelled so that no text scan
    // reads the word: an escape, a line continuation inside the word, or the words as tokens
    // through `stringify!`. Beside each stands a copy of the call that never runs.
    let moved = format!("DELETE FROM {TABLE} WHERE created_at + 0 < ?1");
    let lives = [
        format!("\"{}\"", moved.replacen('D', "\\x44", 1)),
        format!("\"{}\"", moved.replacen('D', "\\u{44}", 1)),
        format!("\"{}\"", moved.replacen("DELETE", "DE\\\nLETE", 1)),
        format!("r#\"{moved}\"#"),
        format!("stringify!({moved})"),
    ];
    let call = a_copy_of_the_call();
    let dead = [
        format!("fn _unused(cutoff: i64) {{ let _ = {call}; }}\n"),
        format!("#[cfg(any())]\nfn _dead(cutoff: i64) {{ let _ = {call}; }}\n"),
        format!("macro_rules! _never {{ ($c:expr) => {{ sqlx::query!(\"{PRUNE}\", $c) }}; }}\n"),
        format!("let _unused = {call};\n"),
    ];
    let mut members = 0;
    for live in &lives {
        for copy in &dead {
            let source = format!("{copy}let done = sqlx::query({live}).bind(cutoff);");
            assert_eq!(
                prune_pin_problems(&source),
                ["the source writes the word delete 2 times, not once"],
                "a prune spelled around the word count was not refused: {source}"
            );
            members += 1;
        }
    }
    assert_eq!(members, 20, "the population changed");
    eprintln!("examined {members} spelled prunes");
}

#[test]
fn a_source_that_does_not_lex_is_refused() {
    // An unclosed delimiter: the pin cannot read where the call ends, so it refuses the source.
    let source = format!("let done = sqlx::query!(\"{PRUNE}\", cutoff;");
    let problems = prune_pin_problems(&source);
    assert!(
        problems
            .iter()
            .any(|problem| problem.starts_with("the source does not lex as Rust")),
        "a source that does not lex was read: {source}: {problems:?}"
    );
}

#[test]
fn a_first_line_rustc_removes_as_a_shebang_is_refused() {
    // rustc removes a first line opening with `#!` unless `[` follows past whitespace and plain
    // comments, so a call written on it is no code: the pin refuses the source. `\u{a0}` and
    // `\u{3000}` are whitespace to `proc-macro2` and not to rustc, which removes those lines too.
    let call = a_good_prune_source();
    let mut members = 0;
    for bom in ["", "\u{feff}"] {
        for gap in [
            "",
            " ",
            "\t",
            "/usr/bin/env run-cargo-script ",
            " /* c */ ",
            " /** d */ ",
            "\u{a0}[allow(unused)] ",
            "\u{3000}[allow(unused)] ",
        ] {
            let source = format!("{bom}#!{gap}{call}\nfn prune_before() {{}}\n");
            let problems = prune_pin_problems(&source);
            assert!(
                problems.iter().any(|problem| problem.contains("shebang")),
                "a shebang line was read as code: {source:?}: {problems:?}"
            );
            members += 1;
        }
    }
    assert_eq!(members, 16, "the population changed");
    // An inner attribute is no shebang: the call under `#![allow(unused)]` is read.
    assert_eq!(
        prune_pin_problems(&format!("#![allow(unused)]\n{call}")),
        Vec::<String>::new()
    );
}

/// Macros that keep an item and discard the doc attribute before it, each matching a doc attribute
/// written with a call as its tokens: an expression, a token tree, an inner attribute, and a raw
/// `r#doc`, which rustc resolves as `doc` and both counts skip as one.
const DOC_DISCARDERS: [(&str, &str); 4] = [
    (
        "macro_rules! keep { (#[doc = $e:expr] $i:item) => { $i }; }",
        "#[doc = @]",
    ),
    (
        "macro_rules! keep { (#[doc($($t:tt)*)] $i:item) => { $i }; }",
        "#[doc(@)]",
    ),
    (
        "macro_rules! keep { (#![doc = $e:expr] $i:item) => { $i }; }",
        "#![doc = @]",
    ),
    (
        "macro_rules! keep { (# $attribute:tt $i:item) => { $i }; }",
        "#[r#doc = @]",
    ),
];

#[test]
fn a_query_written_inside_a_doc_attribute_is_neither_a_run_nor_a_word() {
    // A macro takes a doc attribute whose tokens are a copy of the call and discards it, keeping
    // the item: rustc runs only the prune in the item. Each member was labelled by rustc 1.97.0
    // through a stand-in `sqlx` whose `run` logs the statement: only the tested prune runs it.
    let copy = a_copy_of_the_call();
    let none_run = "the code hands the statement to sqlx::query! 0 times, not once";
    let lives = [
        (
            format!(
                "sqlx::query(\"DELETE FROM {TABLE} WHERE created_at + 0 < ?1\").bind(cutoff).run();"
            ),
            vec![none_run],
        ),
        (
            format!(
                "sqlx::query!(\"DELETE FROM main.{TABLE} WHERE created_at < ?1\", cutoff).run();"
            ),
            vec![none_run],
        ),
        (format!("{copy}.run();"), vec![]),
        (
            "let _ = cutoff;".to_owned(),
            vec![
                none_run,
                "the source writes the word delete 0 times, not once",
            ],
        ),
    ];
    let mut members = 0;
    for (discarder, attribute) in DOC_DISCARDERS {
        let attribute = attribute.replace('@', &copy);
        for (live, expected) in &lives {
            let prune = format!("pub fn prune_before(cutoff: i64) {{ {live} }}");
            for source in [
                format!("{discarder}\nkeep!({attribute} {prune});\n"),
                format!("{discarder}\nkeep!({attribute} struct Kept;);\n{prune}\n"),
            ] {
                assert_eq!(
                    prune_pin_problems(&source),
                    *expected,
                    "a call inside a doc attribute was read as code: {source}"
                );
                members += 1;
            }
        }
    }
    assert_eq!(members, 32, "the population changed");
    eprintln!("examined {members} doc-attribute copies");
}

#[test]
fn each_count_names_the_number_it_read() {
    // Two calls are two runs and two words; a text holding no statement is refused by the test
    // file's own count as well as by one statement too many.
    let twice = format!("{}\n{}", a_good_prune_source(), a_good_prune_source());
    assert_eq!(
        prune_pin_problems(&twice),
        [
            "the code hands the statement to sqlx::query! 2 times, not once",
            "the source writes the word delete 2 times, not once",
        ]
    );
    assert_eq!(
        own_statement_problems("fn plan() {}"),
        ["the test file writes the statement 0 times, not once"]
    );
}
