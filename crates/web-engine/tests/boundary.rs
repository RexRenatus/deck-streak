//! SPEC-345 R5 (ADR-356 D1): the web engine's boundary, `src/wasm.rs`, reaches the engine only
//! through the core's dispatcher, which it starts on the web transport. That module compiles only
//! for `wasm32`, so no native test can run it: this census reads its source instead, one boundary
//! function at a time, and names each statement a function owes and does not hold.

// The examined helper prints its count on purpose; clippy.toml's in-test allowances cover only
// `#[test]` bodies.
#![allow(clippy::print_stdout)]

use std::fs;
use std::path::Path;

fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

/// Each boundary function the census reads: its name, why it owes what it owes, and the
/// statements its body holds for it. A statement is compared with every blank removed, so a
/// reflow by rustfmt changes nothing.
const OWED: [(&str, &str, &[&str]); 31] = [
    (
        "create_backend",
        "starts the core's dispatcher on the web transport and keeps it",
        &[
            "Dispatcher::start(Transport::Web, &msg.encode_to_vec())",
            "DISPATCHER.with(|d| *d.borrow_mut() = Some(dispatcher))",
        ],
    ),
    (
        "dispatcher",
        "answers the dispatcher it keeps, and refuses before one is started",
        &[
            "DISPATCHER.with(|d| d.borrow().clone())",
            ".ok_or_else(|| refuse(\"the engine is not initialised\"))",
        ],
    ),
    (
        "call",
        "runs each service and method through the dispatcher",
        &["dispatcher()?.run(service, method, input)"],
    ),
    (
        "query",
        "reads each fixed read through the dispatcher and answers its JSON",
        &["dispatcher()?.read(read)", "serde_json::from_slice(&reply)"],
    ),
    (
        "open",
        "opens the collection through the dispatcher and says whether it existed and its notes",
        &[
            "file == COLLECTION_PATH",
            "call(service::COLLECTION, 0, &request.encode_to_vec())?",
            "query(Read::NoteCount)?",
            "serde_json::json!({ \"existed\": existed, \"notes\": notes })",
            "LAST_ANSWER.with(|kept| *kept.borrow_mut() = None)",
        ],
    ),
    (
        "seed",
        "refuses a collection that holds notes, then adds Basic notes through the dispatcher",
        &[
            "query(Read::NoteCount)?",
            ".and_then(serde_json::Value::as_i64) != Some(0)",
            "return Err(refuse(\"seed refuses a collection that holds notes\"));",
            "entry.name == \"Basic\"",
            "call(service::NOTES, 2,",
            "u32::try_from(reply.nids.len())",
        ],
    ),
    (
        "snapshot",
        "answers one card's row from the core's fixed read",
        &["query(Read::CardSnapshot(card_id))?", ".pointer(\"/0\")"],
    ),
    // The owner's exempt tap (SPEC-345 R9): each tap names one write and a target of its kind,
    // and the write reaches the engine only as the gesture the tap builds.
    (
        "run_exempt",
        "maps each tap to its write and a target of the write's kind, and runs the owner's gesture through the dispatcher",
        &[
            "0 => (ExemptWrite::Forget, Target::Card(target)),",
            "1 => (ExemptWrite::SetDueDate, Target::Card(target)),",
            "2 => (ExemptWrite::DeletePreset, Target::Preset(target)),",
            "3 => (ExemptWrite::ChangeNoteType, Target::Note(target)),",
            "4 => (ExemptWrite::DeleteCard, Target::Card(target)),",
            "5 => (ExemptWrite::DeleteNote, Target::Note(target)),",
            "let gesture = OwnerGesture::from_tap(write, target).map_err(refuse)?;",
            "dispatcher()?.run_exempt(gesture, input).map_err(refuse)",
        ],
    ),
    // The review's exports (SPEC-350 R1 to R3): each reaches the engine through the dispatcher,
    // and a rating, bury or flag reaches only the card the review showed.
    (
        "deck_tree",
        "reads the deck tree with today's counts through the dispatcher and answers its decks",
        &[
            "now: now_millis() / 1000",
            "call(service::DECKS, 4, &request.encode_to_vec())?",
            "root.children.iter().map(deck_json)",
        ],
    ),
    (
        "deck_json",
        "answers a deck's id, name, counts and children as the deck list reads them",
        &[
            "\"id\": node.deck_id.to_string()",
            "\"review\": node.review_count",
            "\"children\": node.children.iter().map(deck_json).collect::<Vec<_>>()",
        ],
    ),
    (
        "joined",
        "joins a rendered side's text nodes and each replacement's current text",
        &[
            "Some(Value::Text(text)) => Some(text.as_str())",
            "Some(Value::Replacement(replacement)) => Some(replacement.current_text.as_str())",
            ".collect()",
        ],
    ),
    // The undo of the review's own last answer (SPEC-371 R6, R7): the offer reads and judges and
    // writes nothing, and the undo runs only for the record the offer named, through the gesture.
    (
        "undo_offer",
        "offers only the review's own last answer, judged against the engine now, with the card as one line of text",
        &[
            "LAST_ANSWER.with(|kept| kept.borrow().clone())",
            "call(service::COLLECTION, 7, &[])?",
            "undo_answer::judge(&recorded, &now, review_of(recorded.review)?, last.card)",
            "call(service::CARD_RENDERING, 14,",
            "preserve_media_filenames: true",
        ],
    ),
    (
        "undo",
        "undoes only the offered answer, judged again, through the owner's gesture on its card, then forgets the record and the kept card",
        &[
            "last_answer_for(kept.borrow().as_ref(), card, step)",
            "call(service::COLLECTION, 7, &[])?",
            "undo_answer::judge(&recorded, &now, review_of(recorded.review)?, card)",
            "OwnerGesture::from_tap(ExemptWrite::Undo, Target::Card(card))",
            ".run_exempt(gesture, &recorded.encode_to_vec())",
            "LAST_ANSWER.with(|kept| *kept.borrow_mut() = None)",
            "SHOWN.with(|kept| *kept.borrow_mut() = None)",
        ],
    ),
    (
        "set_current_deck",
        "makes the chosen deck current through the dispatcher",
        &["call(service::DECKS, 22, &DeckId { did: deck }.encode_to_vec())"],
    ),
    (
        "current_card",
        "reads the queue's head, renders it, strips it, labels it and keeps it",
        &[
            "call(service::SCHEDULER, 3, &request.encode_to_vec())?",
            "call(service::CARD_RENDERING, 6,",
            "partial_render: false",
            "call(service::CARD_RENDERING, 9,",
            "call(service::SCHEDULER, 24, &states.encode_to_vec())?",
            "call(service::COLLECTION, 7, &[])?",
            "*kept.borrow_mut() = Some(Shown {",
        ],
    ),
    (
        "rate",
        "records only the kept card's press, as the owner's answer, with the state its grade picks",
        &[
            "grade(rating)",
            "shown_for(kept.borrow().as_ref(), card)",
            "grade.pick(states.again, states.good)",
            "OwnerAnswer::from_press(shown.card, pressed(grade))",
            ".run_answer(answer, &request.encode_to_vec())",
            "query(Read::NewestReview)?",
            "newest.card == shown.card",
            "LAST_ANSWER.with(|kept| *kept.borrow_mut() = recorded)",
        ],
    ),
    (
        "pressed",
        "gives the core the grade the wire named, one for one (SPEC-365 R7)",
        &[
            "Grade::Again => answer::Grade::Again,",
            "Grade::Good => answer::Grade::Good,",
        ],
    ),
    (
        "bury",
        "buries only the kept card, the user's bury of that card alone",
        &[
            "shown_for(kept.borrow().as_ref(), card)",
            "bury_of(card)",
            "call(service::SCHEDULER, 14, &request.encode_to_vec())?",
        ],
    ),
    (
        "flag",
        "toggles red on only the kept card",
        &[
            "shown_for(kept.borrow().as_ref(), card)",
            "toggled_red(kept.flag)",
            "call(service::CARDS, 4,",
        ],
    ),
    (
        "faces",
        "completes both faces of only the kept card through the core, reading media it was given",
        &[
            "shown_for(kept.borrow().as_ref(), card)",
            "Files::new(names.into_iter().zip(contents.into_iter().map(|bytes| bytes.to_vec()))",
            "let wanted = Wanted::new(&files);",
            ".face(card, Side::Question, true, &wanted)",
            ".face(card, Side::Answer, true, &wanted)",
            "set(&reply, \"question\", &face_value(question)?)?;",
            "set(&reply, \"answer\", &face_value(answer)?)?;",
            "set(&reply, \"wanted\", &wanted_value(wanted.into_names())?)?;",
        ],
    ),
    (
        "read",
        "answers the core's read from the files given, recording what they lack",
        &["self.ask(name, limit)"],
    ),
    (
        "set",
        "writes one field of a reply",
        &["Reflect::set(target, &JsValue::from_str(key), value).map(|_| ())"],
    ),
    (
        "face_value",
        "carries a face's text, CSS, clips and omitted names as the core completed them",
        &[
            "set(&value, \"text\", &face.text.into())?;",
            "set(&value, \"css\", &face.css.into())?;",
            "set(&value, \"autoplay\", &clips_value(face.autoplay)?)?;",
            "set(&value, \"replay\", &clips_value(face.replay)?)?;",
            "face.omitted.into_iter().map(JsValue::from).collect()",
            "set(&value, \"omitted\", &omitted)?;",
        ],
    ),
    (
        "clips_value",
        "carries every clip, in the core's order",
        &["for clip in clips {", "array.push(&clip_value(clip)?);"],
    ),
    (
        "clip_value",
        "carries a sound's bytes with the type the core's table gives its name, and speech as text",
        &[
            "set(&value, \"kind\", &\"sound\".into())?;",
            "media_type(&name, &TYPES).map_or(JsValue::NULL, JsValue::from_str)",
            "set(&value, \"name\", &name.into())?;",
            "set(&value, \"bytes\", &Uint8Array::from(bytes.as_slice()).into())?;",
            "set(&value, \"kind\", &\"speech\".into())?;",
            "set(&value, \"text\", &text.into())?;",
            "set(&value, \"language\", &language.into())?;",
            "set(&value, \"rate\", &rate.into())?;",
        ],
    ),
    (
        "wanted_value",
        "names each file the core asked for and lacked, with its limit",
        &[
            "for (name, limit) in names {",
            "set(&ask, \"name\", &name.into())?;",
            "set(&ask, \"limit\", &limit.into())?;",
            "array.push(&ask);",
        ],
    ),
    (
        "credential_on_obtained",
        "lets the core's sync key rule decide whether a login is kept, and at which generation",
        &["credential::on_obtained(Generation::from(started), Generation::from(current))"],
    ),
    (
        "credential_may_send",
        "lets the core's sync key rule decide whether the held key may be sent",
        &["credential::may_send(Generation::from(held), Generation::from(current), sealed)"],
    ),
    (
        "credential_classify",
        "lets the core's sync key rule read a sync's answer as accepted, refused or failed",
        &["credential::classify(error.as_deref())"],
    ),
    (
        "credential_on_outcome",
        "lets the core's sync key rule decide whether an answer drops the key",
        &["credential::on_outcome(Generation::from(sent), Generation::from(current), outcome)"],
    ),
    (
        "credential_on_removed",
        "lets the core's sync key rule name the generation a removal moves the store to",
        &["credential::on_removed(Generation::from(current))"],
    ),
];

