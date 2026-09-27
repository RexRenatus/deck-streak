//! The `api` role (SPEC-025).
//!
//! STUB for the red-first commit: the role returns at once.

use deck_streak_api::ApiError;
use deck_streak_kernel::{Environment, SettingsError};

use crate::wiring::WiringError;

/// Why the `api` role stopped with an error.
#[derive(Debug, thiserror::Error)]
pub enum ApiRoleError {
    /// A kernel setting refused start.
    #[error(transparent)]
    Settings(#[from] SettingsError),
    /// The API refused start or stopped serving.
    #[error(transparent)]
    Api(#[from] ApiError),
    /// The shutdown signal's handlers could not be installed.
    #[error("the role could not install its shutdown signal handlers")]
    Signals(#[source] std::io::Error),
    /// The database could not be opened.
    #[error("the database could not be opened")]
    Database(#[source] WiringError),
}

/// Runs the `api` role.
///
/// # Errors
///
/// Every refusal of [`ApiRoleError`].
pub async fn run(env: &Environment) -> Result<(), ApiRoleError> {
    let _ = env;
    Ok(())
}
