//! `deckstreakd`: the one release binary of the service (SPEC-025 R1; ADR-010). Its first argument
//! names the role it runs, and each role but `data` is one systemd unit: `api`, `bot` (SPEC-026: the
//! Telegram bot's long poll), `job` (SPEC-027: `deckstreakd job <id>` runs one job of the table
//! and exits), and `mcp` (SPEC-119: the MCP adapter's server on a loopback address). `data` is
//! the owner's, run by hand on the host (SPEC-021 R8): `deckstreakd data export` writes the export
//! to standard output, and `deckstreakd data erase --confirm ERASE` erases. `preset` is the
//! owner's too (SPEC-387): `deckstreakd preset list` prints every preset, `deckstreakd preset
//! propose <id>` records a proposal that moves one preset to the scheduler's defaults, and
//! `deckstreakd preset verify <id>` settles it once the owner's change has synced.
//!
//! `main` installs the kernel's logging before anything else, so every line the process writes,
//! even a refusal to start, is a JSON event with its journal priority (SPEC-031 R1); it is then the
//! one reader of the process environment (SPEC-020 R10). A missing or unknown role exits 2 with a
//! usage line naming the roles and the jobs, and so does a `data` erase without its confirmation
//! word, before the database is opened; a role that refuses start or fails exits 1; a role that
//! stops on its shutdown signal exits 0. The `job` role exits with its runner's code (R7).
#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![warn(missing_docs, clippy::all)]

use std::ffi::OsString;
use std::process::ExitCode;
use std::time::Duration;

use anyhow::Context;
use deck_streak_coordination::jobs::{self, Job, TABLE};
use deck_streak_daemon::{role_api, role_bot, role_mcp};
use deck_streak_kernel::{Environment, Redactor, logging};

mod role_data;
mod role_job;
mod role_preset;

use deck_streak_ingest::preset::PresetCommand;
use role_data::{CONFIRMATION, DataCommand};

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
    /// The Telegram bot (SPEC-026).
    Bot,
    /// One job of the table, run once by its timer (SPEC-027).
    Job(Job),
    /// The owner's export or erase, run by hand (SPEC-021).
    Data(DataCommand),
    /// The MCP adapter's server (SPEC-119).
    Mcp,
    /// The owner's preset read and proposal, run by hand (SPEC-387).
    Preset(PresetCommand),
}

impl Role {
    /// Every role's name.
    const NAMES: [&'static str; 6] = ["api", "bot", "job", "data", "mcp", "preset"];

    /// The role `arguments` name: `api`, `bot` or `mcp` alone, `job` and the id of a job of the
    /// table, or `data` or `preset` and its command.
    fn from_arguments(arguments: &[OsString]) -> Option<Self> {
        match arguments {
            [name] if name == "api" => Some(Self::Api),
            [name] if name == "bot" => Some(Self::Bot),
            [name] if name == "mcp" => Some(Self::Mcp),
            [name, id] if name == "job" => id.to_str().and_then(jobs::job).map(Self::Job),
            [name, command @ ..] if name == "data" => {
                DataCommand::from_arguments(command).map(Self::Data)
            }
            [name, command @ ..] if name == "preset" => {
                role_preset::command(command).map(Self::Preset)
            }
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
            "usage: deckstreakd <role>, or deckstreakd job <id>, or deckstreakd data export, or \
             deckstreakd data erase --confirm {CONFIRMATION}, or deckstreakd preset \
             list|propose <id>|verify <id>; the roles are: {}; the jobs are: {}",
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

/// Runs `role` to its end, and returns the process's exit code. `redactor` is the log writer's: a
/// role registers every credential it loads with it, so no later line can carry one (SPEC-020 R12).
async fn run(role: Role, environment: &Environment, redactor: &Redactor) -> anyhow::Result<u8> {
    match role {
        Role::Api => role_api::run(environment, redactor)
            .await
            .context("the api role")
            .map(|()| 0),
        Role::Bot => role_bot::run(environment, redactor)
            .await
            .context("the bot role")
            .map(|()| 0),
        Role::Job(job) => role_job::run(environment, redactor, &job)
            .await
            .context("the job role"),
        Role::Data(command) => role_data::run(environment, command)
            .await
            .context("the data role"),
        Role::Mcp => role_mcp::run(environment, redactor)
            .await
            .context("the mcp role")
            .map(|()| 0),
        Role::Preset(command) => role_preset::run(environment, command)
            .await
            .context("the preset role"),
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::time::Duration;

    use deck_streak_ingest::preset::PresetCommand;

    use super::{EXIT_GRACE, Role};

    /// The role a command line of space-separated arguments names.
    fn parse(line: &str) -> Option<Role> {
        let arguments: Vec<OsString> = line.split(' ').map(OsString::from).collect();
        Role::from_arguments(&arguments)
    }

    #[test]
    fn the_preset_role_parses_its_three_commands_and_refuses_every_other_shape() {
        // SPEC-387 A11: the three commands of ADR-401 D10, each with the id it carries.
        assert!(
            matches!(
                parse("preset list"),
                Some(Role::Preset(PresetCommand::List))
            ),
            "preset list parsed to {:?}",
            parse("preset list")
        );
        assert!(
            matches!(
                parse("preset propose 1001"),
                Some(Role::Preset(PresetCommand::Propose(1001)))
            ),
            "preset propose 1001 parsed to {:?}",
            parse("preset propose 1001")
        );
        assert!(
            matches!(
                parse("preset verify 7"),
                Some(Role::Preset(PresetCommand::Verify(7)))
            ),
            "preset verify 7 parsed to {:?}",
            parse("preset verify 7")
        );
        let refused = [
            "preset",
            "preset list extra",
            "preset propose",
            "preset propose x",
            "preset propose 1 2",
            "preset verify",
            "preset verify 1.5",
            "preset remove 1",
            "presets list",
        ];
        for line in refused {
            assert!(parse(line).is_none(), "{line} parsed to {:?}", parse(line));
        }
        println!("examined {} refused shape(s)", refused.len());
        assert_eq!(Role::names(), "api, bot, job, data, mcp, preset");
    }

    #[test]
    fn the_exit_grace_is_one_second() {
        // How long a role that stopped while waiting for the open lock lingers at exit: the whole
        // value, so a longer wait slows every such stop and a shorter one abandons work at once.
        assert_eq!(EXIT_GRACE, Duration::from_secs(1));
    }
}
