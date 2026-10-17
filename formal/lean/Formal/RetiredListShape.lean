-- @phx covers scripts/mutation_rows.py anchor=retirement_records digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
-- @phx vectors formal/vectors/retired-list-shape.jsonl
-- @phx cites #421
-- @phx theorem every_retired_list_is_refused_by_name_or_read_as_before ramp=report
-- @phx witness old_violates kills=every_retired_list_is_refused_by_name_or_read_as_before

/-!
# Formal.RetiredListShape

The retirement check's reading of `scripts/mutation-rows.retired.json`, as `retirement_records`
reads the document `parse_document` returns (#421, SPEC-465 R1 to R5, ADR-482). The parser has
already refused a repeated key and a text that is not JSON, so the document here is any JSON value
with each object's keys distinct.

**The claim**, one `Prop` over a reader, `Claim`, over every document:
- the reader never crashes;
- it refuses exactly the documents that are not well-formed (R1 to R4), each with the whole
  sentence of the first violation, an entry named by its index from 0;
- on a well-formed document it answers the ids the base's reading answers.

**What is ported.** `retirement_records` branch for branch: R1's object test, then `retired` read
with an absent key as no entry, then R2's array test, then each entry in order, R3's object test
before R4's string-id test, and the record filter that keeps an entry whose reason and approval
are both non-blank, by `record[entry["id"]] = entry`. Python's `str(value).strip()` is non-blank
for a string holding a character `str.strip()` keeps, and for every value that is not a string
(`1`, `True`, `None`, `[]` and `{}` each print a word); a missing key reads its default, `""`,
which is blank. The record is the list of the kept ids in their order; a row's id is a string, so
a key that is not one can be held by no row.

**What is not ported.** The parser itself, and what `retired` does with the record: the leaving
rows of the base and the approval each prints.

The witness is dev's reading at `36c4783a`: `scripts/mutation_rows.py:1394-1396` iterated
`listed.get("retired", [])`, called `entry.get` on each entry and subscripted `entry["id"]`, with
no test of either shape, so a top-level array crashed `retired` with an `AttributeError` at `.get`
and exit 1 (#421).
-/

namespace Formal.RetiredListShape

/-- A parsed JSON document: an object as its keys and values in order, an array, a string, a
number (its value is never read here), a boolean or null. -/
inductive Json where
  | obj (fields : List (List Char × Json))
  | arr (items : List Json)
  | str (text : List Char)
  | num (value : Int)
  | bool (value : Bool)
  | null

/-- What the reading answers: a refusal with its sentence, the ids it keeps, or a crash. -/
inductive Answer where
  | refused (sentence : List Char)
  | ids (kept : List (List Char))
  | crashed
  deriving DecidableEq, Repr

/-- Two texts are equal, character by character, as `==` compares two `str`. -/
def same : List Char → List Char → Bool
  | [], [] => true
  | a :: as, b :: bs => a == b && same as bs
  | _, _ => false

/-- `obj.get(key)`: the value under `key`, or none. The parser refused a repeated key, so the
first one is the only one. -/
def lookup (key : List Char) : List (List Char × Json) → Option Json
  | [] => none
  | (k, v) :: rest => if same k key then some v else lookup key rest

/-- The code points `str.strip()` removes, as Python reads `str.isspace`. -/
def spaces : List Nat :=
  [9, 10, 11, 12, 13, 28, 29, 30, 31, 32, 133, 160, 5760, 8192, 8193, 8194, 8195, 8196, 8197,
    8198, 8199, 8200, 8201, 8202, 8232, 8233, 8239, 8287, 12288]

/-- One character `str.strip()` removes. -/
def isSpace (c : Char) : Bool :=
  spaces.contains c.toNat

/-- `str(entry.get(key, "")).strip()` is non-blank: a string with a character the strip keeps,
any other value, and never a missing key, whose default is `""`. -/
def nonBlank : Option Json → Bool
  | none => false
  | some (.str text) => text.any fun c => !isSpace c
  | some _ => true

/-- The key `listed.get("retired", [])` reads. -/
def retiredKey : List Char :=
  "retired".toList

/-- The key of an entry's row id. -/
def idKey : List Char :=
  "id".toList

/-- The key of an entry's reason. -/
def reasonKey : List Char :=
  "reason".toList

/-- The key of an entry's approval. -/
def approvalKey : List Char :=
  "approval".toList

/-- The list's path, as `RETIRED` spells it. -/
def listPath : List Char :=
  "scripts/mutation-rows.retired.json".toList

/-- R1's sentence. -/
def notAnObject : List Char :=
  listPath ++ " must hold a top-level object".toList

/-- R2's sentence. -/
def notAnArray : List Char :=
  listPath ++ ": `retired` is not an array of records".toList

/-- R3's sentence for the entry at index `i`. -/
def entryNotAnObject (i : Nat) : List Char :=
  listPath ++ ": retired[".toList ++ Nat.toDigits 10 i ++ "] is not an object".toList

/-- R4's sentence for the entry at index `i`. -/
def entryWithoutAnId (i : Nat) : List Char :=
  listPath ++ ": retired[".toList ++ Nat.toDigits 10 i ++ "] has no string id".toList

/-- `record[entry["id"]] = entry`: a missing key is a `KeyError` and a list or an object is
unhashable, a `TypeError`, each a crash (none); a string id is kept; any other value is a key no
row's id equals. -/
def keep (kept : List (List Char)) (fields : List (List Char × Json)) :
    Option (List (List Char)) :=
  match lookup idKey fields with
  | some (.str id) => some (kept ++ [id])
  | some (.num _) => some kept
  | some (.bool _) => some kept
  | some .null => some kept
  | some (.arr _) => none
  | some (.obj _) => none
  | none => none

/-- `retirement_records`' loop over the entries from index `i`, `kept` the ids kept so far: R3,
then R4, then the record filter, the first wrong entry refused by its index. -/
def recordLoop : Nat → List (List Char) → List Json → Answer
  | _, kept, [] => .ids kept
  | i, kept, entry :: rest =>
    match entry with
    | .obj fields =>
      match lookup idKey fields with
      | some (.str _) =>
        if nonBlank (lookup reasonKey fields) && nonBlank (lookup approvalKey fields)
        then
          match keep kept fields with
          | some more => recordLoop (i + 1) more rest
          | none => .crashed
        else recordLoop (i + 1) kept rest
      | _ => .refused (entryWithoutAnId i)
    | _ => .refused (entryNotAnObject i)

/-- `retirement_records`, the reading that ships: R1, `retired` with an absent key read as no
entry, R2, then the loop from index 0. -/
def retirementRecords : Json → Answer
  | .obj fields =>
    match (lookup retiredKey fields).getD (.arr []) with
    | .arr entries => recordLoop 0 [] entries
    | _ => .refused notAnArray
  | _ => .refused notAnObject

/-- `for entry in value`: an array's items, a string's characters and an object's keys, each a
string; a number, a boolean and null are not iterable, a `TypeError` (none). -/
def iterate : Json → Option (List Json)
  | .arr items => some items
  | .str text => some (text.map fun c => .str [c])
  | .obj fields => some (fields.map fun field => .str field.1)
  | _ => none

/-- dev's loop at the base: no shape test, `entry.get` of a value that is not an object an
`AttributeError`, and the record filter before the subscript. -/
def oldLoop : List (List Char) → List Json → Answer
  | kept, [] => .ids kept
  | kept, entry :: rest =>
    match entry with
    | .obj fields =>
      if nonBlank (lookup reasonKey fields) && nonBlank (lookup approvalKey fields)
      then
        match keep kept fields with
        | some more => oldLoop more rest
        | none => .crashed
      else oldLoop kept rest
    | _ => .crashed

/-- dev's reading at the base: `listed.get("retired", [])`, a crash for a document that is not an
object, then the iteration and the loop. -/
def old : Json → Answer
  | .obj fields =>
    match iterate ((lookup retiredKey fields).getD (.arr [])) with
    | some entries => oldLoop [] entries
    | none => .crashed
  | _ => .crashed

/-- R3 and R4 for the entry at index `i`, as the SPEC states them. -/
def entryFault (i : Nat) : Json → Option (List Char)
  | .obj fields =>
    match lookup idKey fields with
    | some (.str _) => none
    | _ => some (entryWithoutAnId i)
  | _ => some (entryNotAnObject i)

/-- The first entry from index `i` that R3 or R4 refuses. -/
def firstFault : Nat → List Json → Option (List Char)
  | _, [] => none
  | i, entry :: rest =>
    match entryFault i entry with
    | some sentence => some sentence
    | none => firstFault (i + 1) rest

/-- R1 to R4: the sentence of a document's first violation, or none when it is well-formed. -/
def violation : Json → Option (List Char)
  | .obj fields =>
    match lookup retiredKey fields with
    | none => none
    | some (.arr entries) => firstFault 0 entries
    | some _ => some notAnArray
  | _ => some notAnObject

/-- The claim over a reader: it never crashes, it refuses exactly the documents R1 to R4 refuse,
each with the sentence of the first violation, and a well-formed document reads as the base reads
it. -/
def Claim (reader : Json → Answer) : Prop :=
  ∀ document : Json,
    reader document ≠ .crashed ∧
      (∀ sentence, reader document = .refused sentence ↔ violation document = some sentence) ∧
      (violation document = none → reader document = old document)

theorem recordLoop_agrees :
    ∀ (entries : List Json) (i : Nat) (kept : List (List Char)),
      recordLoop i kept entries ≠ .crashed ∧
        (∀ sentence,
          recordLoop i kept entries = .refused sentence ↔ firstFault i entries = some sentence) ∧
        (firstFault i entries = none → recordLoop i kept entries = oldLoop kept entries)
  | [], _, _ => by simp [recordLoop, firstFault, oldLoop]
  | entry :: rest, i, kept => by
    cases entry with
    | obj fields =>
      cases hid : lookup idKey fields with
      | none => simp [recordLoop, firstFault, entryFault, hid]
      | some value =>
        cases value with
        | str id =>
          have hkeep : keep kept fields = some (kept ++ [id]) := by simp [keep, hid]
          by_cases hb :
              (nonBlank (lookup reasonKey fields) &&
                nonBlank (lookup approvalKey fields)) = true
          · have ih := recordLoop_agrees rest (i + 1) (kept ++ [id])
            simp only [recordLoop, firstFault, entryFault, oldLoop, hid, hb, hkeep, ite_true]
            exact ih
          · have ih := recordLoop_agrees rest (i + 1) kept
            simp only [recordLoop, firstFault, entryFault, oldLoop, hid, hb, Bool.false_eq_true,
              ite_false]
            exact ih
        | arr items => simp [recordLoop, firstFault, entryFault, hid]
        | obj inner => simp [recordLoop, firstFault, entryFault, hid]
        | num n => simp [recordLoop, firstFault, entryFault, hid]
        | bool b => simp [recordLoop, firstFault, entryFault, hid]
        | null => simp [recordLoop, firstFault, entryFault, hid]
    | arr items => simp [recordLoop, firstFault, entryFault]
    | str text => simp [recordLoop, firstFault, entryFault]
    | num n => simp [recordLoop, firstFault, entryFault]
    | bool b => simp [recordLoop, firstFault, entryFault]
    | null => simp [recordLoop, firstFault, entryFault]

/-- The reading that ships refuses every wrongly shaped list by the name of its first violation,
never crashes, and reads every well-formed list as the base read it. -/
theorem every_retired_list_is_refused_by_name_or_read_as_before : Claim retirementRecords := by
  intro document
  cases document with
  | obj fields =>
    cases hr : lookup retiredKey fields with
    | none => simp [retirementRecords, violation, old, iterate, recordLoop, oldLoop, hr]
    | some value =>
      cases value with
      | arr entries =>
        have h := recordLoop_agrees entries 0 []
        simp only [retirementRecords, violation, old, iterate, hr, Option.getD_some]
        exact h
      | str text => simp [retirementRecords, violation, hr]
      | obj inner => simp [retirementRecords, violation, hr]
      | num n => simp [retirementRecords, violation, hr]
      | bool b => simp [retirementRecords, violation, hr]
      | null => simp [retirementRecords, violation, hr]
  | arr items => simp [retirementRecords, violation]
  | str text => simp [retirementRecords, violation]
  | num n => simp [retirementRecords, violation]
  | bool b => simp [retirementRecords, violation]
  | null => simp [retirementRecords, violation]

/-! ## The witness: dev's reading of a top-level array -/

/-- dev's reading crashes on a top-level array, so the claim is false of it. -/
theorem old_violates : ¬ Claim old := by
  intro h
  exact (h (.arr [])).1 rfl

end Formal.RetiredListShape
