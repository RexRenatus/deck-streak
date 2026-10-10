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
const OWED: [(&str, &str, &[&str]); 35] = [
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
    (
        "sync_login",
        "sends the engine's sync login, its endpoint set, on (1,3) through the dispatcher and answers the host key (SPEC-364 B6, ADR-375 D16)",
        &[
            "endpoint: Some(endpoint)",
            "dispatcher()?.run(service::SYNC, 3, &request.encode_to_vec())",
            ".map_err(sync_refusal)?",
            "Ok(auth.hkey)",
        ],
    ),
    (
        "sync_collection",
        "sends the engine's normal sync with no media and no timeout override on (1,5) through the dispatcher (SPEC-364 B6, ADR-375 D16)",
        &[
            "endpoint: Some(endpoint)",
            "io_timeout_secs: None",
            "sync_media: false",
            "dispatcher()?.run(service::SYNC, 5, &request.encode_to_vec())",
            ".map_err(sync_refusal)?",
        ],
    ),
    (
        "handshake",
        "hands the Worker's statement to the core's dispatcher and answers the core's own decision and sentence, keeping no rule of its own (SPEC-374 R23)",
        &[
            "dispatcher()?.handshake(statement.as_deref());",
            "let outcome = deck_streak_engine_core::handshake::decide(statement.as_deref());",
            "match deck_streak_engine_core::handshake::admits(outcome) {",
            "Ok(()) => Ok(None),",
            "let error: BackendError = decode(&refusal)?;",
            "Ok(Some(error.message))",
        ],
    ),
    (
        "sync_refusal",
        "keeps an engine refusal's bytes for the credential module's classifier and answers any other refusal as the boundary's (SPEC-364 B6, ADR-375 D16)",
        &[
            "Refusal::Engine { error } => Uint8Array::from(error.as_slice()).into()",
            "refuse(StudyError::CallRefused { service, method })",
        ],
    ),
];

