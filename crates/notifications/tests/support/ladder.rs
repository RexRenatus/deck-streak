//! What the ladder's tests share (SPEC-084): a transport that implements every ladder call and
//! records each one, answering from a script; the golden calls read as the port's calls; and
//! seeders of the owner's latest message, the week's celebrations and the held queue. Every value
//! is synthetic.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use deck_streak_kernel::{StudyDayRule, UtcMillis};
use deck_streak_notifications::owner_message;
use deck_streak_notifications::{BotTransport, Pass, PushFuture, Pushed, Router, Tier};
use serde_json::Value;
use sqlx::Row;

use super::Harness;

/// One call the router made of the transport.
#[derive(Clone, Debug, PartialEq)]
pub struct Call {
    /// The port's call, by its name.
    pub name: &'static str,
    /// The emoji it carried: a dice's or a reaction's.
    pub emoji: Option<String>,
    /// The reveal's pause.
    pub pause: Option<Duration>,
    /// The message it carried, if any.
    pub text: Option<String>,
}

/// A bot transport that implements every ladder call, records each, and answers each from its
/// script, delivering when the script is spent.
#[derive(Debug, Default)]
pub struct Scripted {
    calls: Mutex<Vec<Call>>,
    script: Mutex<HashMap<&'static str, VecDeque<Pushed>>>,
}

impl Scripted {
    /// Scripts the next answers of `name`, in order.
    pub fn script(&self, name: &'static str, answers: impl IntoIterator<Item = Pushed>) {
        self.script
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .entry(name)
            .or_default()
            .extend(answers);
    }

    /// Every call so far, in order.
    pub fn calls(&self) -> Vec<Call> {
        self.calls
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// Forgets the calls so far and the script.
    pub fn clear(&self) {
        self.calls
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clear();
        self.script
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clear();
    }

    fn answer(&self, call: Call) -> PushFuture<'_> {
        let pushed = self
            .script
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get_mut(call.name)
            .and_then(VecDeque::pop_front)
            .unwrap_or(Pushed::Delivered);
        self.calls
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(call);
        Box::pin(std::future::ready(pushed))
    }
}

impl BotTransport for Scripted {
    fn push_message<'a>(&'a self, _pass: &'a Pass, text: &'a str) -> PushFuture<'a> {
        self.answer(Call {
            name: "push_message",
            emoji: None,
            pause: None,
            text: Some(text.to_owned()),
        })
    }

    fn push_reveal<'a>(
        &'a self,
        _pass: &'a Pass,
        _placeholder: &'a str,
        text: &'a str,
        pause: Duration,
    ) -> PushFuture<'a> {
        self.answer(Call {
            name: "push_reveal",
            emoji: None,
            pause: Some(pause),
            text: Some(text.to_owned()),
        })
    }

    fn push_dice<'a>(&'a self, _pass: &'a Pass, emoji: &'a str) -> PushFuture<'a> {
        self.answer(Call {
            name: "push_dice",
            emoji: Some(emoji.to_owned()),
            pause: None,
            text: None,
        })
    }

    fn push_reaction<'a>(
        &'a self,
        _pass: &'a Pass,
        _message_id: i64,
        emoji: &'a str,
    ) -> PushFuture<'a> {
        self.answer(Call {
            name: "push_reaction",
            emoji: Some(emoji.to_owned()),
            pause: None,
            text: None,
        })
    }

    fn push_pin<'a>(&'a self, _pass: &'a Pass, text: &'a str) -> PushFuture<'a> {
        self.answer(Call {
            name: "push_pin",
            emoji: None,
            pause: None,
            text: Some(text.to_owned()),
        })
    }
}

/// A harness whose router delivers through a [`Scripted`] transport.
pub async fn scripted(start: UtcMillis) -> (Harness, Arc<Scripted>) {
    let mut harness = Harness::without_bot(start).await;
    let bot = Arc::new(Scripted::default());
    harness.router = Router::new(
        Arc::clone(&harness.policy),
        harness.db.clone(),
        harness.clock.clone(),
        StudyDayRule::default(),
    )
    .with_bot(bot.clone());
    (harness, bot)
}