/// What the boundary no longer holds (SPEC-365 A11): `rate` never calls `AnswerCard` through
/// `call`, and no export answers the queue's head; and no call runs Undo (3,8) through `call`,
/// with no argument or any other (SPEC-371 A17). Each is a text whose presence is refused, with
/// the function that must not hold it, or `None` for the whole source.
const RETIRED: [(Option<&str>, &str, &str); 3] = [
    (
        Some("rate"),
        "call(service::SCHEDULER, 4,",
        "records a grade only through the owner's answer, never through `call`",
    ),
    (
        None,
        "pub fn answer(",
        "answers no card but the kept one: the queue-head export is gone",
    ),
    (
        None,
        "call(service::COLLECTION, 8,",
        "undoes only through the owner's gesture, never through `call`",
    ),
];

/// `text` with every blank removed.
fn squeezed(text: &str) -> String {
    text.chars().filter(|c| !c.is_whitespace()).collect()
}

/// The body of `fn <name>(` in `source`, from the first brace after the name to the brace that
/// closes it. A string literal is skipped whole, so a brace inside one counts for nothing. A name
/// defined other than once, or a body that never closes, is refused.
fn body<'a>(source: &'a str, name: &str) -> Result<&'a str, String> {
    let head = format!("fn {name}(");
    let starts: Vec<usize> = source.match_indices(&head).map(|(at, _)| at).collect();
    let [start] = starts.as_slice() else {
        return Err(format!("`{head}` occurs {} times, not once", starts.len()));
    };
    let Some(open) = source[*start..].find('{').map(|at| start + at) else {
        return Err(format!("`{head}` has no body"));
    };
    let mut depth = 0_usize;
    let mut chars = source[open..].char_indices();
    while let Some((at, c)) = chars.next() {
        match c {
            '"' => {
                while let Some((_, c)) = chars.next() {
                    match c {
                        '\\' => {
                            chars.next();
                        }
                        '"' => break,
                        _ => {}
                    }
                }
            }
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Ok(&source[open..=open + at]);
                }
            }
            _ => {}
        }
    }
    Err(format!("`{head}`'s body never closes"))
}

