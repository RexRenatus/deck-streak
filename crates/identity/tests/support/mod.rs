//! Test support for identity's linking and passkey tests (SPEC-359): a `tracing` capture that keeps
//! every event and span it is given, field by field, so a test can read both what was logged and
//! that nothing secret was.

// Each test binary uses a part of this module, and an integration test's helpers panic on a failed
// fixture.
#![allow(dead_code, clippy::expect_used)]

use std::fmt::{self, Write as _};
use std::sync::{Arc, Mutex, PoisonError};

use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Id, Record};
use tracing::{Event, Metadata, Subscriber};

/// One captured event or span: where it came from, and each field as it was recorded.
#[derive(Clone, Debug)]
pub struct Line {
    /// The event's level, or `span` for a span's fields.
    pub level: String,
    /// The module path the event or span was emitted from.
    pub target: String,
    /// Each field's name and its recorded text, in the order they were recorded.
    pub fields: Vec<(String, String)>,
}

impl Line {
    /// The recorded text of the field `name`, if the line carries it.
    #[must_use]
    pub fn field(&self, name: &str) -> Option<&str> {
        self.fields
            .iter()
            .find(|(field, _)| field == name)
            .map(|(_, value)| value.as_str())
    }

    /// The whole line as one text: its level, its target and every field, for a search that must
    /// not miss a value wherever it was recorded.
    #[must_use]
    pub fn rendered(&self) -> String {
        let mut text = format!("{} {}", self.level, self.target);
        for (name, value) in &self.fields {
            let _ = write!(text, " {name}={value}");
        }
        text
    }
}

/// A subscriber that keeps every event and span it is given.
#[derive(Clone, Default)]
pub struct Captured(Arc<Mutex<Vec<Line>>>);

impl Captured {
    /// Every line captured so far.
    #[must_use]
    pub fn lines(&self) -> Vec<Line> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    fn push(&self, line: Line) {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(line);
    }
}

/// The fields of one event or span, as they are recorded.
#[derive(Default)]
struct Fields(Vec<(String, String)>);

impl Visit for Fields {
    fn record_str(&mut self, field: &Field, value: &str) {
        self.0.push((field.name().to_owned(), value.to_owned()));
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.0.push((field.name().to_owned(), format!("{value:?}")));
    }
}

impl Subscriber for Captured {
    fn enabled(&self, _: &Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, span: &Attributes<'_>) -> Id {
        let mut fields = Fields::default();
        span.record(&mut fields);
        self.push(Line {
            level: "span".to_owned(),
            target: span.metadata().target().to_owned(),
            fields: fields.0,
        });
        Id::from_u64(1)
    }

    fn record(&self, _: &Id, values: &Record<'_>) {
        let mut fields = Fields::default();
        values.record(&mut fields);
        self.push(Line {
            level: "span".to_owned(),
            target: String::new(),
            fields: fields.0,
        });
    }

    fn record_follows_from(&self, _: &Id, _: &Id) {}

    fn event(&self, event: &Event<'_>) {
        let metadata = event.metadata();
        let mut fields = Fields::default();
        event.record(&mut fields);
        self.push(Line {
            level: metadata.level().to_string(),
            target: metadata.target().to_owned(),
            fields: fields.0,
        });
    }

    fn enter(&self, _: &Id) {}

    fn exit(&self, _: &Id) {}
}
