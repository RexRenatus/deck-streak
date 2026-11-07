import Formal.PathGate

/-!
# Formal.PathGateVectors

The path gate's half of the vector writer (SPEC-634). `Formal/Vectors.lean` dispatches
`lean --run Formal/Vectors.lean PathGate` to `run`, which prints one header line naming the first
covered item (`prefix_of`) and the digest the entry recorded for it, then one line per input.

A `decide` line holds the event's three strings, the job, the change list (`null` when it is
unreadable), the map (`null` when the reader refused it), the refused map's text when the vectors
hold one, the port's `reads` answer for each changed path (`null` for an unreadable list), the
decision, and the admission of that job skipped while the path-gate job succeeded with the
decision as its output. An `admit` line holds the results in order, the path-gate job's success,
its outputs, and the admission: the refused gated jobs and every other job with its result.

The formal checker byte-compares the output with `formal/vectors/path-gate.jsonl`, and
`--write-vectors` writes it; the file is never edited by hand. The script's test reads every
vector through `scripts/path_gate.py`, and feeds the map text to the script's reader, which must
refuse it.

The inputs: each event on a change no job reads; change lists empty and unreadable for each job; a
map the reader refused, unparsed or with a key held twice, for each job; paths inside, outside and
mixed, through `always`, through `*` and for a job with no entry; renames as their two paths;
paths at a boundary (`tools` beside `toolsx/`); maps with no reads, with `*` in `always` and the
witness's own map; and the admissions. Every text the port reads, and every text a vector line
carries, is written as characters. This module carries no header line, so the registry lists it as
support.
-/

namespace Formal.PathGateVectors

open Formal.PathGate