/// Each statement of [`OWED`] that `source` does not hold, named with its function and why the
/// function owes it.
fn problems(source: &str) -> Vec<String> {
    let mut found = Vec::new();
    for (name, why, statements) in OWED {
        match body(source, name) {
            Err(problem) => found.push(format!("{name}: {problem}")),
            Ok(text) => {
                let text = squeezed(text);
                for statement in statements {
                    if !text.contains(&squeezed(statement)) {
                        found.push(format!("{name} {why}, and its body lacks `{statement}`"));
                    }
                }
            }
        }
    }
    found
}

/// Each text of [`RETIRED`] that `source` still holds, named with where and why it is refused.
fn retired(source: &str) -> Vec<String> {
    let mut found = Vec::new();
    for (name, text, why) in RETIRED {
        let held = match name {
            Some(name) => {
                body(source, name).is_ok_and(|body| squeezed(body).contains(&squeezed(text)))
            }
            None => squeezed(source).contains(&squeezed(text)),
        };
        if held {
            found.push(format!(
                "{} {why}, and holds `{text}`",
                name.unwrap_or("src/wasm.rs")
            ));
        }
    }
    found
}

fn boundary() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/wasm.rs");
    fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

#[test]
fn each_boundary_function_reaches_the_engine_through_the_dispatcher() {
    let source = boundary();
    let functions = examined("boundary function(s) of src/wasm.rs", OWED.to_vec());
    let statements: usize = functions.iter().map(|(_, _, owed)| owed.len()).sum();
    println!("examined {statements} owed statement(s)");
    let retired_texts = examined("retired text(s)", RETIRED.to_vec());
    assert_eq!(
        (problems(&source), retired(&source)),
        (Vec::<String>::new(), Vec::<String>::new()),
        "{} retired text(s) judged",
        retired_texts.len()
    );
}

