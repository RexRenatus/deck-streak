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
//! `$CREDENTIALS_DIRECTORY/anki-sync-username` and `anki-sync-password` through the kernel's
//! loader, as the sync's login does (ADR-010): a credential never arrives as an argument or an
//! environment variable, and a missing or empty one refuses by its id (SPEC-066 R6).

use std::env;
use std::path::Path;
use std::process::ExitCode;

use deck_streak_ingest::engine::{AnkiEngine, RslibEngine, SyncLogin};
use deck_streak_ingest::settings::{SYNC_PASSWORD, SYNC_USERNAME};
use deck_streak_kernel::{CredentialLoader, CredentialsDirectory, Redactor};

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

/// The login the service will use: the endpoint is a setting, the account two credentials the
/// kernel's loader reads by their ids, each refusal answered with its message, which names the id.
fn login() -> Result<SyncLogin, String> {
    let endpoint = env::var("DECKSTREAK_SYNC_ENDPOINT")
        .map_err(|_| "the setting DECKSTREAK_SYNC_ENDPOINT is not set".to_owned())?;
    let directory = env::var_os("CREDENTIALS_DIRECTORY")
        .ok_or_else(|| "CREDENTIALS_DIRECTORY is not set".to_owned())?;
    let directory = CredentialsDirectory::new(directory)
        .ok_or_else(|| "CREDENTIALS_DIRECTORY is not an absolute path".to_owned())?;
    let loader = CredentialLoader::new(directory, Redactor::new());
    let username = loader
        .load(SYNC_USERNAME)
        .map_err(|refusal| refusal.to_string())?;
    let password = loader
        .load(SYNC_PASSWORD)
        .map_err(|refusal| refusal.to_string())?;
    Ok(SyncLogin::new(
        endpoint,
        username.expose(),
        password.expose(),
    ))
}
