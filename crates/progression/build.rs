//! Arms the settle census's probe (SPEC-072 A12, ADR-197): when the census compiles the workspace
//! it sets `SETTLE_CENSUS`, and progression alone is then compiled with the `settle_census` cfg,
//! under which `settle` carries a deprecation that rustc reports at every use. No other crate sees
//! the cfg, so no other crate compiles differently under the census.

#[allow(
    clippy::print_stdout,
    reason = "cargo reads a build script's instructions from its standard output"
)]
fn main() {
    println!("cargo::rustc-check-cfg=cfg(settle_census)");
    println!("cargo::rerun-if-env-changed=SETTLE_CENSUS");
    if std::env::var_os("SETTLE_CENSUS").is_some() {
        println!("cargo::rustc-cfg=settle_census");
    }
}