/-- Text with each `"` and `\` escaped, the only characters the writer's texts carry that JSON
escapes. -/
def escape : List Char → List Char
  | [] => []
  | c :: cs =>
    if c == '\"' then '\\' :: '\"' :: escape cs
    else if c == '\\' then '\\' :: '\\' :: escape cs
    else c :: escape cs

/-- A JSON string of text read as characters. -/
def textJson (s : List Char) : List Char :=
  '\"' :: escape s ++ ['\"']

/-- A JSON string, for the header. -/
def jsonString (s : String) : String :=
  String.ofList (textJson s.toList)

/-- The first `covers` line of a module: its path, item and recorded digest. -/
def coverOf (module : String) : Option (String × String × String) :=
  (module.splitOn "\n").findSome? fun line =>
    match (line.splitOn " ").filter (· ≠ "") with
    | [_, _, "covers", path, anchor, digest] =>
      if anchor.startsWith "anchor=" && digest.startsWith "digest=" then
        some (path, (anchor.drop 7).toString, (digest.drop 7).toString)
      else none
    | _ => none

/-- The header line: what the vectors were written from, and how many follow. -/
def headerLine (entry path anchor digest : String) (count : Nat) : String :=
  "{\"schema\":\"phx.formal.vectors.v1\",\"entry\":" ++ jsonString entry ++
    ",\"covers\":" ++ jsonString path ++ ",\"anchor\":" ++ jsonString anchor ++
    ",\"digest\":" ++ jsonString digest ++ ",\"vectors\":" ++ toString count ++ "}"

/-- Each item after the first, with a comma before it. -/
def commaEach : List (List Char) → List Char
  | [] => []
  | y :: ys => ',' :: y ++ commaEach ys

/-- The items joined by commas. -/
def joinComma : List (List Char) → List Char
  | [] => []
  | x :: xs => x ++ commaEach xs

/-- A JSON array of rendered items. -/
def arrayJson (items : List (List Char)) : List Char :=
  '[' :: joinComma items ++ [']']

/-- A JSON object of rendered members. -/
def objectJson (members : List (List Char)) : List Char :=
  '{' :: joinComma members ++ ['}']

/-- One `"key":value` member. -/
def member (key value : List Char) : List Char :=
  textJson key ++ ':' :: value

/-- A JSON boolean. -/
def boolJson (b : Bool) : List Char :=
  if b then "true".toList else "false".toList

/-- JSON's null. -/
def nullJson : List Char :=
  "null".toList

/-- A JSON array of strings. -/
def textsJson (ts : List (List Char)) : List Char :=
  arrayJson (ts.map textJson)

/-- Each pair as a JSON array of its two strings. -/
def pairsJson (ps : List (List Char × List Char)) : List Char :=
  arrayJson (ps.map fun (a, b) => arrayJson [textJson a, textJson b])

/-- The path-gate job's outputs as a JSON object from each job to its word. -/
def outJson (ps : List (List Char × List Char)) : List Char :=
  objectJson (ps.map fun (a, b) => member a (textJson b))

/-- The event's three strings as a JSON array. -/
def eventJson (ev : Event) : List Char :=
  textsJson [ev.1, ev.2.1, ev.2.2]

/-- The change list, `null` when it is unreadable. -/
def changedJson : Option (List (List Char)) → List Char
  | none => nullJson
  | some l => textsJson l

/-- The map in the schema's form, `null` when the reader refused it. -/
def mapJson : Option ReadMap → List Char
  | none => nullJson
  | some mp =>
    objectJson
      [member "schema".toList (textJson "read-map.v1".toList),
        member "always".toList (textsJson mp.always),
        member "jobs".toList
          (objectJson (mp.jobs.map fun (job, r) =>
            member job (objectJson [member "reads".toList (textsJson r)])))]

/-- The refused map's text, `null` when the vectors hold none. -/
def mapTextJson : Option (List Char) → List Char
  | none => nullJson
  | some t => textJson t

/-- Each decision by the script's word. -/
def decisionWord : Decision → List Char
  | Decision.runJob => "run".toList
  | Decision.skipJob => "skip".toList

/-- The port's `reads` answer for each changed path, `null` for an unreadable list. -/
def readsJson (m : Option ReadMap) (job : List Char) : Option (List (List Char)) → List Char
  | none => nullJson
  | some l => arrayJson (l.map fun q => boolJson (reads m job q))

/-- An admission: the refused gated jobs, and every other job with its result. -/
def admissionJson (a : List (List Char) × List (List Char × List Char)) : List Char :=
  objectJson [member "refused".toList (textsJson a.1), member "rest".toList (pairsJson a.2)]

/-- One input to the decision. -/
structure DecideCase where
  name : List Char
  ev : Event
  job : List Char
  changed : Option (List (List Char))
  readMap : Option ReadMap
  mapText : Option (List Char)

/-- One input to the admission. -/
structure AdmitCase where
  name : List Char
  needs : List (List Char × List Char)
  gateOk : Bool
  gateOut : List (List Char × List Char)

/-- The map most cases read: `always` holds an exact path and a prefix; `rust` reads a prefix and
an exact path, `engine` a prefix, `release` the exact path `tools` and `census` every path. -/
def fixture : ReadMap :=
  { always := ["Cargo.toml".toList, "crates/".toList],
    jobs :=
      [("rust".toList, ["tools/".toList, "docs/rust.md".toList]),
        ("engine".toList, ["web/engine/".toList]),
        ("release".toList, ["tools".toList]),
        ("census".toList, ["*".toList])] }

/-- A map with no `always` entry and one job with no reads. -/
def bareMap : ReadMap :=
  { always := [], jobs := [("rust".toList, [])] }

/-- A map whose `always` holds `*`. -/
def starMap : ReadMap :=
  { always := ["*".toList], jobs := [("rust".toList, ["crates/".toList])] }

/-- The map of the decision's witness. -/
def witnessMap : ReadMap :=
  { always := [], jobs := [("rust".toList, ["crates/".toList])] }

/-- A map text that names the job `rust` twice, which the reader refuses. -/
def keyTwiceText : List Char :=
  ("{\"schema\":\"read-map.v1\",\"always\":[\"Cargo.toml\",\"crates/\"]," ++
    "\"jobs\":{\"rust\":{\"reads\":[\"tools/\"]},\"rust\":{\"reads\":[\"*\"]}}}").toList

/-- A pull request into dev on the fixture map. -/
def onFixture (name job : List Char) (changed : List (List Char)) : DecideCase :=
  { name := name, ev := prDev, job := job, changed := some changed, readMap := some fixture,
    mapText := none }

/-- An event for `rust` on a change it does not read. -/
def atEvent (name : List Char) (ev : Event) : DecideCase :=
  { onFixture name "rust".toList ["docs/a.md".toList] with ev := ev }

/-- A pull request into dev whose change list is unreadable. -/
def unreadable (name job : List Char) : DecideCase :=
  { onFixture name job [] with changed := none }

/-- A pull request into dev on a map the reader refused, with its text when the vectors hold it. -/
def refusedMap (name job : List Char) (text : Option (List Char)) : DecideCase :=
  { onFixture name job ["docs/a.md".toList] with readMap := none, mapText := text }

/-- A pull request into dev on another map. -/
def onMap (name job : List Char) (changed : List (List Char)) (mp : ReadMap) : DecideCase :=
  { onFixture name job changed with readMap := some mp }

/-- Every input to the decision: events 9, change lists 10, refused maps 10, reads 16, renames 3,
boundaries 8 and other maps 5. -/
def decideCases : List DecideCase :=
  [ atEvent "event-pr-into-dev".toList prDev,
    atEvent "event-push-into-dev".toList ("push".toList, "dev".toList, "false".toList),
    atEvent "event-push-without-base".toList ("push".toList, "".toList, "false".toList),
    atEvent "event-pr-into-main".toList ("pull_request".toList, "main".toList, "false".toList),
    atEvent "event-pr-release-run".toList ("pull_request".toList, "dev".toList, "true".toList),
    atEvent "event-pr-release-empty".toList ("pull_request".toList, "dev".toList, "".toList),
    atEvent "event-pr-target-into-dev".toList
      ("pull_request_target".toList, "dev".toList, "false".toList),
    atEvent "event-merge-group".toList ("merge_group".toList, "dev".toList, "false".toList),
    atEvent "event-dispatch".toList ("workflow_dispatch".toList, "".toList, "".toList),
    unreadable "changed-none-rust".toList "rust".toList,
    unreadable "changed-none-engine".toList "engine".toList,
    unreadable "changed-none-release".toList "release".toList,
    unreadable "changed-none-census".toList "census".toList,
    unreadable "changed-none-web".toList "web".toList,
    onFixture "changed-empty-rust".toList "rust".toList [],
    onFixture "changed-empty-engine".toList "engine".toList [],
    onFixture "changed-empty-release".toList "release".toList [],
    onFixture "changed-empty-census".toList "census".toList [],
    onFixture "changed-empty-web".toList "web".toList [],
    refusedMap "map-unparsed-rust".toList "rust".toList none,
    refusedMap "map-unparsed-engine".toList "engine".toList none,
    refusedMap "map-unparsed-release".toList "release".toList none,
    refusedMap "map-unparsed-census".toList "census".toList none,
    refusedMap "map-unparsed-web".toList "web".toList none,
    refusedMap "map-key-twice-rust".toList "rust".toList (some keyTwiceText),
    refusedMap "map-key-twice-engine".toList "engine".toList (some keyTwiceText),
    refusedMap "map-key-twice-release".toList "release".toList (some keyTwiceText),
    refusedMap "map-key-twice-census".toList "census".toList (some keyTwiceText),
    refusedMap "map-key-twice-web".toList "web".toList (some keyTwiceText),
    onFixture "rust-inside-prefix".toList "rust".toList ["tools/build.sh".toList],
    onFixture "rust-inside-exact".toList "rust".toList ["docs/rust.md".toList],
    onFixture "rust-outside".toList "rust".toList ["docs/a.md".toList],
    onFixture "rust-mixed".toList "rust".toList ["docs/a.md".toList, "tools/x.py".toList],
    onFixture "rust-outside-many".toList "rust".toList
      ["docs/a.md".toList, "web/app/x.ts".toList, "README.md".toList],
    onFixture "rust-always-exact".toList "rust".toList ["Cargo.toml".toList],
    onFixture "rust-always-prefix".toList "rust".toList ["crates/a/src/lib.rs".toList],
    onFixture "engine-always-exact".toList "engine".toList ["Cargo.toml".toList],
    onFixture "engine-inside".toList "engine".toList ["web/engine/a.ts".toList],
    onFixture "engine-outside".toList "engine".toList ["web/app/a.ts".toList],
    onFixture "release-exact".toList "release".toList ["tools".toList],
    onFixture "release-outside".toList "release".toList ["docs/a.md".toList],
    onFixture "census-star".toList "census".toList ["docs/a.md".toList],
    onFixture "census-star-mixed".toList "census".toList
      ["docs/a.md".toList, "web/app/a.ts".toList],
    onFixture "unknown-job-outside".toList "web".toList ["docs/a.md".toList],
    onFixture "unknown-job-boundary".toList "web".toList ["toolsx/a".toList],
    onFixture "rename-into-read".toList "rust".toList ["docs/old.md".toList, "tools/new.sh".toList],
    onFixture "rename-out-of-read".toList "rust".toList
      ["tools/old.sh".toList, "docs/new.md".toList],
    onFixture "rename-outside".toList "rust".toList ["docs/old.md".toList, "docs/new.md".toList],
    onFixture "release-exact-beside-toolsx".toList "release".toList ["toolsx/a".toList],
    onFixture "release-exact-beside-subpath".toList "release".toList ["tools/a".toList],
    onFixture "rust-exact-beside-longer-name".toList "rust".toList ["docs/rust.mdx".toList],
    onFixture "rust-prefix-beside-toolsx".toList "rust".toList ["toolsx/a".toList],
    onFixture "rust-prefix-bare-name".toList "rust".toList ["tools".toList],
    onFixture "rust-prefix-itself".toList "rust".toList ["tools/".toList],
    onFixture "engine-always-prefix-beside-cratesx".toList "engine".toList ["cratesx/a".toList],
    onFixture "engine-always-prefix-bare-name".toList "engine".toList ["crates".toList],
    onMap "bare-map-no-entry".toList "engine".toList ["docs/a.md".toList] bareMap,
    onMap "bare-map-no-reads".toList "rust".toList ["docs/a.md".toList] bareMap,
    onMap "star-always".toList "rust".toList ["docs/a.md".toList] starMap,
    onMap "witness-map-outside".toList "rust".toList ["docs/a.md".toList] witnessMap,
    onMap "witness-map-inside".toList "rust".toList ["crates/a.rs".toList] witnessMap ]

/-- Every input to the admission. -/
def admitCases : List AdmitCase :=
  [ ⟨"admit-empty".toList, [], true, []⟩,
    ⟨"admit-success".toList, [("rust".toList, "success".toList)], true, []⟩,
    ⟨"admit-success-gate-failed".toList, [("rust".toList, "success".toList)], false, []⟩,
    ⟨"admit-gate-skip".toList, [("rust".toList, "skipped".toList)], true,
      [("rust".toList, "skip".toList)]⟩,
    ⟨"admit-gate-failed-skip".toList, [("rust".toList, "skipped".toList)], false,
      [("rust".toList, "skip".toList)]⟩,
    ⟨"admit-gate-said-run".toList, [("rust".toList, "skipped".toList)], true,
      [("rust".toList, "run".toList)]⟩,
    ⟨"admit-gate-silent".toList, [("rust".toList, "skipped".toList)], true, []⟩,
    ⟨"admit-gate-other-job".toList, [("rust".toList, "skipped".toList)], true,
      [("engine".toList, "skip".toList)]⟩,
    ⟨"admit-failure".toList, [("rust".toList, "failure".toList)], true,
      [("rust".toList, "skip".toList)]⟩,
    ⟨"admit-cancelled".toList, [("engine".toList, "cancelled".toList)], true,
      [("engine".toList, "skip".toList)]⟩,
    ⟨"admit-ungated-skipped".toList, [("web".toList, "skipped".toList)], false, []⟩,
    ⟨"admit-ungated-failure".toList, [("mutation-rust".toList, "failure".toList)], true, []⟩,
    ⟨"admit-mixed".toList,
      [("rust".toList, "skipped".toList), ("engine".toList, "success".toList),
        ("web".toList, "success".toList), ("release".toList, "skipped".toList),
        ("census".toList, "failure".toList)], true,
      [("rust".toList, "skip".toList), ("release".toList, "run".toList)]⟩,
    ⟨"admit-mixed-gate-failed".toList,
      [("rust".toList, "skipped".toList), ("engine".toList, "success".toList),
        ("web".toList, "success".toList), ("release".toList, "skipped".toList)], false,
      [("rust".toList, "skip".toList), ("release".toList, "skip".toList)]⟩,
    ⟨"admit-every-gated-skip".toList,
      [("rust".toList, "skipped".toList), ("engine".toList, "skipped".toList),
        ("release".toList, "skipped".toList), ("census".toList, "skipped".toList)], true,
      [("rust".toList, "skip".toList), ("engine".toList, "skip".toList),
        ("release".toList, "skip".toList), ("census".toList, "skip".toList)]⟩,
    ⟨"admit-order".toList,
      [("census".toList, "failure".toList), ("web".toList, "failure".toList),
        ("rust".toList, "failure".toList), ("card-sandbox".toList, "success".toList)], true, []⟩,
    ⟨"admit-skip-word-case".toList, [("rust".toList, "skipped".toList)], true,
      [("rust".toList, "Skip".toList)]⟩ ]

/-- One decision line: the input, the port's `reads` answers, the decision and the admission of
the job skipped on it. -/
def decideLine (c : DecideCase) : List Char :=
  let d := decideGate c.ev c.job c.changed c.readMap
  objectJson
    [member "kind".toList (textJson "decide".toList),
      member "case".toList (textJson c.name),
      member "event".toList (eventJson c.ev),
      member "job".toList (textJson c.job),
      member "changed".toList (changedJson c.changed),
      member "map".toList (mapJson c.readMap),
      member "map_text".toList (mapTextJson c.mapText),
      member "reads".toList (readsJson c.readMap c.job c.changed),
      member "decision".toList (textJson (decisionWord d)),
      member "admit".toList
        (admissionJson (admitGate [(c.job, "skipped".toList)] true [(c.job, decisionWord d)]))]

/-- One admission line: the input and the admission. -/
def admitLine (c : AdmitCase) : List Char :=
  let a := admitGate c.needs c.gateOk c.gateOut
  objectJson
    [member "kind".toList (textJson "admit".toList),
      member "case".toList (textJson c.name),
      member "needs".toList (pairsJson c.needs),
      member "gate_ok".toList (boolJson c.gateOk),
      member "gate_out".toList (outJson c.gateOut),
      member "refused".toList (textsJson a.1),
      member "rest".toList (pairsJson a.2)]

/-- Print the header and every vector: each decision line, then each admission line. -/
def run : IO UInt32 := do
  let module ← IO.FS.readFile "Formal/PathGate.lean"
  match coverOf module with
  | none =>
    IO.eprintln "Formal/PathGate.lean names no covered item"
    return 1
  | some (path, anchor, digest) =>
    IO.println (headerLine "PathGate" path anchor digest
      (decideCases.length + admitCases.length))
    for c in decideCases do
      IO.println (String.ofList (decideLine c))
    for c in admitCases do
      IO.println (String.ofList (admitLine c))
    return 0

end Formal.PathGateVectors
