//! Rebuilds the kernel when a migration is added or changed, so `MIGRATOR` always embeds exactly
//! the files in the workspace's `migrations/` directory (SPEC-020 R15, A21).

#[allow(
    clippy::print_stdout,
    reason = "cargo reads a build script's instructions from its standard output"
)]
fn main() {
    println!("cargo:rerun-if-changed=../../migrations");
}
