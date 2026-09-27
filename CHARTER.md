# DeckStreak charter

## Mission

DeckStreak turns the reviews its owner already does in Anki into XP, levels, streaks, quests and
badges, in a Telegram Mini App and its bot. It reads a private copy of the owner's collection
from the owner's own Anki sync server, scores every study day, and speaks through one policy that
decides when to celebrate, when to nudge and when to stay quiet. Its flagship is the **daily
pre-study reading**: each morning, one short text per topic that primes the new cards Anki will
show that day, written by a named mentor for that subject, read in the Mini App and archived to
the owner's second-brain vault.

DeckStreak is a full-parity successor to its predecessor, a private single-user service ("v9"),
rebuilt as a public, AGPL-licensed Rust workspace with a SvelteKit front end. The predecessor is
the behavioural oracle: its numbers are proved, never re-derived.

## Hard constraints

These bind every delivery. A change that needs to break one is an owner decision recorded in an
ADR before the code, never a builder's call.

### Scope

1. **Full parity.** Every feature of the predecessor is either built with its acceptance criteria
   met, or excluded with a recorded reason (SPEC-001's parity matrix). Nothing is dropped
   silently, and a feature with no production caller in the predecessor is excluded with the
   reason "inert" until the owner revives it.
2. **Two surfaces, one policy.** The Mini App and the bot are two views of one service. Every
   celebration and nudge passes through ONE router, so an event can never celebrate twice.
3. **One small host.** The backend runs as systemd units behind Caddy on one small VM (two vCPU,
   under 2 GiB of RAM), beside the services already there, within a stated memory budget. The VM
   never compiles: binaries are built elsewhere and deployed as artifacts, from a SemVer tag on
   `main` only.
4. **Pull, then read.** DeckStreak syncs a private copy of the collection and reads it read-only.
   The skip day is the ONLY write back to Anki.

### The game's rules that must survive the port

5. **Coins are the only confiscable stake.** Streaks, XP, levels, badges and CEFR progress are
   structurally unconfiscatable; no penalty may touch them.
6. **A false fine is worse than a missed fine.** Every time-boxed verdict is revisable against
   late-arriving reviews, and a wrong fine refunds itself with a correction message.
7. **The day turns over at 04:00 local, not midnight**, and a digest never fires before the day
   it reports has closed (`digest_hour >= rollover_hour`). Every screen shows the server's study
   day.
8. **The game math ports verbatim**, proved by the parity oracle: the per-review XP table with
   round-half-to-even, the level curve `50L² − 50L`, the five-pillar score, streak freezes,
   chest odds 70/22/7/1 with pity at 8, 14 and 40, the coin mint and the daily loss cap, and the
   Bloom-tier multiplier T1–T4 = 1/2/3/5 on the law track.
9. **Idempotency guards travel with their feature**: the cron-fire claim and release with every
   catch-up notification, the drill grant ledger with every drill grant, post-once-or-update with
   any external datapoint, and one stored roll per random event. A feature ported without its
   guard reproduces a bug that was already fixed.
10. **The eleven anti-goals are one block**: no imposed penalty; no confiscation beyond coins; no
    fabricated near-miss or loss disguised as a win; no pretending a random outcome was chosen;
    no claim the sync cannot honour; no Screen Time enforcement; no tunnels or opened inbound
    ports; no lock-screen enforcement; no unbounded loss, faucet or notification volume; no
    dishonest copy; no secret, address or forward-looking date on anything public.

### Privacy and the public repository

11. **The repository is public; the learner is private.** No IP address, hostname that encodes
    one, cloud project id, secret name, chat id, VM name, host path or personal data (review
    data, deck or note text, study targets, dates or plans) enters the repository, an issue, a
    pull request or a commit message. A personal default is configuration with a neutral example
    value. Test fixtures are synthetic. The scrubber's deny-list runs on the tree, the history and
    every issue body.
12. **No forward-looking dates or timelines anywhere public, and none in any AI text.** A public
    page shows what already happened, never what is planned.
13. **Export and erase are symmetric, and that is tested.** The same set of tables is exported and
    erased; singletons are reset in place; the cron-fire ledger and the schema version table are
    exempt from both; offsite copies that survive an erase are disclosed.
14. **Owner-only is a safety property.** The service answers one owner. The bot gates every
    command on the owner's identity, and the Mini App validates Telegram `initData` on the server
    and pins it to the owner's user id, which is configuration held in a secret. Commands with
    stakes or destructive effects (panic, pardon, contracts, the agent's on-demand run) carry the
    same gate.
15. **No secret on disk or in a repository.** Secrets live in Secret Manager and reach a process
    as a systemd credential at start. Never in an environment variable, argv, a log line, a
    database row or a committed file.

### The AI agent

16. **An optional AI route, fail closed (ADR-015, ADR-054).** The AI route is off unless the owner
    enables one, and DeckStreak is whole without it: every AI duty then records `ai_route_absent`,
    the surfaces say readings are not enabled, and nothing alerts. When enabled through the owner's
    subscription proxy, the agent runs on the VM with its own device key, reaches the proxy over a
    reverse tunnel opened from the maintainer's machine, and never uses an `apiKeyHelper`. Each run
    is capped in turns and time. When an enabled route, the tunnel or the gate fails, the AI step
    says so and changes nothing: the digest goes out in its deterministic form with a line saying
    coaching was unavailable, and the vault is untouched. A silent skip is a defect.
17. **Every AI output is gated before delivery** by the packs' blocking checks (no dates, the
    scrubber, memory scope, no human claim, the output contract; citations for law). Untrusted
    text (cards, vault notes, the learner's writing) is fenced as data, and an agent reading it
    holds no tool that can carry data out.
18. **Personas are public templates and a private roster.** The repository ships neutral
    templates; names, bios, the subjects run and every weak spot stay in private configuration.
    A persona reads the memory of its own subject only, and never the journal.

### The readings

19. **The owner's reading rules** (SPEC-001 §readings): fail loud and never write a placeholder;
    a law reading is an 800–1500-word primer scaled to its new cards; readings generate whenever
    the last sync succeeded, pause after two days without study, and offer one comeback reading
    per lapse; "I read it" is written only on the owner's tap and never cleared by code; 40 XP on
    read plus 60 when 80% of its new cards are studied within two study days, at most 100 per
    reading, granted once; the Mini App is the primary surface and the vault keeps an archive
    copy; vault writes are atomic and pass the rails that block executable content; no daily cap
    on readings, full telemetry, and the agent's per-run caps as a safety bound.

### How the work is done

20. **Spec, schematic, ADR, tests red first, implementation, gate** (the sdd and tdd packs, and
    [CLAUDE.md](CLAUDE.md)). The crate graph is the context map (the ddd pack). Every pack
    DeckStreak consumes is wired into its gate or CI.
21. **`main` changes only by a release pull request from `dev`.** Builders open pull requests into
    `dev`; required checks are the bar; nobody force-pushes or deletes a protected branch.
