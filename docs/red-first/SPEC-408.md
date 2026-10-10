# Red-first record: SPEC-408

SPEC-408 (R1 to R9, A1 to A5). The SPEC, ADR-422 and the schematic were committed first
(`26e165437cf67cfbfe15e03528457c1de04513a8`); the tests of A1 to A5 and the three pins that grow (the law block's mastery literal, the
bot's progress golden and the bot role's progress reply) were then written over a tree that carries
no mastery sentence, and each of the five read red for its own criterion. Every red below is quoted
from the run at that commit, locally. A1 to A3 are Vitest tests, A4 and A5 are cargo tests.

```text
A1: pnpm exec vitest run web/app/src/lib/law/LawBlock.test.ts -t "the mastery line says what law mastery measures and no other line does"
A2: pnpm exec vitest run web/app/src/lib/progress/CourseLadder.test.ts -t "each course says what its mastery measures right after its summary"
A3: pnpm exec vitest run web/app/src/lib/mastery-about-locales.test.ts -t "every locale carries both mastery descriptions with the English figures"
A4: cargo test -p deck-streak-bot --test progress_commands -- --exact the_progress_reply_ends_with_the_course_mastery_description
A4: cargo test -p deck-streak-bot --test progress_commands -- --exact progress_shows_each_course_band_and_mastery
A4: cargo test -p deck-streak-daemon --test role_bot -- --exact the_bot_role_answers_progress_from_the_courses_its_settings_name
A5: cargo test -p deck-streak-curriculum --test mastery_about -- --exact the_law_mastery_description_states_the_pillars_own_figures
```

```red-first
A1: red at 26e165437cf67cfbfe15e03528457c1de04513a8: AssertionError: expected '' to be 'Law mastery starts at 100% and drops ...' (the mastery line holds no description span)
A1: green at ce1edda232f8f29ad90cdbe81591748bce465338
A2: red at 26e165437cf67cfbfe15e03528457c1de04513a8: AssertionError: expected null to be 'course' (the summary's next sibling is the band list)
A2: green at ce1edda232f8f29ad90cdbe81591748bce465338
A3: red at 26e165437cf67cfbfe15e03528457c1de04513a8: AssertionError: expected { en: [ ...(3) ], ...(6) } to deeply equal { en: [], 'zh-Hans': [], ...(5) } (no locale holds the two keys)
A3: green at ce1edda232f8f29ad90cdbe81591748bce465338
A4: red at 26e165437cf67cfbfe15e03528457c1de04513a8: assertion left == right failed: left: Some("<flag> <b>Beta</b>: A2, 43% mastery, no unit yet"), right: Some("<i>Mastery is an estimate from your reviews: ...</i>") (the last line is the Beta course's)
A4: green at ce1edda232f8f29ad90cdbe81591748bce465338
A5: red at 26e165437cf67cfbfe15e03528457c1de04513a8: assertion left == right failed: left: [], right: ["100", "3", "70"] (the sentence is absent)
A5: green at ce1edda232f8f29ad90cdbe81591748bce465338
```
