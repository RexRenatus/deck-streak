//! The closed states a topic ends a study day in, and a run's outcome (SPEC-045 R5, R6, R12;
//! ADR-045, ADR-054).
//!
//! One enum holds a topic's state, and each variant carries its reason, so a topic cannot hold two
//! states and an unknown reason cannot be written. A could-not-tell reason's class is a function of
//! the reason, R6's closed map, so a class and a reason can never disagree: nothing, the stored
//! row included, holds a class the reason does not give. `ready`, `failed` and `ai_route_absent` are
//! set by the generation (SPEC-046); `failed` carries the closed reasons SPEC-046 names, with the
//! agent's closed causes (SPEC-043), so the store's checks hold every state from its first
//! migration. `ai_route_absent` is a setting, not a fault (ADR-054): it carries no class and no
//! reason, and it never counts as a failure or a refusal.

use std::fmt;

/// The class of a could-not-tell reason: whether the rail broke, or the configuration is at fault.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Class {
    /// The sync, the collection or the resolution itself failed: the owner's setup is sound.
    RailBroken,
    /// The owner's configuration keeps the readings from telling: a setting or a deck to change.
    ConfigFault,
}

impl Class {
    /// Every class.
    pub const ALL: [Self; 2] = [Self::RailBroken, Self::ConfigFault];

    /// The class as the readings' tables store it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RailBroken => "rail_broken",
            Self::ConfigFault => "config_fault",
        }
    }
}

/// Why a topic could not tell whether it has new cards today (R6): six reasons, each of one class.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CouldNotTell {
    /// The last sync did not succeed, so the copy may not hold the owner's latest study.
    SyncFailed,
    /// The collection was held by another process, or the engine failed reading it.
    CollectionLocked,
    /// The collection, or the throwaway copy the queue reads, could not be made or opened.
    CollectionOpenFailed,
    /// The resolution ran past its 30-second budget (R7).
    DaySetResolveTimeout,
    /// No taxonomy is configured, or its file could not be read as one.
    TaxonomyMissing,
    /// A root's answer held fewer new cards than the scheduler's own count (R3).
    DaySetFetchSaturated,
}

impl CouldNotTell {
    /// Every reason, in R6's order.
    pub const ALL: [Self; 6] = [
        Self::SyncFailed,
        Self::CollectionLocked,
        Self::CollectionOpenFailed,
        Self::DaySetResolveTimeout,
        Self::TaxonomyMissing,
        Self::DaySetFetchSaturated,
    ];

    /// The reason's class: R6's closed map, one arm per reason.
    #[must_use]
    #[allow(
        clippy::match_same_arms,
        reason = "the closed map names each reason's class on its own line"
    )]
    pub const fn class(self) -> Class {
        match self {
            Self::SyncFailed => Class::RailBroken,
            Self::CollectionLocked => Class::RailBroken,
            Self::CollectionOpenFailed => Class::RailBroken,
            Self::DaySetResolveTimeout => Class::RailBroken,
            Self::TaxonomyMissing => Class::ConfigFault,
            Self::DaySetFetchSaturated => Class::ConfigFault,
        }
    }

    /// The reason as the readings' tables store it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SyncFailed => "sync_failed",
            Self::CollectionLocked => "collection_locked",
            Self::CollectionOpenFailed => "collection_open_failed",
            Self::DaySetResolveTimeout => "day_set_resolve_timeout",
            Self::TaxonomyMissing => "taxonomy_missing",
            Self::DaySetFetchSaturated => "day_set_fetch_saturated",
        }
    }

    /// The reason stored as `text`.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|reason| reason.as_str() == text)
    }
}

/// A gate the generation runs on a reading (SPEC-046 R6), as a `gate_failed` reason names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReadingGate {
    /// The run succeeded with a non-empty result.
    Complete,
    /// Every seed note is cited, and every citation is a seed note.
    Roster,
    /// Every usable anchor appears in the prose (law).
    Anchors,
    /// The prose is inside the owner's word band (law).
    Band,
    /// No prose line begins a bullet or a numbered item.
    NoListMarkers,
    /// The persona's output contract.
    Contract,
}

impl ReadingGate {
    /// Every gate, in SPEC-046 R6's order.
    pub const ALL: [Self; 6] = [
        Self::Complete,
        Self::Roster,
        Self::Anchors,
        Self::Band,
        Self::NoListMarkers,
        Self::Contract,
    ];

    /// The gate's name in a `gate_failed` reason.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::Roster => "roster",
            Self::Anchors => "anchors",
            Self::Band => "band",
            Self::NoListMarkers => "no_list_markers",
            Self::Contract => "contract",
        }
    }
}

