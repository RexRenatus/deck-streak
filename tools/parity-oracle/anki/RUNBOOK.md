# Runbook: the second oracle's one run

The engine's goldens are written once, by the owner or the maintainer the owner names, in a sandbox, never by a
builder agent and never by CI (ADR-516 D4). This runbook builds the sandbox, proves it has no route out, runs the
driver once, and lists what the run record holds. The run record stays in the owner's private evidence; only the
goldens, the census digests and the driver's pin line reach this tree.

## Inputs

| input | where it comes from | mounted |
|---|---|---|
| the DeckStreak tree | a clean `git archive` export of the commit under test, never the working checkout | read-only |
| the harness source | the owner's private evidence; its `tree_digest` is the pin | read-only |
| the engine | cargo's registry and git checkouts at the pin `dca72f352730abb4222389cc148e46387c17f97f`, never the whole cargo home | read-only |
| the output folder | a new empty folder | the only writable mount |
| a temporary home | a fresh empty folder for cargo's target and the harness's throwaway collections | writable, removed after |

The harness's manifest has an empty `[workspace]` table and the package name `parity-oracle-anki-harness`; it builds
`--offline --locked` against the pinned engine. Its author is the dirty side: it reads the engine's API and source to
drive it, builds every collection through the engine's own API from the corpus's DeckStreak-authored note types,
deletes the note types the engine adds by itself before it builds anything, answers every row by its corpus index
(never an id the engine assigns, never the run's clock), and answers an engine refusal with a short lower-case label
(never the engine's words).

## Steps

1. **Pin the harness.** Outside the sandbox, print the source's digest and set `HARNESS_SOURCE_SHA256` in
   `tools/parity-oracle/anki/generate.py` to it, and commit the pin. The sandbox's tree is a `git archive` export of
   that commit, so the pin is committed before the run and the goldens follow it in a second commit; every golden's
   `generator_sha256` is the pinned driver's digest either way:

   ```sh
   python3 tools/parity-oracle/anki/generate.py --digest-harness PATH/TO/HARNESS/SOURCE
   git archive --format=tar PINNED_COMMIT | tar -x -C EXPORT
   ```

2. **Plant the network control.** Outside the sandbox, start a listener on the host's loopback and connect to it; the
   connection must be accepted (the positive control):

   ```sh
   python3 -m http.server --bind 127.0.0.1 PORT --directory EMPTY/FOLDER &
   curl --max-time 5 --silent --output /dev/null --write-out '%{http_code}\n' http://127.0.0.1:PORT/
   ```

   Then plant a canary file outside every mount (`CANARY`) and read it outside the sandbox.

3. **Enter the sandbox** with an unshared network namespace, the inputs read-only and one writable output folder, as
   an unprivileged user. With bubblewrap, for example:

   ```sh
   bwrap --unshare-net --unshare-pid --die-with-parent \
     --ro-bind /usr /usr --ro-bind /etc /etc --symlink usr/lib /lib --symlink usr/lib64 /lib64 \
     --symlink usr/bin /bin --proc /proc --dev /dev --tmpfs /tmp \
     --ro-bind EXPORT /work/tree --ro-bind HARNESS_SOURCE /work/harness \
     --ro-bind CARGO_REGISTRY /work/cargo/registry --ro-bind CARGO_GIT /work/cargo/git \
     --bind HOME_TMP /work/home --bind OUT /work/out \
     --setenv HOME /work/home --setenv CARGO_HOME /work/cargo --setenv PYTHONDONTWRITEBYTECODE 1 \
     /bin/bash
   ```

   The mechanism on the host (bubblewrap or `systemd-run` with `PrivateNetwork=yes`) is the operator's; the controls
   below are what prove it.

4. **Prove the sandbox.** Inside it, the same connection to `127.0.0.1:PORT` must fail, and `CANARY` must be
   unreadable. Record both failures and the step 2 successes. A connection that succeeds inside stops the run.

5. **Build the harness**, inside, with cargo's offline mode and the pinned lock, into the temporary home; record
   `rustc --version` and the binary's sha256.

6. **Run the driver once**, inside:

   ```sh
   python3 /work/tree/tools/parity-oracle/anki/generate.py --harness HARNESS_BINARY \
     --harness-source /work/harness --engine-source /work/cargo/git --out /work/out
   ```

   Exit 0 prints each file written and `examined N case(s) of M function(s)`. Exit 2 is a refusal before the harness
   started (a writable source, an unpinned or different harness, a registry that fails its checks); exit 3 is a run
   that failed and wrote nothing (another engine, a protocol fault, a refusal in words, a golden holding a stock
   line).

7. **Leave the sandbox** and copy `OUT/goldens/*.json` into `tools/parity-oracle/goldens/` and `OUT/stock-lines.sha256`
   into `tools/parity-oracle/anki/stock-lines.sha256`. Write the schema snapshot from the fresh-collection golden, then
   commit the goldens, the census and the snapshot together, and run the tests that read them:

   ```sh
   python3 tools/parity-oracle/anki/snapshot.py
   python3 -m unittest discover -s tools/parity-oracle -p 'test_*.py'
   cargo test -p deck-streak-ingest --test engine_goldens
   ```

   The host's sandbox may also be an egress-closed virtual machine guest that mounts only the export and cargo's
   registry and git checkouts, read-only, and one output folder; steps 2 and 4's controls prove it the same way, the
   accepted connection and the readable canary on the host, the refused ones in the guest.

## The run record

Kept in the owner's private evidence, never in this tree:

- the step 2 acceptance and the step 4 refusal of the planted connection, and the canary read outside and refused
  inside;
- the mounts, each with its mode, showing the sources read-only and the output folder the only writable one;
- the harness source's digest (equal to the pin), `rustc --version`, and the harness binary's sha256;
- the engine commit the harness reported;
- the driver's exit code and its `examined` line;
- the sha256 of every golden written and of `stock-lines.sha256`, and the listing of the output folder showing no
  other file;
- the dirty side's read list: each engine file the harness's author and the run's operator read.
