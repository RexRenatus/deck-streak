# Red-first record: SPEC-192

SPEC-192 and ADR-192 were committed alone (2a2193a). The guard came next (4fd0748), against the
unchanged tree: its five fixture tests pass, and the one test that walks `crates/*/src` fails by
assertion. The tests, the fifteen rows and the changelog fragment then turned it green (a6a850f).
The replay ran the whole guard file on 4fd0748's tree: six tests, one red by assertion.

```red-first
A1: red at 4fd0748: Lists differ: ['analytics::LeechThreshold (src/settings.[1167 chars]er"'] != [] : First list contains 15 additional elements : 24 impl(s) examined
A1: green at a6a850f
A2: not red: the leech threshold already carried the literal, and the test spells it; the row S19211 proves the test kills the rewritten literal
A3: not red: the mini app URL already carried the literal; the row S19212 proves the test kills the rewritten literal
A4: not red: the API URL already carried the literal; the row S19213 proves the test kills the rewritten literal
A5: not red: the four ingest settings already carried their literals; the rows S19202 to S19205 prove the test kills each rewritten literal
A6: not red: the courses file path already carried the literal; the row S19206 proves the test kills the rewritten literal
A7: not red: the hour, offset, worker and credentials settings already carried their literals; the rows S19207 to S19210 prove the test kills each rewritten literal
A8: not red: the taxonomy path already carried the literal; the row S19214 proves the test kills the rewritten literal
A9: not red: the folder setting already carried the literal; the row S19201 proves the test kills the rewritten literal
A10: not red: the request file already carried the literal; the row S19215 proves the test kills the rewritten literal
```

## The population: 24 `impl Setting for`, and how each is pinned

The guard's 24 (`examined 24 Setting impl(s)`), from `git grep -n 'impl .*Setting for' -- crates |
grep /src/` at the base c814bbc. Plants are the middle letter to `Q` and `XX<shape>XX`, each run
with `cargo test -p <crate> --tests --no-fail-fast`.

| crate | impl | pinned by |
|---|---|---|
| agent | `RosterPath` | row S05766 (SPEC-057), test `persona::a_roster_outside_the_repository_fills_the_four_slots` |
| agent | `AiRoute` | row S04302 (SPEC-043), test `constants::the_ai_route_setting_states_its_shape`; both plants fail exactly that test |
| analytics | `LeechThreshold` | row S19211, test `rollup_metrics::a_malformed_leech_threshold_is_refused_naming_its_whole_shape` |
| api | `SocketSetting` | row S05772, test `listen::a_listen_setting_that_is_not_an_address_is_refused_naming_its_shape` |
| bot | `MiniAppUrl` | row S19212, test `commands::the_mini_app_url_is_https` |
| bot | `ApiUrl` | row S19213, test `transport::the_api_url_is_https_or_loopback_http` |
| daemon | `StateDirectory` | row S05751, test `roles::a_relative_state_directory_is_refused_naming_the_shape_it_must_have` |
| daemon | `NotifyAddress` | row S05755, test `lifecycle::tests::each_private_setting_states_its_shape` |
| daemon | `Micros` | row S05756, the same test |
| daemon | `Text` | row S05757, the same test |
| daemon | `RequestFile` | both plants survived (rc 0, no failure); row S19215, test `sync_request::a_relative_request_path_is_refused_naming_its_whole_shape` |
| identity | `Freshness` | pinned by `init_data::the_freshness_bound_is_read_from_its_setting`; no row |
| ingest | `SyncEndpoint` | row S19202, test `settings::each_malformed_setting_is_refused_naming_its_whole_shape` |
| ingest | `StateDirectory` | row S19203, the same test |
| ingest | `IncludeDecks` | row S19204, the same test |
| ingest | `LawDeckRoot` | row S19205, the same test |
| kernel | `CoursesPath` | row S19206, test `courses_config::the_courses_file_refuses_a_duplicate_or_overlapping_course` |
| kernel | `Hour` | row S19207, test `settings::a_malformed_setting_is_refused_by_name_without_its_value` |
| kernel | `UtcOffset` | row S19208, the same test |
| kernel | `OffloadWorkers` | row S19209, the same test |
| kernel | `CredentialsDirectory` | row S19210, the same test |
| readings | `TaxonomyPath` | row S19214, test `topics::a_taxonomy_file_that_names_a_deck_badly_is_refused_whole_and_never_quoted` |
| vault | `VaultRoot` | pinned by `confinement::the_vault_settings_refuse_by_name_and_never_by_value`; no row |
| vault | `FolderName` | row S19201, the same test |

Fifteen rows in this delivery's band (S19201 to S19215); seven implementations pinned before it by
rows of other bands (five of them by #354) and two by tests alone.
