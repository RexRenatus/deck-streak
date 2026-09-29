//! A duty run's verdict (SPEC-043 R12): delivered, withheld, or unavailable with a closed cause.
//!
//! The verdict is `#[must_use]`: a caller cannot ignore how a run ended. Its set of causes is
//! closed on purpose, so a failure always names why and a new cause is a SPEC amendment.

/// Why a run that was meant to happen did not deliver: the closed set of R12.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Cause {
    /// The device key credential is missing (the runner's exit 1).
    KeyMissing,
    /// The runner refused the launch's shape (exit 2).
    RefusedShape,
    /// The proxy rejected the key (exit 3).
    KeyRejected,
    /// The proxy's roster is exhausted (exit 4).
    CapacityExhausted,
    /// The proxy or its tunnel could not be reached (exit 5).
    ProxyUnreachable,
    /// Claude Code failed or returned an error result (exit 6).
    RunFailed,
    /// The run reached its turn cap.
    TurnCap,
    /// The run reached its wall clock.
    TimeCap,
    /// The run reached its budget cap.
    BudgetCap,
}

impl Cause {
    /// Every cause, in the order R12 names them.
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

    /// The cause's name as `agent_runs` stores it and the alert carries it.
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

/// What a run measured, kept in `agent_runs` and never in a delivered text.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Telemetry {
    /// The turns the run took.
    pub turns: u32,
    /// The input tokens the CLI reported.
    pub input_tokens: u64,
    /// The output tokens the CLI reported.
    pub output_tokens: u64,
    /// The CLI's cost estimate, in millionths of a US dollar.
    pub cost_micro_usd: u64,
    /// How long the run took, in milliseconds.
    pub duration_ms: u64,
}

/// An output every blocking class passed, and what the run measured.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Delivered {
    /// The output, handed to the vault.
    pub output: String,
    /// The run's telemetry.
    pub telemetry: Telemetry,
}

/// An output a blocking class failed: the class and its finding lines, never the output's text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Withheld {
    /// The failing class, as `pack:class`.
    pub class: String,
    /// The probe's finding lines for the class.
    pub findings: Vec<String>,
}

/// How a duty run ended (R12, R16).
#[must_use]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// The output passed the gate and was delivered.
    Delivered(Delivered),
    /// The gate refused the output.
    Withheld(Withheld),
    /// A configured route failed, for the closed cause.
    Unavailable(Cause),
    /// No AI route is configured: nothing ran, and nothing is alerted (R16).
    AiRouteAbsent,
}

impl Verdict {
    /// The verdict's name as `agent_runs` stores it.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Delivered(_) => "delivered",
            Self::Withheld(_) => "withheld",
            Self::Unavailable(_) => "unavailable",
            Self::AiRouteAbsent => "ai_route_absent",
        }
    }
}