/// Why a configured AI route's agent was unavailable (SPEC-043 R12), as an `agent_unavailable`
/// reason names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AgentCause {
    /// The route's key is missing.
    KeyMissing,
    /// The launch was refused for its shape.
    RefusedShape,
    /// The route refused the key.
    KeyRejected,
    /// The route's capacity is exhausted.
    CapacityExhausted,
    /// The proxy could not be reached.
    ProxyUnreachable,
    /// The run failed.
    RunFailed,
    /// The run reached its turn cap.
    TurnCap,
    /// The run reached its time cap.
    TimeCap,
    /// The run reached its budget cap.
    BudgetCap,
}

impl AgentCause {
    /// Every cause, in SPEC-043 R12's order.
    pub const ALL: [Self; 9] = [
        Self::KeyMissing,
        Self::RefusedShape,
        Self::KeyRejected,
        Self::CapacityExhausted,
        Self::ProxyUnreachable,
        Self::RunFailed,
        Self::TurnCap,
        Self::TimeCap,
        Self::BudgetCap,
    ];

    /// The cause's name in an `agent_unavailable` reason.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::KeyMissing => "key_missing",
            Self::RefusedShape => "refused_shape",
            Self::KeyRejected => "key_rejected",
            Self::CapacityExhausted => "capacity_exhausted",
            Self::ProxyUnreachable => "proxy_unreachable",
            Self::RunFailed => "run_failed",
            Self::TurnCap => "turn_cap",
            Self::TimeCap => "time_cap",
            Self::BudgetCap => "budget_cap",
        }
    }
}

/// Why a topic's generation failed (SPEC-046 R2, R5, R7): a closed set.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FailedReason {
    /// The topic's persona template lacks the daily-reading duty.
    FormUnregistered,
    /// The seed holds no note.
    SeedEmpty,
    /// Every note's anchor of a law seed is unusable.
    AnchorUnusableAll,
    /// The one repair failed the named gate too.
    GateFailed(ReadingGate),
    /// A configured route's agent was unavailable, for the named cause.
    AgentUnavailable(AgentCause),
}

/// The prefix of a `gate_failed` reason.
const GATE_FAILED: &str = "gate_failed:";
/// The prefix of an `agent_unavailable` reason.
const AGENT_UNAVAILABLE: &str = "agent_unavailable:";

impl FailedReason {
    /// Every reason, each gate and each cause included.
    #[must_use]
    pub fn every() -> Vec<Self> {
        let mut every = vec![
            Self::FormUnregistered,
            Self::SeedEmpty,
            Self::AnchorUnusableAll,
        ];
        every.extend(ReadingGate::ALL.map(Self::GateFailed));
        every.extend(AgentCause::ALL.map(Self::AgentUnavailable));
        every
    }

    /// The reason as the readings' tables store it.
    #[must_use]
    pub fn to_stored(self) -> String {
        match self {
            Self::FormUnregistered => "form_unregistered".to_owned(),
            Self::SeedEmpty => "seed_empty".to_owned(),
            Self::AnchorUnusableAll => "anchor_unusable_all".to_owned(),
            Self::GateFailed(gate) => format!("{GATE_FAILED}{}", gate.as_str()),
            Self::AgentUnavailable(cause) => format!("{AGENT_UNAVAILABLE}{}", cause.as_str()),
        }
    }

    /// The reason stored as `text`.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        Self::every()
            .into_iter()
            .find(|reason| reason.to_stored() == text)
    }
}

/// The one state a topic ends a study day in (R5).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TopicState {
    /// A reading passed every gate (SPEC-046).
    Ready,
    /// The scheduler queued no new card of the topic today.
    NoNewCards,
    /// The readings could not tell, for a closed reason of one class.
    CouldNotTell(CouldNotTell),
    /// Two study days passed without study (R4).
    Paused,
    /// The generation failed, for a closed reason (SPEC-046).
    Failed(FailedReason),
    /// No AI route is configured: a setting, never a failure (R12, ADR-054).
    AiRouteAbsent,
}

impl TopicState {
    /// Every state, with every reason each state can carry.
    #[must_use]
    pub fn every() -> Vec<Self> {
        let mut every = vec![Self::Ready, Self::NoNewCards];
        every.extend(CouldNotTell::ALL.map(Self::CouldNotTell));
        every.push(Self::Paused);
        every.extend(FailedReason::every().into_iter().map(Self::Failed));
        every.push(Self::AiRouteAbsent);
        every
    }

