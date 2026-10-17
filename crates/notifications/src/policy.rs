//! The typed policy (SPEC-041 R2, R10, R14): `notifications-policy.json`, compiled into the binary
//! and read once at start, so the policy the router runs is the policy the box run judged.
//!
//! Every key of the file has a field here, and a written-back policy equals the file (A1). A
//! refusal names the key: a section, or `kinds.<kind>` for a kind, with serde's words for the field
//! inside it. Start is also refused when the holdout names an undeclared kind, when a kind names an
//! undeclared budget, and when the policy's withhold reasons lack one the router records. Only
//! [`Policy::kind`] makes a [`Kind`], so an occasion can never carry a kind the policy does not
//! declare.

use std::collections::BTreeMap;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::occasion::{COMEBACK_BUDGET, Class, DedupeScope, Kind, Surface, Tier};
use crate::quiet::ClockTime;
use crate::router::Reason;

/// The schema the policy file names.
pub const POLICY_SCHEMA: &str = "phx.notifications.policy.v1";

/// The policy file, as this binary was built with it.
const POLICY_FILE: &str = include_str!("../../../notifications-policy.json");

/// The budget every celebration kind spends: the ladder's weekly budgets (#120).
const CELEBRATION_BUDGET: &str = "celebration";

/// The file's top-level keys, each once.
const TOP_KEYS: [&str; 19] = [
    "schema",
    "surfaces",
    "router",
    "kinds",
    "ladder",
    "celebration_budgets",
    "streak_break",
    "near_miss",
    "quiet_hours",
    "digest",
    "deferral",
    "send_failure",
    "nudge_budgets",
    "lapse",
    "comeback",
    "holdout",
    "withhold",
    "deviations",
    "replies",
];

/// The notification policy.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Policy {
    schema: String,
    surfaces: Vec<Surface>,
    router: RouterSection,
    kinds: BTreeMap<String, KindSpec>,
    pub(crate) ladder: Ladder,
    pub(crate) celebration_budgets: CelebrationBudgets,
    pub(crate) streak_break: StreakBreak,
    pub(crate) near_miss: NearMiss,
    pub(crate) quiet_hours: QuietHours,
    digest: Digest,
    pub(crate) deferral: Deferral,
    pub(crate) send_failure: SendFailure,
    nudge_budgets: BTreeMap<String, NudgeBudget>,
    pub(crate) lapse: Lapse,
    pub(crate) comeback: Comeback,
    holdout: Holdout,
    withhold: Withhold,
    deviations: Vec<Deviation>,
    /// The duties of the command replies, which no kind describes (SPEC-323). Only the box run's
    /// `message-metadata` check reads them; nothing in production does.
    replies: Vec<String>,
}

/// The router the policy names: its module, its symbol and each surface's delivery calls.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RouterSection {
    module: String,
    symbol: String,
    transport: BTreeMap<Surface, Vec<String>>,
}

/// One kind as the file declares it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct KindSpec {
    class: Class,
    tiers: Vec<Tier>,
    budget: Option<String>,
    dedupe: DedupeScope,
    setting: Option<String>,
}

/// The celebration ladder's values, which the ladder's renders read (#120).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Ladder {
    tiers: BTreeMap<Tier, String>,
    pub(crate) rarity: BTreeMap<String, Tier>,
    pub(crate) rarity_floor: BTreeMap<String, Tier>,
    pub(crate) events: BTreeMap<String, Tier>,
    pub(crate) unknown_event: Tier,
    pub(crate) reaction_max_age_hours: u32,
    dedupe: DedupeScope,
}

/// The weekly budgets of the loud tiers, by intensity.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CelebrationBudgets {
    window: String,
    pub(crate) intensities: BTreeMap<String, BTreeMap<Tier, u32>>,
    pub(crate) default_intensity: String,
    pub(crate) downgrade: BTreeMap<Tier, Tier>,
    pub(crate) exempt_events: Vec<String>,
}

/// The streak-break day's cap. The file's key for its deferral says `fanfare`, a word the
/// lexicon keeps out of this context's identifiers, so the field is renamed (R14).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct StreakBreak {
    pub(crate) cap: Tier,
    #[serde(rename = "defer_fanfare")]
    defer_celebration: bool,
}

/// The near-miss copy's bounds.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct NearMiss {
    pub(crate) max_units: u32,
    pub(crate) max_fraction: f64,
}

/// The quiet window and what it does to each class.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct QuietHours {
    pub(crate) start: ClockTime,
    pub(crate) end: ClockTime,
    celebrations: String,
    nudges: String,
    pub(crate) exempt_classes: Vec<Class>,
}

