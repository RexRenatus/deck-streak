//! Fakes shared by the agent's integration tests. Test support only.
#![allow(dead_code, clippy::expect_used)]

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use deck_streak_agent::deck_gate::{CardDecks, DeckFuture, DeckGate, DeckScope, DeckVerdict};
use deck_streak_agent::duty::DutyCaps;
use deck_streak_agent::duty::{AgentAlert, Alerts, Vault, VaultRefused};
use deck_streak_agent::gate::{GateFuture, GateOutcome, OutputGate};
use deck_streak_agent::runner::{RunFuture, RunReply, Runner};
use deck_streak_agent::verdict::{Cause, Telemetry};

/// Alerts, recorded.
#[derive(Default)]
pub struct RecordedAlerts(pub Mutex<Vec<AgentAlert>>);

impl Alerts for RecordedAlerts {
    fn raise(&self, alert: AgentAlert) {
        self.0.lock().expect("alerts").push(alert);
    }
}

/// A vault that keeps what it is given, or refuses.
#[derive(Default)]
pub struct RecordedVault {
    pub delivered: Mutex<Vec<String>>,
    pub refuse: bool,
}

impl Vault for RecordedVault {
    fn deliver(&self, _duty: &str, _subject: &str, output: &str) -> Result<(), VaultRefused> {
        if self.refuse {
            return Err(VaultRefused);
        }
        self.delivered
            .lock()
            .expect("vault")
            .push(output.to_owned());
        Ok(())
    }
}

/// A runner that answers with a fixed reply, and counts its launches.
pub struct FixedRunner {
    pub answer: Result<String, Cause>,
    pub launches: Mutex<u32>,
}

impl FixedRunner {
    pub fn replying(text: &str) -> Self {
        Self {
            answer: Ok(text.to_owned()),
            launches: Mutex::new(0),
        }
    }
    pub fn failing(cause: Cause) -> Self {
        Self {
            answer: Err(cause),
            launches: Mutex::new(0),
        }
    }
}

impl Runner for FixedRunner {
    fn run<'a>(&'a self, _prompt: &'a str, _caps: &'a DutyCaps) -> RunFuture<'a> {
        *self.launches.lock().expect("launches") += 1;
        let answer = self.answer.clone().map(|result| RunReply {
            result,
            telemetry: Telemetry {
                turns: 3,
                input_tokens: 100,
                output_tokens: 50,
                cost_micro_usd: 12_000,
                duration_ms: 900,
            },
        });
        Box::pin(async move { answer })
    }
}

/// A gate with a fixed answer.
pub struct FixedGate(pub GateOutcome);

impl OutputGate for FixedGate {
    fn check<'a>(&'a self, _output: &'a str, _template: &'a str) -> GateFuture<'a> {
        let outcome = self.0.clone();
        Box::pin(async move { outcome })
    }
    fn check_input<'a>(&'a self, _input: &'a str) -> GateFuture<'a> {
        Box::pin(async { GateOutcome::Passed })
    }
}

/// A deck gate with a fixed verdict, which records every scope it was asked to judge.
pub struct FixedDeckGate {
    pub verdict: DeckVerdict,
    pub asked: Mutex<Vec<Vec<CardDecks>>>,
}

impl FixedDeckGate {
    /// The gate the shipped tests run under: it admits every scope (SPEC-381 R4).
    pub fn admitting() -> Self {
        Self::answering(DeckVerdict::Admitted)
    }
    pub fn answering(verdict: DeckVerdict) -> Self {
        Self {
            verdict,
            asked: Mutex::new(Vec::new()),
        }
    }
}

impl DeckGate for FixedDeckGate {
    fn judge<'a>(&'a self, scope: DeckScope<'a>) -> DeckFuture<'a> {
        self.asked.lock().expect("asked").push(scope.cards.to_vec());
        let verdict = self.verdict;
        Box::pin(async move { verdict })
    }
}

/// Writes an executable-by-bash script `body` under `dir` and returns its path.
pub fn script(dir: &Path, name: &str, body: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, format!("#!/usr/bin/env bash\n{body}\n")).expect("a script");
    path
}