/// The control for [`RETIRED`] (SPEC-365 A11): each text planted back is refused by name, so a
/// census gone blind fails here rather than passing over a source it no longer reads.
#[test]
fn a_retired_text_planted_back_is_refused_by_name() {
    let source = boundary();
    let rate = body(&source, "rate").expect("rate has one body");
    let through_call = source.replacen(
        rate,
        "{\n    call(service::SCHEDULER, 4, &[])?;\n    Ok(())\n}",
        1,
    );
    let queue_head = format!("{source}\npub fn answer(rating: u32) -> u32 {{\n    rating\n}}\n");
    let undo_by_call = format!(
        "{source}\npub fn redo() -> u32 {{\n    call(service::COLLECTION, 8, &[]);\n    0\n}}\n"
    );
    let refused = (
        retired(&through_call),
        retired(&queue_head),
        retired(&undo_by_call),
    );
    assert!(
        refused.0.iter().any(|line| line.starts_with("rate "))
            && refused
                .1
                .iter()
                .any(|line| line.starts_with("src/wasm.rs ") && line.contains("pub fn answer("))
            && refused.2.iter().any(|line| {
                line.starts_with("src/wasm.rs ") && line.contains("call(service::COLLECTION, 8,")
            }),
        "a retired text planted back is not refused by name: {refused:?}"
    );
}

#[test]
fn a_boundary_function_that_answers_a_constant_is_refused_by_name() {
    let source = boundary();
    for (name, _, _) in examined("planted constant bodies", OWED.to_vec()) {
        let held = body(&source, name).expect("each boundary function has one body");
        let planted = source.replacen(held, "{\n    Ok(Default::default())\n}", 1);
        let refused = problems(&planted);
        assert!(
            refused
                .iter()
                .any(|line| line.starts_with(&format!("{name} "))),
            "a constant body planted in {name} is not refused by name: {refused:?}"
        );
    }
}

#[test]
fn the_body_reader_skips_a_brace_inside_a_string_and_refuses_a_name_defined_twice() {
    let source = "fn f() -> String { let s = \"}{\"; format!(\"{s}\\\"}\") }\nfn g() {}\n";
    assert_eq!(
        body(source, "f"),
        Ok("{ let s = \"}{\"; format!(\"{s}\\\"}\") }")
    );
    assert_eq!(body(source, "g"), Ok("{}"));
    assert_eq!(
        body("fn f() {}\nfn f() {}\n", "f"),
        Err("`fn f(` occurs 2 times, not once".to_owned())
    );
    assert_eq!(
        body("fn h() {", "h"),
        Err("`fn h(`'s body never closes".to_owned())
    );
}
