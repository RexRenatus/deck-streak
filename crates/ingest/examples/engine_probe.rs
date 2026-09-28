//! The engine probe: the binary ADR-022's size budget measures (SPEC-022 R2).
//!
//! It links every engine path the ingest port uses, so its stripped size is what the service's
//! binary pays for Anki's engine: resolving today's new-card queue, a normal sync and a full
//! download. `engine-measure.yml` builds it in release and strips it. Run by hand, it drives the
//! port:
//!
//! ```text
//! engine_probe queue <collection>
//! engine_probe sync <collection>
//! engine_probe download <collection>
//! ```
//!
//! The two sync commands read the endpoint from `DECKSTREAK_SYNC_ENDPOINT`, and the account from
//! `$CREDENTIALS_DIRECTORY/anki-sync-username` and `anki-sync-password`, as the service will
//! (ADR-010): a credential never arrives as an argument or an environment variable.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use deck_streak_ingest::engine::{AnkiEngine, RslibEngine, SyncLogin};

const USAGE: &str = "usage: engine_probe queue|sync|download <collection>";

#[allow(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "a command-line probe answers on its standard streams"
)]
fn main() -> ExitCode {
    let arguments: Vec<String> = env::args().skip(1).collect();
    match run(&arguments) {
        Ok(answer) => {
            println!("{answer}");
            ExitCode::SUCCESS
        }
        Err(why) => {
            eprintln!("engine_probe: {why}");
            ExitCode::FAILURE
        }
    }
}

fn run(arguments: &[String]) -> Result<String, String> {
    let [command, collection] = arguments else {
        return Err(USAGE.to_owned());
    };
    let collection = Path::new(collection);
    match command.as_str() {
        "queue" => {
            let queue = RslibEngine
                .new_card_queue(collection)
                .map_err(|error| error.to_string())?;
            Ok(format!(
                "{} new card(s) queued today across {} top-level deck(s)",
                queue.new_card_total(),
                queue.roots.len()
            ))
        }
        "sync" => {
            let outcome = runtime()?
                .block_on(RslibEngine.normal_sync(collection, &login()?))
                .map_err(|error| error.to_string())?;
            Ok(format!("{outcome:?}"))
        }
        "download" => {
            runtime()?
                .block_on(RslibEngine.full_download(collection, &login()?))
                .map_err(|error| error.to_string())?;
            Ok("the collection was downloaded".to_owned())
        }
        _ => Err(USAGE.to_owned()),
    }
}

fn runtime() -> Result<tokio::runtime::Runtime, String> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| "no tokio runtime could be built".to_owned())
}

/// The login the service will use: the endpoint is a setting, the account a credential.
fn login() -> Result<SyncLogin, String> {
    let endpoint = env::var("DECKSTREAK_SYNC_ENDPOINT")
        .map_err(|_| "the setting DECKSTREAK_SYNC_ENDPOINT is not set".to_owned())?;
    let credentials = env::var_os("CREDENTIALS_DIRECTORY")
        .map(PathBuf::from)
        .ok_or_else(|| "CREDENTIALS_DIRECTORY is not set".to_owned())?;
    let credential = |id: &str| {
        fs::read_to_string(credentials.join(id))
            .map(|value| value.strip_suffix('\n').unwrap_or(&value).to_owned())
            .map_err(|_| format!("the credential {id} is missing"))
    };
    Ok(SyncLogin::new(
        endpoint,
        credential("anki-sync-username")?,
        credential("anki-sync-password")?,
    ))
}