/// The tier numbered `number`, as a golden writes it.
pub fn tier(number: u64) -> Tier {
    Tier::parse(&format!("T{number}")).expect("a tier from T0 to T5")
}

/// One port call a golden's calls come to, with what the stand-in answers it.
#[derive(Clone, Debug, PartialEq)]
pub struct Expected {
    /// The port's call.
    pub name: &'static str,
    /// Its emoji, unless the golden's case named a dice of its own, which the router does not take.
    pub emoji: Option<String>,
    /// The reveal's pause, when the golden recorded one.
    pub pause: Option<Duration>,
    /// What the stand-in answers.
    pub answer: Pushed,
}

/// The golden `calls` as the port's calls. The predecessor's `send_html_id` and `send_html` are one
/// Bot API method, and its reveal, dice message and pinned message are each one call of the port,
/// whose composition the bot's transport owns (A14): so a run of calls of one key from a
/// `send_html_id` to the next dice, reaction or message is one call, named by the `tier` it renders
/// (`tier_of` reads the tier of the call's key, if the golden keys its calls), and it delivers
/// unless its last call is a failed one other than the pin. A call of another key, as the rollup's
/// line is, ends the run. A call's answer is a failure when `fail` names its method.
pub fn port_calls(
    calls: &[Value],
    fail: &[String],
    mut tier_of: impl FnMut(&[Value]) -> u64,
) -> Vec<Expected> {
    let fails = |method: &str| fail.iter().any(|failing| failing == method);
    let answer = |failed: bool| {
        if failed {
            Pushed::Failed
        } else {
            Pushed::Delivered
        }
    };
    let method = |call: &Value| {
        let array = call.as_array().expect("a call is an array");
        let at = array.len().saturating_sub(2);
        array[at]
            .as_str()
            .expect("a call names its method")
            .to_owned()
    };
    let argument = |call: &Value| call.as_array().and_then(|array| array.last()).cloned();
    let key = |call: &Value| {
        call.as_array()
            .filter(|array| array.len() > 2)
            .and_then(|array| array[0].as_str().map(str::to_owned))
    };
    let mut expected = Vec::new();
    let mut at = 0;
    while at < calls.len() {
        let name = method(&calls[at]);
        match name.as_str() {
            "send_dice" | "set_reaction" => {
                let emoji = argument(&calls[at])
                    .and_then(|emoji| emoji.as_str().map(str::to_owned))
                    .filter(|emoji| !emoji.starts_with("qaa-"));
                expected.push(Expected {
                    name: if name == "send_dice" {
                        "push_dice"
                    } else {
                        "push_reaction"
                    },
                    emoji,
                    pause: None,
                    answer: answer(fails(&name)),
                });
                at += 1;
            }
            "send_html" => {
                expected.push(Expected {
                    name: "push_message",
                    emoji: None,
                    pause: None,
                    answer: answer(fails("send_html")),
                });
                at += 1;
            }
            "send_html_id" => {
                let start = at;
                let mut pause = None;
                at += 1;
                while at < calls.len()
                    && key(&calls[at]) == key(&calls[start])
                    && matches!(
                        method(&calls[at]).as_str(),
                        "sleep" | "edit_html" | "pin_message" | "send_html"
                    )
                {
                    if method(&calls[at]) == "sleep" {
                        let seconds = argument(&calls[at])
                            .and_then(|seconds| seconds.as_f64())
                            .expect("a pause in seconds");
                        pause = Some(Duration::from_secs_f64(seconds));
                    }
                    let last = method(&calls[at]);
                    at += 1;
                    if last == "send_html" || last == "pin_message" {
                        break;
                    }
                }
                let last = method(&calls[at - 1]);
                let delivered = last == "pin_message" || !fails(&last);
                let name = match tier_of(&calls[start..at]) {
                    3 => "push_reveal",
                    5 => "push_pin",
                    _ => "push_message",
                };
                expected.push(Expected {
                    name,
                    emoji: None,
                    pause,
                    answer: answer(!delivered),
                });
            }
            other => panic!("a golden call this reading does not know: {other}"),
        }
    }
    expected
}

