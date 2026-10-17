# Red-first record: SPEC-475

This delivery adds a model and no test, and changes no production line. The model pins behaviour
the base already has: the test log capture's floor is installed as the global default before every
capture, hints `OFF` and answers every callsite "sometimes", and the census refuses every name that
rebuilds the interest cache or registers a callsite outside the helper, so each of the three
properties already holds at the base. No criterion can be red first against the code. Each
criterion's red is its witness instead, a configuration of the same model with one switch turned
off, which the formal checker reports as a violation of the property; the property is then read
clean in `MCLogCaptureFloor.cfg`, which models the code as it is.

```red-first
A1: not red: pins the floor the base already has; its red is the witness witness/no-cached-never-while-a-capture-is-live.cfg, which the checker reports as a violation
A2: not red: pins the floor's level hint the base already has; its red is the witness witness/a-line-under-a-capture-reaches-it.cfg, which the checker reports as a violation
A3: not red: pins the census's refusals the base already has; its red is the witness witness/no-interest-is-asked-before-the-floor.cfg, which the checker reports as a violation
```

## The readings at a3a53d17

The entry was first committed with every cover's digest written as zeros (11bca194), checked, then
stamped from the checker's `STALE` findings and committed again (a3a53d17), where every reading
below was taken. The stamp changed only the six cover lines and the configuration's `states` line:
no action, property, switch or configuration constant moved between the two commits.

### Each witness's violation, read before its property's clean line

The checker's card records a caught witness as the absence of a surviving one and prints no line
of its own for it. Its run of the entry at 11bca194, and again at a3a53d17, reported no surviving
witness. TLC's own violation line for each witness was read by running the witness configuration
by hand over the module at a3a53d17, and each trace was walked step by step:

- `witness/no-cached-never-while-a-capture-is-live.cfg` (`Floor = FALSE`): `Error: Invariant
  NoCachedNeverWhileACaptureIsLive is violated.`, after 149 distinct states, over a trace of seven
  states (`StartCapture`, `RegisterCapture`, `Emit("a")`, `Choose("a")`, `Ask("a")`, `Store("a")`)
  whose dump is sha256 ab51ebcfb1bf1d134c19c3a2943fe1301cb68def9587fd1bb0e0f4677b6a1777. The
  capture registers alone, so `has_just_one` stays true; the thread with no capture then asks its
  own default with no lock, finds no global default and answers "never", and stores it while the
  capture is live.
- `witness/a-line-under-a-capture-reaches-it.cfg` (`FloorHintsOff = FALSE`): `Error: Invariant
  ALineUnderACaptureReachesIt is violated.`, after 692 distinct states, over a trace of eleven
  states (`StartCapture`, `InstallFloor`, `Emit("a")`, `Choose("a")`, `Ask("a")`, `SetGlobal`,
  `RegisterCapture`, `Store("a")`, `Swap`, `Emit("capturer")`) whose dump is sha256
  78ac2534803c95c0d7ade6b3f2532c7d5e9fb36f47ffec1177bf28fafefb3c43. The floor's hint opens the
  level at its registration, the ask falls between `InstallFloor` and `SetGlobal` and answers
  "never", and that answer is stored after the capture's registration had rebuilt the callsite to
  "sometimes", so the capturing thread's next line is dropped.
- `witness/no-interest-is-asked-before-the-floor.cfg` (`CensusHolds = FALSE`): `Error: Invariant
  NoInterestIsAskedBeforeTheFloor is violated.`, after 58 distinct states, over a trace of four
  states (`DirectRegister("a")`, `Choose("a")`, `Ask("a")`) whose dump is sha256
  404215020c3f52c135b674c19cff01ab8d4efb3cc9eb5dc1c2163f687e97cf0a. A registration that skips the
  level gate reaches the lock-free ask before the floor is the global default.

The same three traces, with equal dumps, were read over the module at 11bca194.

### Each property's clean line

The checker's run of the entry at a3a53d17 read `FORMAL OK exit=0` with no finding: no `STALE`,
each of the three properties `clean`, and each witness caught. Each property read alone, with its
own witness, was clean as well.

### Explored states

`MCLogCaptureFloor.cfg` explores 750 distinct states, to a depth of 17, with no deadlock and no
violation; its floor is `states >= 750`.

### The six stamped covers

Each `digest=` is the sha256 of its item's span in `tools/log-capture/capture.rs`, copied from the
checker's `STALE` finding and recomputed from the span:

- `register_floor`: 87bf7a9cb911a16288017df429512cca94839bfb7ec9405434792c652ceba323
- `with_capture`: 72895a5ee1ab794c82d60071ca4600494679c132dc0c08898dba3ccbdbf79a5f
- `hold_capture`: 36c30fe31d90ebd71c992a20b53b18995d8838e7a711d4caecda4aa9138faf76
- `refuse_nested_capture`: 4f62fdd7d993b0cb1bb19e52e402b4d8117cd138313943ff8ab3bce12f3db736
- `register_callsite`: 0ec0912b0e07367ee5b73c40436d6bc5b8b31f730c1117afc56fbc5a387f98a3
- `max_level_hint`: c55d1f2c8f091f67e464b0e94d58a622543a79705fe76b70b3854f1c8848f157

### The traps

A clean reading over actions no state reaches would prove nothing, so each situation the
properties guard was trapped in a scratch copy of the entry at a3a53d17: each trap is its own
invariant `~(<situation>)` beside the main configuration's constants, checked by hand. No trap is
committed.

- `~reached`: violated. A line on the capturing thread reaches its capture.
- `~(step["a"] = "load" /\ capLive)`: violated. The thread with no capture finishes the first
  registration while a capture is live.
- `~(step[Capturer] = "load")`: violated. The capturing thread registers the callsite first.
- `~(captures = 2 /\ capLive)`: violated. A second capture registers after the first ended.
- `~(interest = "none" /\ reg = "registering" /\ Holds(Capturer) /\ step[Capturer] = "none")`:
  violated. The capturing thread can emit while the other thread's registration is in flight.
- `\A t \in Threads : step[t] # "ask"`: clean in the main configuration, over the same 750
  distinct states, and violated with `FloorHintsOff = FALSE`. The lock-free ask is not reached
  while the floor hints `OFF`, which is ADR-492 D1 item 8's finding, and it is reached once the
  hint admits a level, so the clean reading of the main configuration is the code's and not a
  vacuous model's.

With one more actor, `Bystanders = {"a", "b"}`, the main configuration's three properties read
clean over 3242 distinct states, to a depth of 19.