/// When the digest fires, and the rollover it follows.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Digest {
    at: ClockTime,
    rollover: ClockTime,
}

/// The deferral's bounds.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Deferral {
    pub(crate) max_age_minutes: u32,
    pub(crate) flush_max: u32,
    pub(crate) queue_max: u32,
    overflow: String,
    recap_at_flush: bool,
    named_drops: bool,
}

/// The failed-send hold.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SendFailure {
    hold: bool,
    pub(crate) retry_max: u32,
    pub(crate) outage_cooldown_ms: u32,
    keep_first_deferred_at: bool,
}

/// One nudge budget.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct NudgeBudget {
    per_day: u32,
    exempt: Vec<String>,
    exempt_checked_first: bool,
}

/// What a lapse does.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Lapse {
    after_zero_days: u32,
    pub(crate) suppress_classes: Vec<Class>,
    escalation: String,
    habit_backoff: HabitBackoff,
}

/// The habit check-in's backoff in a long lapse.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct HabitBackoff {
    after_days: u32,
    interval_days: u32,
}

/// The comeback's cap.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Comeback {
    pub(crate) max_per_episode: u32,
    pub(crate) min_gap_days: u32,
    episode_key: String,
    after_cap: String,
    landmarks: Landmarks,
    pub(crate) disable_value: String,
}

/// The comeback's fresh-start landmarks.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Landmarks {
    monday_window_days: u32,
    month_start_window_days: u32,
}

/// The holdout, which the ablation delivery draws (#132).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Holdout {
    pct: u32,
    max_pct: u32,
    draw: String,
    seed: String,
    kinds: Vec<String>,
    record_held: bool,
    held_consumes_budget: bool,
    readout_min_n: u32,
}

/// The withhold ledger's rules.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Withhold {
    record: bool,
    kind_suffix: String,
    reasons: Vec<String>,
}

/// One recorded difference from the pack's baseline, with the ADR that decided it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Deviation {
    /// The key that differs, such as `kinds.reading_ready`.
    pub key: String,
    /// The ADR that decided it, as a path from the repository's root.
    pub adr: String,
}

/// Why the policy refuses start. Each names the key it refuses.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum PolicyError {
    /// The file is not a JSON object.
    #[error("the notification policy is not a JSON object: {reason}")]
    NotAnObject {
        /// What the JSON reader said.
        reason: String,
    },
    /// The file has a key the policy does not know.
    #[error("the notification policy has an unknown key {key}")]
    UnknownKey {
        /// The key.
        key: String,
    },
    /// The file lacks a section.
    #[error("the notification policy lacks the key {key}")]
    Missing {
        /// The key.
        key: &'static str,
    },
    /// A value is not what its key holds.
    #[error("the notification policy's {key} is malformed: {reason}")]
    Malformed {
        /// The key: a section, or `kinds.<kind>`.
        key: String,
        /// What is wrong with it.
        reason: String,
    },
    /// A key names a kind the policy does not declare.
    #[error("the notification policy's {key} names the undeclared kind {kind}")]
    UndeclaredKind {
        /// The key that names it.
        key: &'static str,
        /// The kind it names.
        kind: String,
    },
    /// A kind names a budget the policy does not declare.
    #[error("the notification policy's {key} names the undeclared budget {budget}")]
    UndeclaredBudget {
        /// The kind's budget key, `kinds.<kind>.budget`.
        key: String,
        /// The budget it names.
        budget: String,
    },
}

impl Policy {
    /// The policy this binary was built with, read at start.
    ///
    /// # Errors
    ///
    /// Every refusal of [`Policy::parse`].
    pub fn compiled() -> Result<Self, PolicyError> {
        Self::parse(POLICY_FILE)
    }

