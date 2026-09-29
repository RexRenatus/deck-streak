//! The redactor's registry counts what it holds, shows none of it, escapes every spelling a log
//! line can carry, and its log writer passes a flush on to the writer beneath it (SPEC-020 R12,
//! R13; SPEC-057 A18, the kernel's mutants).

use std::io::{self, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use deck_streak_kernel::Redactor;
use deck_streak_kernel::redact::{REDACTED, RedactingMakeWriter};
use tracing_subscriber::fmt::MakeWriter;

/// A writer that counts the flushes it is given and keeps what it is written.
struct Counting {
    flushes: Arc<AtomicUsize>,
    written: Arc<std::sync::Mutex<Vec<u8>>>,
}

impl Write for Counting {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.written
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.flushes.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

#[test]
fn the_registry_counts_the_secrets_it_holds_and_debug_shows_only_that_count() {
    let redactor = Redactor::new();
    assert_eq!(redactor.registered(), 0);
    assert_eq!(format!("{redactor:?}"), "Redactor { registered: 0 }");
    assert!(redactor.register("lantern-quiet-river"));
    assert!(redactor.register("copper-sparrow-drift"));
    // A value already held, and a value too short to register, are not counted.
    assert!(!redactor.register("lantern-quiet-river"));
    assert!(!redactor.register("abc"));
    assert_eq!(redactor.registered(), 2);
    let shown = format!("{redactor:?}");
    assert_eq!(shown, "Redactor { registered: 2 }");
    assert!(
        !shown.contains("lantern"),
        "a secret reached Debug: {shown}"
    );
}

#[test]
fn a_secret_holding_a_quote_leaves_a_json_line_in_its_debug_escaped_spelling() {
    // A value with a quote and a backslash: a tracing field that logs it with `?` writes the Rust
    // debug spelling, and the JSON line then escapes that again.
    let secret = "a\"b\\c-drift";
    let redactor = Redactor::new();
    assert!(redactor.register(secret));
    let debug = format!("{secret:?}");
    let debug = &debug[1..debug.len() - 1];
    let json_of_debug = serde_json::to_string(debug).expect("a string encodes");
    let json_of_debug = &json_of_debug[1..json_of_debug.len() - 1];
    assert_ne!(
        json_of_debug, secret,
        "the spelling under test is a distinct one"
    );
    let line = format!("{{\"value\":\"{json_of_debug}\"}}");
    assert_eq!(
        redactor.redact_line(&line),
        format!("{{\"value\":\"{REDACTED}\"}}")
    );
}

#[test]
fn a_flush_of_the_log_writer_reaches_the_writer_beneath_it() {
    let flushes = Arc::new(AtomicUsize::new(0));
    let written = Arc::new(std::sync::Mutex::new(Vec::new()));
    let make = {
        let (flushes, written) = (Arc::clone(&flushes), Arc::clone(&written));
        move || Counting {
            flushes: Arc::clone(&flushes),
            written: Arc::clone(&written),
        }
    };
    let makes = RedactingMakeWriter::new(Redactor::new(), make);
    let mut writer = makes.make_writer();
    writer.write_all(b"one line\n").expect("written");
    writer.flush().expect("flushed");
    assert_eq!(flushes.load(Ordering::SeqCst), 1);
    assert_eq!(
        written
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .as_slice(),
        b"one line\n"
    );
}
