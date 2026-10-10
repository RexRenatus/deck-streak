//! A duty's declaration and its caps (SPEC-043 R6), and the engine that runs one duty (R12, R16).
//!
//! The caps are a safety bound on one run, never a daily cap on readings.

use std::time::Duration;

/// The default turn cap of a run.
pub const DEFAULT_MAX_TURNS: u32 = 30;
/// The default budget cap of a run, in millionths of a US dollar (5 USD).
pub const DEFAULT_MAX_BUDGET_MICRO_USD: u64 = 5_000_000;
/// The default wall clock of a run, in seconds.
pub const DEFAULT_WALL_SECONDS: u64 = 1800;
/// The daily reading's wall clock, in seconds.
pub const DAILY_READING_WALL_SECONDS: u64 = 620;
/// The duty name the daily reading is recorded under.
pub const DAILY_READING: &str = "daily-reading";

/// The bounds one run is stopped at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DutyCaps {
    /// The most turns the run may take.
    pub max_turns: u32,
    /// The most the run may spend, in millionths of a US dollar.
    pub max_budget_micro_usd: u64,
    /// The wall clock the run is stopped at.
    pub wall_clock: Duration,
}

impl Default for DutyCaps {
    fn default() -> Self {
        Self {
            max_turns: DEFAULT_MAX_TURNS,
            max_budget_micro_usd: DEFAULT_MAX_BUDGET_MICRO_USD,
            wall_clock: Duration::from_secs(DEFAULT_WALL_SECONDS),
        }
    }
}

/// A duty's declaration: its name and its caps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DutySpec {
    /// The duty's name, as `agent_runs` records it.
    pub name: &'static str,
    /// The caps a run of it is stopped at.
    pub caps: DutyCaps,
}

impl DutySpec {
    /// The daily reading: the defaults, with the predecessor's per-topic wall clock.
    #[must_use]
    pub const fn daily_reading() -> Self {
        Self {
            name: DAILY_READING,
            caps: DutyCaps {
                max_turns: DEFAULT_MAX_TURNS,
                max_budget_micro_usd: DEFAULT_MAX_BUDGET_MICRO_USD,
                wall_clock: Duration::from_secs(DAILY_READING_WALL_SECONDS),
            },
        }
    }
}

use deck_streak_kernel::{Clock, KernelError};

use crate::compose::{Parts, compose};
use crate::deck_gate::{DeckGate, DeckScope};
use crate::gate::{GateOutcome, OutputGate};
use crate::route::AiRoute;
use crate::runner::Runner;
use crate::runs::{AgentRuns, RunRecord};
use crate::verdict::{Cause, Delivered, Telemetry, Verdict, Withheld};

/// What an alert says (R11, R12). It carries a cause or a class and never the content.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AlertKind {
    /// A configured route failed, for the closed cause.
    Unavailable(Cause),
    /// The gate refused an output; only the class is named.
    Withheld(String),
}

/// One alert the engine raises, through the ONE router (SPEC-041).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentAlert {
    /// The duty that raised it.
    pub duty: String,
    /// What happened.
    pub kind: AlertKind,
}

/// The alert router, as the agent sees it. Coordination adapts the one router to this port.
pub trait Alerts: Send + Sync {
    /// Raises `alert`.
    fn raise(&self, alert: AgentAlert);
}

/// Where a passed output goes. Coordination adapts the vault to this port.
pub trait Vault: Send + Sync {
    /// Delivers `output` for `subject`.
    ///
    /// # Errors
    ///
    /// A refusal; the engine then reports the run unavailable and delivers nothing.
    fn deliver(&self, duty: &str, subject: &str, output: &str) -> Result<(), VaultRefused>;
}

/// The vault refused a delivery.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("the vault refused the delivery")]
pub struct VaultRefused;

/// One run's inputs.
#[derive(Clone, Debug)]
pub struct DutyInput<'a> {
    /// The persona template's id.
    pub template: &'a str,
    /// The subject the run is for.
    pub subject: &'a str,
    /// The composed pieces: the engine fills nothing in.
    pub parts: Parts<'a>,
    /// The deck scope of the cards in `parts`: each card's home and current deck (SPEC-381 R4).
    pub scope: DeckScope<'a>,
}

/// The engine: one duty run, from route check to record (R6, R9 to R12, R16).
pub struct DutyEngine<'a> {
    /// The route the host is configured with.
    pub route: AiRoute,
    /// The runner.
    pub runner: &'a dyn Runner,
    /// The deck gate, asked before the input gate, `compose` and the runner (SPEC-381 R4).
    pub deck_gate: &'a dyn DeckGate,
    /// The output gate.
    pub gate: &'a dyn OutputGate,
    /// The alert router.
    pub alerts: &'a dyn Alerts,
    /// The vault.
    pub vault: &'a dyn Vault,
    /// The record.
    pub runs: &'a AgentRuns,
    /// The clock.
    pub clock: &'a dyn Clock,
}

impl DutyEngine<'_> {
    /// Runs `duty` once and records how it ended.
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the record cannot be written.
    pub async fn run(
        &self,
        duty: &DutySpec,
        input: &DutyInput<'_>,
    ) -> Result<Verdict, KernelError> {
        let (verdict, telemetry) = self.decide(duty, input).await;
        self.runs
            .record(&RunRecord {
                duty: duty.name,
                template: input.template,
                subject: input.subject,
                verdict: &verdict,
                telemetry,
                at: self.clock.now(),
            })
            .await?;
        Ok(verdict)
    }

    async fn decide(&self, duty: &DutySpec, input: &DutyInput<'_>) -> (Verdict, Option<Telemetry>) {
        if !self.route.is_configured() {
            return (Verdict::AiRouteAbsent, None);
        }
        let _ = self.deck_gate.judge(input.scope).await;
        for untrusted in [input.parts.memory, input.parts.cards] {
            if let GateOutcome::Failed { class, findings } = self.gate.check_input(untrusted).await
            {
                return (self.withhold(duty, class, findings), None);
            }
        }
        let Ok(prompt) = compose(&input.parts) else {
            return (self.unavailable(duty, Cause::RefusedShape), None);
        };
        let reply = match self.runner.run(&prompt, &duty.caps).await {
            Ok(reply) => reply,
            Err(cause) => return (self.unavailable(duty, cause), None),
        };
        let telemetry = reply.telemetry;
        match self.gate.check(&reply.result, input.parts.template).await {
            GateOutcome::Failed { class, findings } => {
                (self.withhold(duty, class, findings), Some(telemetry))
            }
            GateOutcome::Passed => {
                if self
                    .vault
                    .deliver(duty.name, input.subject, &reply.result)
                    .is_err()
                {
                    return (self.unavailable(duty, Cause::RunFailed), Some(telemetry));
                }
                let delivered = Delivered {
                    output: reply.result,
                    telemetry,
                };
                (Verdict::Delivered(delivered), Some(telemetry))
            }
        }
    }

    fn unavailable(&self, duty: &DutySpec, cause: Cause) -> Verdict {
        self.alerts.raise(AgentAlert {
            duty: duty.name.to_owned(),
            kind: AlertKind::Unavailable(cause),
        });
        Verdict::Unavailable(cause)
    }

    fn withhold(&self, duty: &DutySpec, class: String, findings: Vec<String>) -> Verdict {
        self.alerts.raise(AgentAlert {
            duty: duty.name.to_owned(),
            kind: AlertKind::Withheld(class.clone()),
        });
        Verdict::Withheld(Withheld { class, findings })
    }
}