/// What the boundary no longer holds (SPEC-365 A11; SPEC-364 B6, ADR-375 D11): `rate` never calls
/// `AnswerCard` through `call`, and no export answers the queue's head; no call runs Undo (3,8)
/// through `call`, with no argument or any other (SPEC-371 A17); neither sync export reaches
/// `admit` or `run_method`. Each is a text whose presence is refused, with
/// the function that must not hold it, or `None` for the whole source.
const RETIRED: [(Option<&str>, &str, &str); 7] = [
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
    (
        Some("sync_login"),
        "admit(",
        "reaches the engine through the dispatcher alone, never through the study allow-list",
    ),
    (
        Some("sync_login"),
        "run_method(",
        "reaches the engine through the dispatcher alone, never through `run_method`",
    ),
    (
        Some("sync_collection"),
        "admit(",
        "reaches the engine through the dispatcher alone, never through the study allow-list",
    ),
    (
        Some("sync_collection"),
        "run_method(",
        "reaches the engine through the dispatcher alone, never through `run_method`",
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

/// The control for the sync exports' rows of [`RETIRED`] (SPEC-364 B6, ADR-375 D11): `admit(` or
/// `run_method(` planted into either export's body is refused by that export's name, so the
/// census cannot pass over a sync that went through the study allow-list.
#[test]
fn a_sync_export_that_reaches_admit_or_run_method_is_refused_by_name() {
    let source = boundary();
    let exports = examined("sync export(s)", vec!["sync_login", "sync_collection"]);
    let mut planted_count = 0_usize;
    for name in exports {
        let held = body(&source, name).expect("each sync export has one body");
        for planted_call in [
            "admit(service, method)?;",
            "run_method(service, method, &[])?;",
        ] {
            let planted =
                source.replacen(held, &format!("{{\n    {planted_call}\n{}", &held[1..]), 1);
            let refused = retired(&planted);
            let text = &planted_call[..=planted_call.find('(').expect("a call has a paren")];
            assert!(
                refused
                    .iter()
                    .any(|line| line.starts_with(&format!("{name} "))
                        && line.ends_with(&format!("`{text}`"))),
                "`{text}` planted in {name} is not refused by name: {refused:?}"
            );
            planted_count += 1;
        }
    }
    assert_eq!(planted_count, 4, "examined {planted_count} of 4 plants");
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

// SPEC-377 R4, R5, A6 (ADR-388 D7 to D9): the full sync's choice in the web engine. The pool is
// installed as the core's `Files` port at start; the four choice exports reach the engine only
// through `one_way` and the dispatcher's unsynced read, and take no path, id set or snapshot answer
// of their own; the confirm hands the core the Worker's answer; the cancel drops the held stage;
// and every choice file is a new name minted after the pool is reserved for it.

/// The pool's port and its install, compared as [`OWED`]'s statements are (SPEC-377 R4).
const POOL_PORT: [(&str, &str, &[&str]); 3] = [
    (
        "create_backend",
        "installs the pool as the core's files port when it starts the dispatcher",
        &[
            "let port: Arc<dyn CoreFiles> = Arc::new(PoolPort);",
            "dispatcher.install_files(port);",
        ],
    ),
    (
        "holds",
        "answers that a path holds a file only when the pool lists its one name",
        &["files::holds(&pool.list(), &name)"],
    ),
    (
        "same",
        "names two paths one file only when they are one pool name",
        &["files::same(&open.to_string_lossy(), &path.to_string_lossy())"],
    ),
];

/// The confirm's snapshot check: the answer the core judges is the Worker's (SPEC-377 R5, R6).
const SNAPSHOT: [(&str, &str, &[&str]); 1] = [(
    "full_sync_confirm",
    "hands the core the snapshot answer the Worker read, and no answer of its own",
    &["backed_up.snapshot_found(&SnapshotAnswer { found })"],
)];

/// The cancel: the held stage is dropped, the model's `Cancel` (ADR-388 D9).
const CANCEL: [(&str, &str, &[&str]); 1] = [(
    "full_sync_cancel",
    "drops the stage held between the owner's taps",
    &["STAGE.with(|stage| {", "*stage.borrow_mut() = None;"],
)];

/// The reserve before each choice file (ADR-388 D8): the pool grows before the name is minted.
const RESERVE: [(&str, &str, &[&str]); 1] = [(
    "choice_file",
    "reserves the pool for a new file before it mints that file's name",
    &[
        "pool.reserve_minimum_capacity(pool.count() + 3).await.map_err(storage)?;",
        "files::choice_name(&pool.list(), COLLECTION_PATH, kind)",
    ],
)];

/// The four choice exports (SPEC-377 R5): each name, the parameters it takes and nothing more, and
/// the statements its body holds, each a call of `one_way` or of the dispatcher's unsynced read.
const CHOICE_EXPORTS: [(&str, &str, &[&str]); 4] = [
    (
        "full_sync_count",
        "key: String, endpoint: String, required: u32",
        &[
            "let copy = choice_file(Kind::Server).await?;",
            "one_way::count(&dispatcher()?, &answer, &auth, Path::new(&copy))",
            "STAGE.with(|stage| *stage.borrow_mut() = Some(Stage { counted, copy }));",
        ],
    ),
    (
        "full_sync_confirm",
        "direction: u32, key: String, endpoint: String, found: bool",
        &[
            "OwnerGesture::from_tap(ExemptWrite::OneWaySync, Target::Collection)",
            "let backup = choice_file(Kind::Backup).await?;",
            "one_way::back_up(&dispatcher, confirmed, Path::new(&backup), Path::new(&copy))",
            "let fresh = choice_file(Kind::Server).await?;",
            "one_way::recheck(&dispatcher, checked, &auth, Path::new(&fresh))",
            "one_way::write(&dispatcher, ready, gesture, &auth)",
        ],
    ),
    ("full_sync_cancel", "", &["*stage.borrow_mut() = None;"]),
    ("unsynced", "", &["dispatcher()?.unsynced()"]),
];

/// What no choice export may hold: an engine call other than `one_way`'s and the unsynced read,
/// an id set or a counted state of its own making, or a name minted without the reserve.
const CHOICE_REFUSED: [&str; 11] = [
    "call(",
    "query(",
    ".run(",
    "run_method(",
    "admit(",
    ".execute(",
    ".private(",
    ".run_one_way(",
    "IdSets",
    "Counted::show(",
    "choice_name(",
];

/// Each statement of `owed` that `source` does not hold, named with its function and why.
fn lacks(source: &str, owed: &[(&str, &str, &[&str])]) -> Vec<String> {
    let mut found = Vec::new();
    for (name, why, statements) in owed {
        match body(source, name) {
            Err(problem) => found.push(format!("{name}: {problem}")),
            Ok(text) => {
                let text = squeezed(text);
                for statement in *statements {
                    if !text.contains(&squeezed(statement)) {
                        found.push(format!("{name} {why}, and its body lacks `{statement}`"));
                    }
                }
            }
        }
    }
    found
}

/// The parameter list of `fn <name>(`, up to its closing parenthesis, with every blank removed.
fn parameters(source: &str, name: &str) -> Option<String> {
    let head = format!("fn {name}(");
    let start = source.find(&head)? + head.len();
    let end = source[start..].find(')')? + start;
    Some(squeezed(&source[start..end]))
}

/// What the choice exports owe and hold that they must not, each named with its export: an export
/// the module does not make, a parameter beyond its list, a statement it lacks, or a refused text.
fn choice_problems(source: &str) -> Vec<String> {
    let mut found = lacks(
        source,
        &CHOICE_EXPORTS.map(|(name, _, owed)| {
            (
                name,
                "reaches the engine through `one_way` and the dispatcher",
                owed,
            )
        }),
    );
    let whole = squeezed(source);
    for (name, wanted, _) in CHOICE_EXPORTS {
        let exported = ["pubasyncfn", "pubfn"]
            .iter()
            .any(|head| whole.contains(&format!("#[wasm_bindgen]{head}{name}(")));
        if !exported {
            found.push(format!("{name} is not an export of the module"));
        }
        let taken = parameters(source, name);
        if taken.as_deref() != Some(squeezed(wanted).as_str()) {
            found.push(format!("{name} takes `{taken:?}`, not `{wanted}` alone"));
        }
        if let Ok(text) = body(source, name) {
            let text = squeezed(text);
            for refused in CHOICE_REFUSED {
                if text.contains(&squeezed(refused)) {
                    found.push(format!(
                        "{name} reaches past `one_way`, and holds `{refused}`"
                    ));
                }
            }
        }
    }
    found
}

/// Where `choice_file` mints a name before it reserves the pool, named with the function.
fn reserve_problems(source: &str) -> Vec<String> {
    let mut found = lacks(source, &RESERVE);
    if let Ok(text) = body(source, "choice_file") {
        let text = squeezed(text);
        let [(_, _, [reserve, name])] = RESERVE else {
            return found;
        };
        let at = |statement: &str| text.find(&squeezed(statement));
        if let (Some(reserved), Some(named)) = (at(reserve), at(name))
            && named < reserved
        {
            found.push(format!("choice_file mints `{name}` before `{reserve}`"));
        }
    }
    found
}

/// `source` with `planted` written as the first statement of `name`'s body.
fn planted_in(source: &str, name: &str, planted: &str) -> String {
    let held = body(source, name).unwrap_or_else(|problem| panic!("{name}: {problem}"));
    source.replacen(held, &format!("{{\n    {planted}\n{}", &held[1..]), 1)
}

/// Whether `refused` names `name` and the text `text` as its cause.
fn refused_by_name(refused: &[String], name: &str, text: &str) -> bool {
    refused
        .iter()
        .any(|line| line.starts_with(&format!("{name} ")) && line.contains(text))
}

#[test]
fn the_web_engine_installs_the_pool_as_the_cores_files_port() {
    let source = boundary();
    assert_eq!(lacks(&source, &POOL_PORT), Vec::<String>::new());

    // the install planted out, and each port answer planted as a constant, are refused by name
    let install = "dispatcher.install_files(port);";
    let uninstalled = source.replacen(install, "let _ = &port;", 1);
    assert_ne!(uninstalled, source, "the install was not planted out");
    let refused = lacks(&uninstalled, &POOL_PORT);
    assert!(
        refused_by_name(&refused, "create_backend", install),
        "an uninstalled port is not refused by name: {refused:?}"
    );
    for name in ["holds", "same"] {
        let held = body(&source, name).expect("each port answer has one body");
        let constant = source.replacen(held, "{\n    true\n}", 1);
        let refused = lacks(&constant, &POOL_PORT);
        assert!(
            refused_by_name(&refused, name, "lacks"),
            "a constant {name} is not refused by name: {refused:?}"
        );
    }
    examined("pool port function(s) of src/wasm.rs", POOL_PORT.to_vec());
}

#[test]
fn the_snapshot_answer_reaches_the_core_as_the_worker_read_it() {
    let source = boundary();
    assert_eq!(lacks(&source, &SNAPSHOT), Vec::<String>::new());
    assert_eq!(
        parameters(&source, "full_sync_confirm").filter(|taken| taken.ends_with("found:bool")),
        Some(squeezed(CHOICE_EXPORTS[1].1)),
        "the confirm takes the Worker's answer as its last parameter"
    );

    // an answer of the export's own making is refused by name
    let forged = source.replacen(
        "SnapshotAnswer { found }",
        "SnapshotAnswer { found: true }",
        1,
    );
    assert_ne!(forged, source, "the answer was not planted");
    let refused = lacks(&forged, &SNAPSHOT);
    assert!(
        refused_by_name(&refused, "full_sync_confirm", "SnapshotAnswer"),
        "a forged snapshot answer is not refused by name: {refused:?}"
    );
    examined("snapshot check(s) of src/wasm.rs", SNAPSHOT.to_vec());
}

#[test]
fn a_cancel_drops_the_held_stage() {
    let source = boundary();
    assert_eq!(lacks(&source, &CANCEL), Vec::<String>::new());

    // a cancel that keeps the stage is refused by name
    let kept = source.replacen("*stage.borrow_mut() = None;", "let _ = &stage;", 1);
    assert_ne!(kept, source, "the clearing was not planted out");
    let held = body(&kept, "full_sync_cancel").expect("the cancel has one body");
    let refused = lacks(&kept, &CANCEL);
    assert!(
        squeezed(held).contains("let_=&stage;")
            && refused_by_name(&refused, "full_sync_cancel", "= None;"),
        "a cancel that keeps the stage is not refused by name: {refused:?}"
    );
    examined("cancel(s) of src/wasm.rs", CANCEL.to_vec());
}

#[test]
fn each_choice_file_is_reserved_before_it_is_named() {
    let source = boundary();
    assert_eq!(reserve_problems(&source), Vec::<String>::new());
    let minting: Vec<&str> = CHOICE_EXPORTS
        .iter()
        .filter(|(name, _, _)| {
            body(&source, name).is_ok_and(|text| squeezed(text).contains("choice_file(Kind::"))
        })
        .map(|(name, _, _)| *name)
        .collect();
    assert_eq!(minting, ["full_sync_count", "full_sync_confirm"]);

    // a name minted unreserved, or minted before the reserve, is refused by name
    let [(_, _, [reserve, name])] = RESERVE else {
        panic!("the reserve census names one reserve and one name");
    };
    let unreserved = source.replacen(reserve, "let _ = &pool;", 1);
    assert_ne!(unreserved, source, "the reserve was not planted out");
    let refused = reserve_problems(&unreserved);
    assert!(
        refused_by_name(&refused, "choice_file", "reserve_minimum_capacity"),
        "an unreserved name is not refused by name: {refused:?}"
    );
    let early = planted_in(&source, "choice_file", &format!("let _ = {name};"));
    let refused = reserve_problems(&early);
    assert!(
        refused_by_name(&refused, "choice_file", "before"),
        "a name minted before the reserve is not refused by name: {refused:?}"
    );
    let direct = planted_in(
        &source,
        "full_sync_confirm",
        "let _ = files::choice_name(&[], COLLECTION_PATH, Kind::Backup);",
    );
    let refused = choice_problems(&direct);
    assert!(
        refused_by_name(&refused, "full_sync_confirm", "choice_name("),
        "a name minted past the reserve is not refused by name: {refused:?}"
    );
    examined("choice file minter(s) of src/wasm.rs", RESERVE.to_vec());
}

#[test]
fn the_choice_exports_reach_the_engine_only_through_one_way_and_the_dispatcher() {
    let source = boundary();
    assert_eq!(choice_problems(&source), Vec::<String>::new());

    // an engine call past `one_way`, a parameter beyond the list, and an export unmade are each
    // refused by the export's name
    for (name, _, _) in CHOICE_EXPORTS {
        let past = planted_in(&source, name, "call(service::SYNC, 5, &[])?;");
        let refused = choice_problems(&past);
        assert!(
            refused_by_name(&refused, name, "`call(`"),
            "an engine call planted in {name} is not refused by name: {refused:?}"
        );
        let head = format!("fn {name}(");
        let widened = source.replacen(&head, &format!("{head}copy: String, "), 1);
        let refused = choice_problems(&widened);
        assert!(
            refused_by_name(&refused, name, "takes"),
            "a path parameter planted in {name} is not refused by name: {refused:?}"
        );
    }
    let unexported = source.replacen("#[wasm_bindgen]\npub fn unsynced(", "pub fn unsynced(", 1);
    assert_ne!(
        unexported, source,
        "the unsynced export was not planted out"
    );
    let refused = choice_problems(&unexported);
    assert!(
        refused_by_name(&refused, "unsynced", "not an export"),
        "an unexported read is not refused by name: {refused:?}"
    );
    let exports = examined("choice export(s) of src/wasm.rs", CHOICE_EXPORTS.to_vec());
    let refused_texts = examined("refused text(s)", CHOICE_REFUSED.to_vec());
    println!(
        "examined {} x {} refusal(s)",
        exports.len(),
        refused_texts.len()
    );
}

/// The backups' three exports (SPEC-377 R15 to R17; ADR-388 D14, D15, D17, D18), each with the
/// parameters it takes and the statements it owes: every pool name comes from `files.rs`, and every
/// removal from the core's retention rule. The export answers bytes, which the Worker transfers.
const BACKUP_EXPORTS: [(&str, &str, &[&str]); 3] = [
    (
        "backups",
        "",
        &[
            "let listed = recorded(&pool).await?;",
            "files::backups(&listed)",
        ],
    ),
    (
        "export_backup",
        "id: String",
        &[
            "files::exported(&pool.list(), COLLECTION_PATH, &id)",
            "pool.export_db(&name)",
        ],
    ),
    (
        "retain",
        "",
        &[
            "STAGE.with(|stage| stage.borrow().is_some())",
            "let listed = recorded(&pool).await?;",
            "let held = files::backups(&listed);",
            "retention::removals(&kept)",
        ],
    ),
];

/// What no backup export may hold: an engine call, a path it was handed, or a name it made itself.
const BACKUP_REFUSED: [&str; 9] = [
    "call(",
    "query(",
    ".run(",
    "run_method(",
    "dispatcher(",
    "Path::new(",
    "choice_name(",
    "choice_file(",
    "format!(",
];

/// What the backup exports owe and hold that they must not, each named with its export: an export
/// the module does not make, a parameter beyond its list, a statement it lacks, a refused text, or
/// an export that answers its bytes as anything but bytes.
fn backup_problems(source: &str) -> Vec<String> {
    let mut found = lacks(
        source,
        &BACKUP_EXPORTS.map(|(name, _, owed)| {
            (
                name,
                "reaches the pool through `files.rs` and the core's retention",
                owed,
            )
        }),
    );
    let whole = squeezed(source);
    for (name, wanted, _) in BACKUP_EXPORTS {
        let exported = ["pubasyncfn", "pubfn"]
            .iter()
            .any(|head| whole.contains(&format!("#[wasm_bindgen]{head}{name}(")));
        if !exported {
            found.push(format!("{name} is not an export of the module"));
        }
        let taken = parameters(source, name);
        if taken.as_deref() != Some(squeezed(wanted).as_str()) {
            found.push(format!("{name} takes `{taken:?}`, not `{wanted}` alone"));
        }
        if let Ok(text) = body(source, name) {
            let text = squeezed(text);
            for refused in BACKUP_REFUSED {
                if text.contains(&squeezed(refused)) {
                    found.push(format!(
                        "{name} reaches past `files.rs`, and holds `{refused}`"
                    ));
                }
            }
        }
    }
    if !whole.contains("pubasyncfnexport_backup(id:String)->Result<Vec<u8>,JsValue>{") {
        found.push("export_backup answers its bytes as something other than bytes".to_owned());
    }
    found
}

#[test]
fn the_backup_exports_reach_the_pool_only_through_files_and_the_core() {
    let source = boundary();
    assert_eq!(backup_problems(&source), Vec::<String>::new());

    // an engine call, a path parameter and a name made in the export are each refused by the
    // export's name, and so is an export that answers its bytes as text
    for (name, _, _) in BACKUP_EXPORTS {
        let past = planted_in(&source, name, "call(service::SYNC, 5, &[])?;");
        let refused = backup_problems(&past);
        assert!(
            refused_by_name(&refused, name, "`call(`"),
            "an engine call planted in {name} is not refused by name: {refused:?}"
        );
        let head = format!("fn {name}(");
        let widened = source.replacen(&head, &format!("{head}path: String, "), 1);
        let refused = backup_problems(&widened);
        assert!(
            refused_by_name(&refused, name, "takes"),
            "a path parameter planted in {name} is not refused by name: {refused:?}"
        );
        let minted = planted_in(&source, name, "let name = format!(\"/deck-streak/{id}\");");
        let refused = backup_problems(&minted);
        assert!(
            refused_by_name(&refused, name, "`format!(`"),
            "a name made in {name} is not refused by name: {refused:?}"
        );
    }
    let text = source.replacen(
        "pub async fn export_backup(id: String) -> Result<Vec<u8>, JsValue> {",
        "pub async fn export_backup(id: String) -> Result<String, JsValue> {",
        1,
    );
    assert_ne!(text, source, "the export's answer was not planted as text");
    let refused = backup_problems(&text);
    assert!(
        refused_by_name(&refused, "export_backup", "other than bytes"),
        "an export that answers text is not refused by name: {refused:?}"
    );
    let exports = examined("backup export(s) of src/wasm.rs", BACKUP_EXPORTS.to_vec());
    let refused_texts = examined("refused text(s)", BACKUP_REFUSED.to_vec());
    println!(
        "examined {} x {} refusal(s)",
        exports.len(),
        refused_texts.len()
    );
}

/// The helpers the backup exports call, each with what its body owes: the installed pool or a
/// refusal, a record of each newly listed file before the list is answered, the age in whole
/// seconds and never negative, and the export's refusal naming the id. Like the exports they are
/// compiled for `wasm32` alone, so this census is their one native reader (SPEC-377 section 16).
const BACKUP_HELPERS: [(&str, &[&str]); 4] = [
    (
        "installed",
        &[
            "POOL.with(|p| p.borrow().clone())",
            ".ok_or_else(|| refuse(\"storage-refused: the pool is not installed\"))",
        ],
    ),
    (
        "recorded",
        &[
            "let unrecorded = files::unrecorded(&pool.list());",
            "if !unrecorded.is_empty() {",
            "pool.import_db_unchecked(&files::record(name, made), &[])",
            "Ok(pool.list())",
        ],
    ),
    ("age", &["u64::try_from((now - made) / 1000).unwrap_or(0)"]),
    (
        "unlisted",
        &["refuse(format!(\"no listed backup is named {id}\"))"],
    ),
];

/// Each statement of [`BACKUP_HELPERS`] that `source` does not hold, named with its helper.
fn helper_problems(source: &str) -> Vec<String> {
    lacks(
        source,
        &BACKUP_HELPERS.map(|(name, owed)| (name, "serves the backup exports", owed)),
    )
}

#[test]
fn the_backup_helpers_keep_the_pool_the_record_the_age_and_the_refusal() {
    let source = boundary();
    assert_eq!(helper_problems(&source), Vec::<String>::new());

    // each helper answering a constant, its body replaced whole, is refused by its name
    for (name, _) in BACKUP_HELPERS {
        let held = body(&source, name).unwrap_or_else(|problem| panic!("{name}: {problem}"));
        let constant = source.replacen(held, "{\n    Default::default()\n}", 1);
        assert_ne!(constant, source, "{name}'s body was not replaced");
        let refused = helper_problems(&constant);
        assert!(
            refused_by_name(&refused, name, "lacks"),
            "{name} answering a constant is not refused by name: {refused:?}"
        );
    }
    // the record's guard turned round, and the age's division turned to a remainder, are refused
    for (name, from, to) in [
        (
            "recorded",
            "if !unrecorded.is_empty() {",
            "if unrecorded.is_empty() {",
        ),
        ("age", "(now - made) / 1000", "(now - made) % 1000"),
    ] {
        let planted = source.replacen(from, to, 1);
        assert_ne!(planted, source, "{name}: `{from}` was not planted");
        let refused = helper_problems(&planted);
        assert!(
            refused_by_name(&refused, name, "lacks"),
            "`{to}` planted in {name} is not refused by name: {refused:?}"
        );
    }
    let helpers = examined("backup helper(s) of src/wasm.rs", BACKUP_HELPERS.to_vec());
    let owed: usize = helpers.iter().map(|(_, owed)| owed.len()).sum();
    println!("examined {owed} owed statement(s)");
}
