//! A recording log subscriber for SPEC-023's tests: every event a test's code logs on this thread,
//! with its level and its fields, in order. A WARN the SPEC promises (R3's uncovered law root, R6's
//! self-check) is read here as a positive artifact: the event, its level and the fields it names.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::{Arc, Mutex, PoisonError};

use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Id, Record};
use tracing::{Event, Level, Metadata, Subscriber};

/// One logged event: its level and every field, each written as text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Logged {
    /// The event's level.
    pub level: Level,
    /// Every field by name, the message included as `message`.
    pub fields: BTreeMap<String, String>,
}

impl Logged {
    /// The field `name`, when the event carries it.
    #[must_use]
    pub fn field(&self, name: &str) -> Option<&str> {
        self.fields.get(name).map(String::as_str)
    }
}

/// What a recorder has logged so far.
#[derive(Clone, Default)]
pub struct Logs(Arc<Mutex<Vec<Logged>>>);

impl Logs {
    /// A subscriber that records into these logs; set it as a test's default.
    #[must_use]
    pub fn recorder(&self) -> Recorder {
        Recorder(self.clone())
    }

    /// Every event recorded, in order.
    #[must_use]
    pub fn events(&self) -> Vec<Logged> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// Every WARN event recorded, in order.
    #[must_use]
    pub fn warnings(&self) -> Vec<Logged> {
        self.events()
            .into_iter()
            .filter(|event| event.level == Level::WARN)
            .collect()
    }
}

/// The subscriber: every event enabled and recorded; spans are accepted and never recorded.
pub struct Recorder(Logs);

impl Subscriber for Recorder {
    fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, _span: &Attributes<'_>) -> Id {
        Id::from_u64(1)
    }

    fn record(&self, _span: &Id, _values: &Record<'_>) {}

    fn record_follows_from(&self, _span: &Id, _follows: &Id) {}

    fn event(&self, event: &Event<'_>) {
        let mut fields = Fields::default();
        event.record(&mut fields);
        (self.0)
            .0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(Logged {
                level: *event.metadata().level(),
                fields: fields.0,
            });
    }

    fn enter(&self, _span: &Id) {}

    fn exit(&self, _span: &Id) {}
}

/// An event's fields, each written as text.
#[derive(Default)]
struct Fields(BTreeMap<String, String>);

impl Visit for Fields {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.0.insert(field.name().to_owned(), format!("{value:?}"));
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.0.insert(field.name().to_owned(), value.to_owned());
    }
}
