//! The engine's own sync server, for the web engine's browser tests (SPEC-364 B1 to B3, ADR-375
//! D20). The Playwright config starts it as its second web server, configured by the `SYNC_*`
//! environment it builds (a synthetic user, the loopback host, a port and an empty base), and waits
//! for its `/health`; the Vite config forwards `/anki-sync/` to it, so the Worker's sync reaches the
//! engine's real protocol through the page's own origin.
//!
//! It is the engine's server unchanged: this file holds no logic of its own to test.

use std::process::{ExitCode, Termination};

use anki::sync::http_server::SimpleServer;

fn main() -> ExitCode {
    SimpleServer::run().report()
}
