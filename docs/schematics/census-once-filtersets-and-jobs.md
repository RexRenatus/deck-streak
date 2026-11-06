# The census set, the engine set and the rest: filtersets, stages and jobs (SPEC-632)

Drawn from this change's `scripts/check.sh` and `.github/workflows/ci.yml`. Three nextest
filtersets split the workspace's tests;
four stages of `scripts/check.sh` read them; four `ci.yml` jobs run those stages, each a need of
the aggregate `ci`.

```mermaid
flowchart LR
    subgraph sets["filtersets in scripts/check.sh"]
        engine_set["ENGINE_TESTS: the sync and engine_budget binaries"]
        census_set["CENSUS_TESTS: the xp_census binary"]
        rest_set["the rest: not (ENGINE_TESTS | CENSUS_TESTS)"]
    end
    subgraph stages["stages"]
        st_test["test, test profile"]
        st_release["test-release, release profile"]
        st_engine["test-engine, one slice of m/n"]
        st_census["test-census, no slice"]
    end
    subgraph jobs["ci.yml jobs"]
        j_rust["rust"]
        j_release["release"]
        j_engine["engine, two slices"]
        j_census["census"]
        j_ci["ci, the aggregate"]
    end
    rest_set --> st_test
    rest_set --> st_release
    engine_set --> st_engine
    census_set --> st_census
    st_test --> j_rust
    st_release --> j_release
    st_engine --> j_engine
    st_census --> j_census
    j_rust --> j_ci
    j_engine --> j_ci
    j_census --> j_ci
    j_release --> j_ci
```

Each test of the workspace runs in exactly one stage of a profile: the rest in `test` and again in
`test-release`, the engine set in `test-engine`, and the census set once, in `test-census`, in the
test profile. `test-engine` and `test-census` derive their `--test` targets from their own set's
`binary_id(=<package>::<target>)` terms, so cargo builds only those targets.

The `census` job restores the Rust cache keyed on the toolchain pin and `Cargo.lock` and saves
none, as the engine and release jobs do, and uploads its stage logs as `check-stage-logs-census`.