/// Scripts `bot` to answer each expected call as the golden's stand-in did.
pub fn script(bot: &Scripted, expected: &[Expected]) {
    for call in expected {
        bot.script(call.name, [call.answer]);
    }
}

/// Holds the calls `bot` made to what the golden's calls come to.
pub fn assert_calls(bot: &Scripted, expected: &[Expected], context: &str) {
    let made: Vec<(&str, Option<String>, Option<Duration>)> = bot
        .calls()
        .into_iter()
        .zip(expected.iter().map(Some).chain(std::iter::repeat(None)))
        .map(|(call, expected)| {
            let emoji = expected.and_then(|expected| expected.emoji.as_ref().map(|_| ()));
            let pause = expected.and_then(|expected| expected.pause.map(|_| ()));
            (call.name, emoji.and(call.emoji), pause.and(call.pause))
        })
        .collect();
    let wanted: Vec<(&str, Option<String>, Option<Duration>)> = expected
        .iter()
        .map(|call| (call.name, call.emoji.clone(), call.pause))
        .collect();
    assert_eq!(made, wanted, "{context}: the port's calls, in order");
}

/// Records `message_id`, arrived at `arrived_at`, as the owner's latest message, through the
/// writer the bot records it with.
pub async fn seed_owner_message(harness: &Harness, message_id: i64, arrived_at: i64) {
    owner_message::record(
        &harness.db,
        message_id,
        UtcMillis::from_epoch_millis(arrived_at),
    )
    .await
    .expect("the owner's latest message is seeded");
}

/// Clears the owner's latest message.
pub async fn clear_owner_message(harness: &Harness) {
    sqlx::query("UPDATE owner_last_message SET message_id = NULL, arrived_at = NULL WHERE id = 1")
        .execute(harness.db.reader())
        .await
        .expect("the owner's latest message is cleared");
}

/// Records a celebration delivered at `tier` on `study_day`, as the router records one.
pub async fn seed_sent(harness: &Harness, key: &str, tier: Tier, study_day: i64) {
    sqlx::query(
        "INSERT INTO notification_decisions (dedupe_key, kind, surface, arm, reason, \
         tier_requested, tier_rendered, study_day, created_at) \
         VALUES (?, 'celebration', 'bot', 'send', NULL, ?, ?, ?, 0)",
    )
    .bind(key)
    .bind(tier.as_str())
    .bind(tier.as_str())
    .bind(study_day)
    .execute(harness.db.reader())
    .await
    .expect("a delivered celebration is seeded");
}

/// One queue row to seed.
#[derive(Clone, Debug)]
pub struct HeldSeed<'a> {
    pub key: &'a str,
    pub tier: Tier,
    pub hold: &'a str,
    pub tries: i64,
    pub state: &'a str,
    pub deferred_at: i64,
    pub study_day: i64,
}

/// Holds a celebration on the queue, as the router holds one, its text its key.
pub async fn seed_held(harness: &Harness, row: &HeldSeed<'_>) {
    sqlx::query(
        "INSERT INTO notification_queue (kind, dedupe_key, surface, tier_requested, tier_pending, \
         text, hold, tries, state, deferred_at, study_day, created_at) \
         VALUES ('celebration', ?, 'bot', ?, ?, ?, ?, ?, ?, ?, ?, 0)",
    )
    .bind(row.key)
    .bind(row.tier.as_str())
    .bind(row.tier.as_str())
    .bind(row.key)
    .bind(row.hold)
    .bind(row.tries)
    .bind(row.state)
    .bind(row.deferred_at)
    .bind(row.study_day)
    .execute(harness.db.reader())
    .await
    .expect("a held celebration is seeded");
}

/// One queue row as the router left it: its key, state, pending tier, hold and failed sends.
pub async fn queue_rows(harness: &Harness) -> Vec<(String, String, String, String, i64)> {
    sqlx::query(
        "SELECT dedupe_key, state, tier_pending, hold, tries FROM notification_queue ORDER BY id",
    )
    .fetch_all(harness.db.reader())
    .await
    .expect("the queue is read")
    .into_iter()
    .map(|row| (row.get(0), row.get(1), row.get(2), row.get(3), row.get(4)))
    .collect()
}