    /// The policy `text` holds.
    ///
    /// # Errors
    ///
    /// [`PolicyError`], naming the key it refuses.
    pub fn parse(text: &str) -> Result<Self, PolicyError> {
        let object = match serde_json::from_str::<Value>(text) {
            Ok(Value::Object(object)) => object,
            Ok(_) => {
                return Err(PolicyError::NotAnObject {
                    reason: "another JSON value".to_owned(),
                });
            }
            Err(error) => {
                return Err(PolicyError::NotAnObject {
                    reason: error.to_string(),
                });
            }
        };
        if let Some(key) = object.keys().find(|key| !TOP_KEYS.contains(&key.as_str())) {
            return Err(PolicyError::UnknownKey { key: key.clone() });
        }
        let policy = Self {
            schema: section(&object, "schema")?,
            surfaces: section(&object, "surfaces")?,
            router: section(&object, "router")?,
            kinds: kinds(&object)?,
            ladder: section(&object, "ladder")?,
            celebration_budgets: section(&object, "celebration_budgets")?,
            streak_break: section(&object, "streak_break")?,
            near_miss: section(&object, "near_miss")?,
            quiet_hours: section(&object, "quiet_hours")?,
            digest: section(&object, "digest")?,
            deferral: section(&object, "deferral")?,
            send_failure: section(&object, "send_failure")?,
            nudge_budgets: section(&object, "nudge_budgets")?,
            lapse: section(&object, "lapse")?,
            comeback: section(&object, "comeback")?,
            holdout: section(&object, "holdout")?,
            withhold: section(&object, "withhold")?,
            deviations: section(&object, "deviations")?,
            replies: section(&object, "replies")?,
        };
        policy.check()?;
        Ok(policy)
    }

    /// The declared kind `name`, or `None` when the policy declares no such kind.
    #[must_use]
    pub fn kind(&self, name: &str) -> Option<Kind> {
        self.kinds.get(name).map(|spec| {
            Kind::declared(
                name.to_owned(),
                spec.class,
                spec.tiers.clone(),
                spec.budget.clone(),
                spec.dedupe,
                spec.setting.clone(),
            )
        })
    }

    /// The kinds of the celebration class, whose deliveries the weekly budget counts (SPEC-084 R4).
    pub(crate) fn celebration_kinds(&self) -> impl Iterator<Item = &str> {
        self.kinds
            .iter()
            .filter(|(_, spec)| spec.class == Class::Celebration)
            .map(|(name, _)| name.as_str())
    }

    /// Every recorded deviation from the pack's baseline.
    #[must_use]
    pub fn deviations(&self) -> &[Deviation] {
        &self.deviations
    }

    /// The refusals no single section can see: the schema, the holdout's kinds, each kind's budget
    /// and the reasons the router records.
    fn check(&self) -> Result<(), PolicyError> {
        if self.schema != POLICY_SCHEMA {
            return Err(PolicyError::Malformed {
                key: "schema".to_owned(),
                reason: format!("it names {:?}, not {POLICY_SCHEMA}", self.schema),
            });
        }
        if let Some(kind) = self
            .holdout
            .kinds
            .iter()
            .find(|kind| !self.kinds.contains_key(kind.as_str()))
        {
            return Err(PolicyError::UndeclaredKind {
                key: "holdout.kinds",
                kind: kind.clone(),
            });
        }
        for (name, spec) in &self.kinds {
            if let Some(budget) = &spec.budget
                && budget != CELEBRATION_BUDGET
                && budget != COMEBACK_BUDGET
                && !self.nudge_budgets.contains_key(budget)
            {
                return Err(PolicyError::UndeclaredBudget {
                    key: format!("kinds.{name}.budget"),
                    budget: budget.clone(),
                });
            }
        }
        if let Some(reason) = Reason::ALL
            .into_iter()
            .find(|reason| !self.withhold.reasons.iter().any(|r| r == reason.as_str()))
        {
            return Err(PolicyError::Malformed {
                key: "withhold.reasons".to_owned(),
                reason: format!("it lacks {}, which the router records", reason.as_str()),
            });
        }
        if let Some(reply) = self
            .replies
            .iter()
            .find(|reply| self.kinds.contains_key(reply.as_str()))
        {
            return Err(PolicyError::Malformed {
                key: "replies".to_owned(),
                reason: format!(
                    "it names {reply}, a declared kind: a notification is judged, never skipped as a reply"
                ),
            });
        }
        Ok(())
    }
}

/// The section `key` of `object`, read as its type.
fn section<T: DeserializeOwned>(
    object: &Map<String, Value>,
    key: &'static str,
) -> Result<T, PolicyError> {
    let value = object.get(key).ok_or(PolicyError::Missing { key })?;
    serde_json::from_value(value.clone()).map_err(|error| PolicyError::Malformed {
        key: key.to_owned(),
        reason: error.to_string(),
    })
}

/// The `kinds` section, each kind read on its own so a refusal names it.
fn kinds(object: &Map<String, Value>) -> Result<BTreeMap<String, KindSpec>, PolicyError> {
    let declared: Map<String, Value> = section(object, "kinds")?;
    declared
        .into_iter()
        .map(|(name, value)| {
            let spec = serde_json::from_value(value).map_err(|error| PolicyError::Malformed {
                key: format!("kinds.{name}"),
                reason: error.to_string(),
            })?;
            Ok((name, spec))
        })
        .collect()
}