    /// The state's name as the readings' tables store it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::NoNewCards => "no_new_cards",
            Self::CouldNotTell(_) => "could_not_tell",
            Self::Paused => "paused",
            Self::Failed(_) => "failed",
            Self::AiRouteAbsent => "ai_route_absent",
        }
    }

    /// The class a could-not-tell state carries, and no other state.
    #[must_use]
    pub const fn class(self) -> Option<Class> {
        match self {
            Self::CouldNotTell(reason) => Some(reason.class()),
            _ => None,
        }
    }

    /// The reason a could-not-tell or failed state carries, as stored, and no other state.
    #[must_use]
    pub fn reason(self) -> Option<String> {
        match self {
            Self::CouldNotTell(reason) => Some(reason.as_str().to_owned()),
            Self::Failed(reason) => Some(reason.to_stored()),
            _ => None,
        }
    }

    /// Whether the state counts as a failure: `failed` alone. An absent AI route is a setting.
    #[must_use]
    pub const fn is_failure(self) -> bool {
        matches!(self, Self::Failed(_))
    }

    /// Whether the state is a refusal the health check counts (SPEC-050 R2): could-not-tell and
    /// failed, never an absent AI route, a pause or a quiet day.
    #[must_use]
    pub const fn refuses(self) -> bool {
        matches!(self, Self::CouldNotTell(_) | Self::Failed(_))
    }

    /// The state stored as `name`, `class` and `reason`, or `None` when the three do not name one
    /// state together: an unknown name or reason, a class the reason does not give, or a class or
    /// reason on a state that carries none.
    #[must_use]
    pub fn from_stored(name: &str, class: Option<&str>, reason: Option<&str>) -> Option<Self> {
        let state = match (name, reason) {
            ("ready", None) => Self::Ready,
            ("no_new_cards", None) => Self::NoNewCards,
            ("could_not_tell", Some(reason)) => Self::CouldNotTell(CouldNotTell::parse(reason)?),
            ("paused", None) => Self::Paused,
            ("failed", Some(reason)) => Self::Failed(FailedReason::parse(reason)?),
            ("ai_route_absent", None) => Self::AiRouteAbsent,
            _ => return None,
        };
        (state.class().map(Class::as_str) == class).then_some(state)
    }
}

impl fmt::Display for TopicState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.reason() {
            Some(reason) => write!(f, "{} ({reason})", self.name()),
            None => f.write_str(self.name()),
        }
    }
}

/// A resolution run's outcome (R8): the day set resolved, every topic paused, the run refused as a
/// whole for a could-not-tell reason, or (SPEC-046) no AI route configured.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RunOutcome {
    /// Every gate passed and the queue answered: each topic has its own state or day set.
    Resolved,
    /// Two study days passed without study: every topic is paused.
    Paused,
    /// The run was refused as a whole, before or at the queue, for this reason.
    CouldNotTell(CouldNotTell),
    /// No AI route is configured, so nothing was resolved (SPEC-046 R16).
    AiRouteAbsent,
}

impl RunOutcome {
    /// Every outcome, with every reason a refused run can carry.
    #[must_use]
    pub fn every() -> Vec<Self> {
        let mut every = vec![Self::Resolved, Self::Paused];
        every.extend(CouldNotTell::ALL.map(Self::CouldNotTell));
        every.push(Self::AiRouteAbsent);
        every
    }

    /// The outcome's name as `reading_runs` stores it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Resolved => "resolved",
            Self::Paused => "paused",
            Self::CouldNotTell(_) => "could_not_tell",
            Self::AiRouteAbsent => "ai_route_absent",
        }
    }

    /// The reason a refused run carries, and no other outcome.
    #[must_use]
    pub const fn reason(self) -> Option<CouldNotTell> {
        match self {
            Self::CouldNotTell(reason) => Some(reason),
            _ => None,
        }
    }

    /// The outcome stored as `name`, `class` and `reason`, or `None` when the three do not name one
    /// outcome together.
    #[must_use]
    pub fn from_stored(name: &str, class: Option<&str>, reason: Option<&str>) -> Option<Self> {
        let outcome = match (name, reason) {
            ("resolved", None) => Self::Resolved,
            ("paused", None) => Self::Paused,
            ("could_not_tell", Some(reason)) => Self::CouldNotTell(CouldNotTell::parse(reason)?),
            ("ai_route_absent", None) => Self::AiRouteAbsent,
            _ => return None,
        };
        let stored_class = outcome.reason().map(|reason| reason.class().as_str());
        (stored_class == class).then_some(outcome)
    }
}
