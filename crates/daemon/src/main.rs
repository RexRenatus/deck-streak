//! `deckstreakd`: the one release binary of the service (SPEC-025 R1; ADR-010). Its first argument
//! names the role it runs, and each role is one systemd unit: `api`, `job` (SPEC-027: `deckstreakd
//! job <id>` runs one job of the table and exits), and `bot` with SPEC-026.
//!
//! `main` installs the kernel's logging before anything else, so every line the process writes,
//! even a refusal to start, is a JSON event with its journal priority (SPEC-031 R1); it is then the
//! one reader of the process environment (SPEC-020 R10). A missing or unknown role exits 2 with a
//! usage line naming the roles and the jobs; a role that refuses start or fails exits 1; a role that
//! stops on its shutdown signal exits 0. The `job` role exits with its runner's code (R7).
#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![warn(missing_docs, clippy::all)]

use std::ffi::OsString;
use std::process::ExitCode;
use std::time::Duration;

use anyhow::Context;
use deck_streak_coordination::jobs::{self, Job, TABLE};
use deck_streak_daemon::role_api;
use deck_streak_kernel::{Environment, Redactor, logging};

mod role_job;

/// The exit code of a start with no role, an unknown one, or arguments the role does not take.
const USAGE: u8 = 2;

/// How long the runtime waits, as the process exits, for blocking work still running: a role that
/// stopped while waiting for the database's open lock leaves that wait behind.
const EXIT_GRACE: Duration = Duration::from_secs(1);

/// The roles, by the name the first argument gives.
#[derive(Clone, Copy, Debug)]
enum Role {
    /// The HTTP service the Mini App calls (SPEC-025).
    Api,
    /// One job of the table, run once by its timer (SPEC-027).
    Job(Job),
}

impl Role {
    /// Every role's name.
    const NAMES: [&'static str; 2] = ["api", "job"];

    /// The role `arguments` name: `api` alone, or `job` and the id of a job of the table.
    fn from_arguments(arguments: &[OsString]) -> Option<Self> {
        match arguments {
            [name] if name == "api" => Some(Self::Api),
            [name, id] if name == "job" => id.to_str().and_then(jobs::job).map(Self::Job),
            _ => None,
        }
    }

    /// The roles' names, for the usage line.
    fn names() -> String {
        Self::NAMES.join(", ")
    }

    /// The table's jobs, for the usage line.
    fn jobs() -> String {
        TABLE.map(|job| job.id).join(", ")
    }
}

fn main() -> ExitCode {
    let redactor = Redactor::new();
    if logging::install(&redactor).is_err() {
        // Unreachable in a fresh process, and with no subscriber there is nowhere to say so.
        return ExitCode::FAILURE;
    }
    let arguments: Vec<OsString> = std::env::args_os().skip(1).collect();
    let Some(role) = Role::from_arguments(&arguments) else {
        tracing::error!(
            "usage: deckstreakd <role>, or deckstreakd job <id>; the roles are: {}; the jobs are: {}",
            Role::names(),
            Role::jobs()
        );
        return ExitCode::from(USAGE);
    };
    let environment = Environment::from_vars(std::env::vars_os());
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            tracing::error!(%error, "the async runtime could not be built");
            return ExitCode::FAILURE;
        }
    };
    let outcome = runtime.block_on(run(role, &environment, &redactor));
    runtime.shutdown_timeout(EXIT_GRACE);
    match outcome {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            tracing::error!(
                error = format!("{error:#}"),
                "the role stopped with an error"
            );
            ExitCode::FAILURE
        }
    }
}

/// Runs `role` to its end, and returns the process's exit code.
async fn run(role: Role, environment: &Environment, redactor: &Redactor) -> anyhow::Result<u8> {
    match role {
        Role::Api => role_api::run(environment)
            .await
            .context("the api role")
            .map(|()| 0),
        Role::Job(job) => role_job::run(environment, redactor, &job)
            .await
            .context("the job role"),
    }
}
