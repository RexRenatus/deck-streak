//! The Kani harnesses as they were proved (SPEC-509 R9; ADR-527 D7). `src/kani_proofs.rs`
//! compiles only under the Kani verifier (`#[cfg(kani)]`), so no native build holds a harness and
//! no native test can call one. This test reads the file as text instead and holds each harness,
//! and the symbolic key `key_order` draws from, to the text it was proved as: a harness changed in
//! any way fails here until this copy changes with it, and the change is proved again under Kani.

// An integration test is test code: it prints the examined count on purpose.
#![allow(clippy::print_stdout)]

/// The harness file, as the Kani verifier reads it.
const HARNESSES: &str = include_str!("../src/kani_proofs.rs");

/// Each function of the harness file, from its `fn` line to its closing brace, as proved.
const PROVED: [(&str, &str); 5] = [
    (
        "any_key",
        r"fn any_key() -> Key {
    Key {
        t: kani::any(),
        rank: kani::any(),
        origin: kani::any(),
        seq: kani::any(),
    }
}",
    ),
    (
        "sync_codec",
        r#"fn sync_codec() {
    let bytes: [u8; 48] = kani::any();
    let len: usize = kani::any();
    kani::assume(len <= bytes.len());
    let _ = decode(&bytes[..len]);

    let first: [u8; 4] = kani::any();
    let second: [u8; 4] = kani::any();
    kani::assume(first.iter().chain(second.iter()).all(u8::is_ascii));
    let (Ok(first), Ok(second)) = (
        String::from_utf8(first.to_vec()),
        String::from_utf8(second.to_vec()),
    ) else {
        panic!("ASCII bytes are UTF-8");
    };
    let head = OpHead {
        origin: kani::any(),
        seq: kani::any(),
        hlc: kani::any(),
    };
    let op = Op::NoteCreated {
        note: kani::any(),
        notetype: kani::any(),
        deck: kani::any(),
        guid: kani::any(),
        fields: vec![first, second],
    };
    let frame = encode(&head, &op);
    assert!(frame.is_ok());
    if let Ok(frame) = frame {
        assert!(decode(&frame) == Ok((head, op)));
    }
}"#,
    ),
    (
        "key_order",
        r"fn key_order() {
    let (a, b, c) = (any_key(), any_key(), any_key());
    assert!((compare(&a, &b) == Ordering::Equal) == (a == b));
    assert!(compare(&a, &b) == compare(&b, &a).reverse());
    if compare(&a, &b) == Ordering::Less && compare(&b, &c) == Ordering::Less {
        assert!(compare(&a, &c) == Ordering::Less);
    }
}",
    ),
    (
        "hlc_tick_no_overflow",
        r"fn hlc_tick_no_overflow() {
    let (last, seen, pt): (i64, i64, i64) = (kani::any(), kani::any(), kani::any());
    match tick(last, seen, pt) {
        Ok(next) => assert!(next > last && next > seen && next >= pt),
        Err(ClockError::Exhausted) => assert!(last.max(seen) == i64::MAX),
    }
}",
    ),
    (
        "next_id_holds_its_residue_and_bounds",
        r"fn next_id_holds_its_residue_and_bounds() {
    let (floor, now, number): (i64, i64, u8) = (kani::any(), kani::any(), kani::any());
    let Some(slot) = Slot::new(number) else {
        return;
    };
    if let Ok(id) = next_id(floor, now, slot) {
        assert!(id.rem_euclid(SLOTS) == i64::from(number));
        assert!(id > floor);
        assert!(id >= now);
    }
}",
    ),
];

/// The function `name` of `text`: from its `fn` line to the first line after it that is `}`.
fn function<'t>(text: &'t str, name: &str) -> Option<&'t str> {
    let start = text.find(&format!("\nfn {name}("))? + 1;
    let length = text[start..].find("\n}\n")? + 2;
    Some(&text[start..start + length])
}

/// `proved` with its body replaced by a constant, the way a mutant that drops a body writes it: a
/// function with a return type answers `Default::default()`, one without answers `()`.
fn constant_body(proved: &str) -> String {
    let signature = proved.lines().next().unwrap_or_default();
    let value = if signature.contains(" -> ") {
        "Default::default()"
    } else {
        "()"
    };
    format!("{signature}\n    {value}\n}}")
}

#[test]
fn each_sync_kani_harness_reads_as_it_was_proved() {
    assert_eq!(
        HARNESSES.matches("\nfn ").count(),
        PROVED.len(),
        "every function of the harness file is held here"
    );
    let mut examined = 0;
    for (name, proved) in PROVED {
        assert_eq!(function(HARNESSES, name), Some(proved), "{name}");
        let constant = constant_body(proved);
        let planted = HARNESSES.replacen(proved, &constant, 1);
        assert_eq!(
            function(&planted, name),
            Some(constant.as_str()),
            "{name}: the planted constant body is the function read"
        );
        assert_ne!(
            function(&planted, name),
            Some(proved),
            "{name}: a planted constant body reads as a change"
        );
        examined += 1;
    }
    assert_eq!(
        examined,
        PROVED.len(),
        "every harness function reached its assertions"
    );
    println!(
        "examined {examined} of {} harness function(s), each with a planted constant body",
        PROVED.len()
    );
}
