-- @phx covers deploy/deploy.sh anchor=refuse_unlisted_settings digest=sha256:253c766344f7d7bfcb5a4a133b173090770847f84b7e46bcdb973fe364088a3d
-- @phx vectors formal/vectors/deploy-entry-refusal.jsonl
-- @phx cites #484, #428, #423
-- @phx theorem a_caddy_step_proceeds_exactly_on_listed_settings_each_given_once_with_a_value ramp=report
-- @phx witness the_shells_variables_alone_admit_an_entry_no_shell_can_spell kills=a_caddy_step_proceeds_exactly_on_listed_settings_each_given_once_with_a_value

/-!
# Formal.DeployEntryRefusal

Each Caddy step's settings check, the function `refuse_unlisted_settings` of `deploy/deploy.sh`
(#484, SPEC-473, ADR-490), ported branch for branch over bytes, and its claim proved over every
environment block the kernel can hold.

**What is ported.** The loop over the prefixed names of the shell's variables, `variablesLoop`;
the read of the received block as `mapfile -t -d ''` splits it at each NUL, `entries`, and the read
that fails, `none`; the word split of `SETTINGS`, `fields`, and the `!=` loop over its words,
`named`; and the loop over the entries, `entryLoop`: the cut at the first `=`, `cut`; the prefix
test, `startsWith pfx`; the listed test; the no-value test as the script writes it, the key
compared with its whole entry; and the record of the keys given, one space and then each key
followed by one space, with its `*" $key "*` test, `isInfixOf`. Each refusal's stderr line is
`Fault.line`, byte for byte with its newline.

**The claim**, `Claim`, over every list of the shell's variable names and every block laid out as
the kernel keeps it (each entry followed by its NUL, no entry holding NUL): the step proceeds
exactly when the block holds an entry, every entry whose key (its bytes before its first `=`)
begins with the prefix has a key that is one of the 13 listed names and holds `=`, no such key
occurs twice, and every prefixed variable name is a listed name; a read that fails never proceeds;
and two blocks whose entries pair off with the same key and the same presence of `=` get the same
verdict, so the check reads no value. The claim names the keys with library functions; every
bridge from the script's mechanisms to them is a lemma below.

**What is not ported.** Which entries bash makes variables of, and what a start-up file makes, is
the input `vars`; start-up code that runs before the function is outside the check and outside this
proof (#423). `SETTINGS` lies outside the covered function, so `settingsText` is its text as the
file holds it, and the vectors test ties it to the file.

**The witness** is the check at dev `dafbbbbb` (`deploy/deploy.sh:35-43` there), the
shell-variable loop alone with no read of the received block. SPEC-127's red-first record measured
that step running with the entry whose key is the prefix followed by the byte 0x01
(`docs/red-first/SPEC-127.md:265`): its port proceeds on that block with no shell variable, and the
claim refuses it.
-/

namespace Formal.DeployEntryRefusal

/-- Bytes, as the kernel keeps an environment entry and bash compares one. -/
abbrev Bytes := List UInt8

/-- The bytes of a text as UTF-8; every text this entry spells is ASCII, one byte a character. -/
def ascii (s : String) : Bytes := s.toUTF8.data.toList

/-- The bytes between `SETTINGS`' quotes (`deploy/deploy.sh:29-34` at dev). -/
def settingsText : Bytes :=
  ascii ("\nDECKSTREAK_DEPLOY_REPO DECKSTREAK_DEPLOY_HOST DECKSTREAK_DEPLOY_ELEVATE " ++
    "DECKSTREAK_DEPLOY_CHECKOUT\nDECKSTREAK_DEPLOY_ROOT DECKSTREAK_DEPLOY_UNIT_DIR " ++
    "DECKSTREAK_DEPLOY_ENV_FILE\nDECKSTREAK_DEPLOY_CADDY_DIR DECKSTREAK_DEPLOY_CADDYFILE " ++
    "DECKSTREAK_DEPLOY_CADDY_CONFIG\nDECKSTREAK_DEPLOY_READY_SECONDS DECKSTREAK_DEPLOY_READY_POLL " ++
    "DECKSTREAK_DEPLOY_KEEP\n")

/-- `for setting in $SETTINGS`, unquoted: bash splits the text into words at spaces, tabs and
newlines and drops the empty fields. `word` holds the current word's bytes, the latest first. -/
def fieldsFrom : Bytes → Bytes → List Bytes
  | [], word => if word = [] then [] else [word.reverse]
  | c :: rest, word =>
    if c = 32 ∨ c = 9 ∨ c = 10 then
      if word = [] then fieldsFrom rest [] else word.reverse :: fieldsFrom rest []
    else fieldsFrom rest (c :: word)

/-- The words of a text, as the unquoted expansion splits it. -/
def fields (text : Bytes) : List Bytes := fieldsFrom text []

/-- `named=; for setting in $SETTINGS; do [ "$x" != "$setting" ] || named=1; done`: `found` is
whether `named` is set, and each word is compared byte for byte. -/
def namedFrom (x : Bytes) : Bool → List Bytes → Bool
  | found, [] => found
  | found, setting :: rest => namedFrom x (if x ≠ setting then found else true) rest

/-- `[ -n "$named" ]` after the loop: whether some word equals the name. -/
def named (x : Bytes) (settings : List Bytes) : Bool := namedFrom x false settings

/-- The deploy settings' prefix. -/
def pfx : Bytes := ascii "DECKSTREAK_DEPLOY_"

/-- `case $key in DECKSTREAK_DEPLOY_*)`, and the prefix `${!DECKSTREAK_DEPLOY_@}` reads: the key is
the subject, so its own bytes are compared literally. -/
def startsWith : Bytes → Bytes → Bool
  | [], _ => true
  | _ :: _, [] => false
  | a :: p, b :: k => if a = b then startsWith p k else false

/-- `key=${entry%%=*}`: the longest suffix that begins with `=` removed, so the bytes before the
first `=`, or the whole entry when it holds none. -/
def cut : Bytes → Bytes
  | [] => []
  | c :: rest => if c = 61 then [] else c :: cut rest

/-- `mapfile -t -d ''`: one record at each NUL, the NUL removed; a last record without its NUL is
kept, and nothing follows a final NUL. `record` holds the current record, the latest byte first. -/
def entriesFrom : Bytes → Bytes → List Bytes
  | [], record => if record = [] then [] else [record.reverse]
  | c :: rest, record =>
    if c = 0 then record.reverse :: entriesFrom rest [] else entriesFrom rest (c :: record)

/-- The records `mapfile` reads from the received block. -/
def entries (area : Bytes) : List Bytes := entriesFrom area []

/-- `case $given in *" $key "*)`: the quoted middle matches literally, anywhere in the record. -/
def isInfixOf (needle : Bytes) : Bytes → Bool
  | [] => startsWith needle []
  | c :: rest => startsWith needle (c :: rest) || isInfixOf needle rest

/-- Why the check refuses. -/
inductive Fault where
  | notASetting (name : Bytes)
  | unread
  | noValue (key : Bytes)
  | twice (key : Bytes)
  deriving DecidableEq, Repr

/-- Each refusal's stderr line, as the function echoes it. -/
def Fault.line : Fault → Bytes
  | .notASetting name => ascii "deploy: " ++ name ++ ascii " is not a setting of deploy.sh\n"
  | .unread => ascii "deploy: the environment this step received could not be read\n"
  | .noValue key => ascii "deploy: " ++ key ++ ascii " is given without a value\n"
  | .twice key => ascii "deploy: " ++ key ++ ascii " is given twice\n"

/-- The check's verdict: the step proceeds past it, or it exits 1 with the fault's line. -/
inductive Verdict where
  | proceed
  | refuse (fault : Fault)
  deriving DecidableEq, Repr

/-- The loop over `${!DECKSTREAK_DEPLOY_@}` (`deploy/deploy.sh:37-41` at dev): each name of the
shell's variables that begins with the prefix, in the order given, and a name no word equals is
not a setting. -/
def variablesLoop : List Bytes → Option Fault
  | [] => none
  | name :: rest =>
    if startsWith pfx name then
      if named name (fields settingsText) then variablesLoop rest else some (.notASetting name)
    else variablesLoop rest

/-- The loop over the received entries (`deploy/deploy.sh:46-56` at dev). `given` is the record of
the keys given, as bytes: it starts as one space, and each key joins it followed by one space. The
listed test comes first, then the no-value test, then the given-twice test. -/
def entryLoop (listed : List Bytes) : Bytes → List Bytes → Option Fault
  | _, [] => none
  | given, entry :: rest =>
    if startsWith pfx (cut entry) then
      if named (cut entry) listed then
        if cut entry ≠ entry then
          if isInfixOf ([32] ++ cut entry ++ [32]) given then some (.twice (cut entry))
          else entryLoop listed (given ++ cut entry ++ [32]) rest
        else some (.noValue (cut entry))
      else some (.notASetting (cut entry))
    else entryLoop listed given rest

/-- `refuse_unlisted_settings`: the shell-variable loop, then the read (`none` when it fails), then
the entry loop; a read that yields no record is refused as unread. -/
def refuseUnlistedSettings (vars : List Bytes) (read : Option Bytes) : Verdict :=
  match variablesLoop vars with
  | some fault => .refuse fault
  | none =>
    match read with
    | none => .refuse .unread
    | some area =>
      if entries area = [] then .refuse .unread
      else
        match entryLoop (fields settingsText) [32] (entries area) with
        | some fault => .refuse fault
        | none => .proceed

/-- The check at dev `dafbbbbb`: the shell-variable loop alone, with no read of the received block. -/
def variablesAlone (vars : List Bytes) (_read : Option Bytes) : Verdict :=
  match variablesLoop vars with
  | some fault => .refuse fault
  | none => .proceed

/-! ## The claim -/

/-- The 13 listed settings, in `SETTINGS` order. -/
def listedSettings : List Bytes :=
  [ascii "DECKSTREAK_DEPLOY_REPO", ascii "DECKSTREAK_DEPLOY_HOST",
    ascii "DECKSTREAK_DEPLOY_ELEVATE", ascii "DECKSTREAK_DEPLOY_CHECKOUT",
    ascii "DECKSTREAK_DEPLOY_ROOT", ascii "DECKSTREAK_DEPLOY_UNIT_DIR",
    ascii "DECKSTREAK_DEPLOY_ENV_FILE", ascii "DECKSTREAK_DEPLOY_CADDY_DIR",
    ascii "DECKSTREAK_DEPLOY_CADDYFILE", ascii "DECKSTREAK_DEPLOY_CADDY_CONFIG",
    ascii "DECKSTREAK_DEPLOY_READY_SECONDS", ascii "DECKSTREAK_DEPLOY_READY_POLL",
    ascii "DECKSTREAK_DEPLOY_KEEP"]

/-- An entry's key: its bytes before its first `=`. -/
def keyOf (e : Bytes) : Bytes := e.takeWhile (· != 61)

/-- A block as the kernel lays it out: each entry followed by its NUL. -/
def layout (block : List Bytes) : Bytes := block.flatMap (· ++ [0])

/-- No entry of the block holds NUL. -/
def NoNul (block : List Bytes) : Prop := ∀ e ∈ block, (0 : UInt8) ∉ e

/-- The keys of a block's entries that begin with the prefix, in order. -/
def prefixedKeys (block : List Bytes) : List Bytes :=
  (block.map keyOf).filter fun k => decide (pfx <+: k)

/-- The issue's property: the block holds an entry; every prefixed key is listed and its entry holds
`=`; no prefixed key occurs twice; and every prefixed variable name is listed. -/
def Admissible (vars block : List Bytes) : Prop :=
  block ≠ [] ∧
  (∀ e ∈ block, pfx <+: keyOf e → keyOf e ∈ listedSettings ∧ (61 : UInt8) ∈ e) ∧
  (prefixedKeys block).Nodup ∧
  (∀ v ∈ vars, pfx <+: v → v ∈ listedSettings)

/-- Two blocks whose entries pair off, in order, with the same key and the same presence of `=`. -/
def SameKeys : List Bytes → List Bytes → Prop
  | [], [] => True
  | x :: a, y :: b => (keyOf x = keyOf y ∧ ((61 : UInt8) ∈ x ↔ (61 : UInt8) ∈ y)) ∧ SameKeys a b
  | _, _ => False

/-- The claim, over a step: it proceeds exactly on the admissible blocks, never on a read that fails,
and alike on two blocks with the same keys. -/
def Claim (step : List Bytes → Option Bytes → Verdict) : Prop :=
  (∀ vars block, NoNul block →
    (step vars (some (layout block)) = .proceed ↔ Admissible vars block)) ∧
  (∀ vars, step vars none ≠ .proceed) ∧
  (∀ vars a b, NoNul a → NoNul b → SameKeys a b →
    (step vars (some (layout a)) = .proceed ↔ step vars (some (layout b)) = .proceed))

/-! ## Bridges -/

/-- The word split of `SETTINGS` is the 13 listed names. -/
theorem fields_settingsText : fields settingsText = listedSettings := by decide +kernel

/-- The `!=` loop sets `named` exactly when it was set or some word equals the name. -/
theorem namedFrom_iff (x : Bytes) :
    ∀ (found : Bool) (settings : List Bytes),
      namedFrom x found settings = true ↔ found = true ∨ x ∈ settings
  | found, [] => by simp [namedFrom]
  | found, setting :: rest => by
    rw [namedFrom, namedFrom_iff x _ rest]
    by_cases h : x = setting
    · subst h; simp
    · simp [h]

/-- The `!=` loop is list membership. -/
theorem named_iff (x : Bytes) (settings : List Bytes) : named x settings = true ↔ x ∈ settings := by
  simp [named, namedFrom_iff]

/-- The prefix test is a list prefix. -/
theorem startsWith_iff : ∀ (p k : Bytes), startsWith p k = true ↔ p <+: k
  | [], k => by simp [startsWith]
  | a :: p, [] => by simp [startsWith]
  | a :: p, b :: k => by
    rw [startsWith, List.cons_prefix_cons, ← startsWith_iff p k]
    by_cases h : a = b <;> simp [h]

/-- The cut at the first `=` is the key. -/
theorem cut_eq_keyOf : ∀ e : Bytes, cut e = keyOf e
  | [] => rfl
  | c :: rest => by
    have ih := cut_eq_keyOf rest
    unfold keyOf at ih ⊢
    by_cases h : c = 61
    · simp [cut, h]
    · simp [cut, h, ih]

/-- The key equals its entry exactly when the entry holds no `=`. -/
theorem cut_eq_self_iff : ∀ e : Bytes, cut e = e ↔ (61 : UInt8) ∉ e
  | [] => by simp [cut]
  | c :: rest => by
    by_cases h : c = 61
    · subst h; simp [cut]
    · simp [cut, h, cut_eq_self_iff rest, Ne.symm h]

/-- A record with no NUL, then its NUL, is read whole, after the bytes already read. -/
theorem entriesFrom_record (rest : Bytes) :
    ∀ (e record : Bytes), (0 : UInt8) ∉ e →
      entriesFrom (e ++ 0 :: rest) record = (record.reverse ++ e) :: entriesFrom rest []
  | [], record, _ => by simp [entriesFrom]
  | c :: e, record, h => by
    have hc : c ≠ 0 := fun hc => h (by simp [hc])
    have he : (0 : UInt8) ∉ e := fun he => h (by simp [he])
    simp only [List.cons_append, entriesFrom, hc, ite_false]
    rw [entriesFrom_record rest e (c :: record) he]
    simp

/-- `mapfile` reads back exactly the entries of a block laid out by the kernel. -/
theorem entries_layout : ∀ block : List Bytes, NoNul block → entries (layout block) = block
  | [], _ => by simp [entries, layout, entriesFrom]
  | e :: block, h => by
    have he : (0 : UInt8) ∉ e := h e (by simp)
    have hb : NoNul block := fun x hx => h x (by simp [hx])
    have ih := entries_layout block hb
    simp only [entries, layout, List.flatMap_cons] at ih ⊢
    rw [List.append_assoc, List.singleton_append, entriesFrom_record _ e [] he, ih]
    simp

/-- No listed name holds a space. -/
theorem listed_no_space : ∀ k ∈ listedSettings, (32 : UInt8) ∉ k := by decide +kernel

/-- No listed name is empty. -/
theorem listed_nonempty : ∀ k ∈ listedSettings, k ≠ [] := by decide +kernel

/-- The record's pattern test is a list infix. -/
theorem isInfixOf_iff (needle : Bytes) : ∀ hay : Bytes, isInfixOf needle hay = true ↔ needle <:+: hay
  | [] => by simp [isInfixOf, startsWith_iff, List.infix_nil]
  | c :: rest => by
    rw [isInfixOf, Bool.or_eq_true, startsWith_iff, isInfixOf_iff needle rest, List.infix_cons_iff]

/-- A spaceless word and a space begin a spaceless word and a space exactly when the words are equal. -/
theorem word_prefix : ∀ (k g tail : Bytes), (32 : UInt8) ∉ k → (32 : UInt8) ∉ g →
    (k ++ [32] <+: g ++ 32 :: tail ↔ k = g)
  | [], [], tail, _, _ => by simp
  | [], c :: g, tail, _, hg => by
    have hc : (32 : UInt8) ≠ c := fun h => hg (by simp [h])
    simp [List.cons_prefix_cons, hc]
  | a :: k, [], tail, hk, _ => by
    have ha : a ≠ 32 := fun h => hk (by simp [h])
    simp [List.cons_prefix_cons, ha]
  | a :: k, c :: g, tail, hk, hg => by
    have hk' : (32 : UInt8) ∉ k := fun h => hk (by simp [h])
    have hg' : (32 : UInt8) ∉ g := fun h => hg (by simp [h])
    simp only [List.cons_append, List.cons_prefix_cons, word_prefix k g tail hk' hg',
      List.cons.injEq]

/-- A needle that begins with a space occurs after a spaceless run exactly when it occurs in what
follows the run. -/
theorem spaceless_infix (needle hay : Bytes) (hn : needle.head? = some 32) :
    ∀ g : Bytes, (32 : UInt8) ∉ g → (needle <:+: g ++ hay ↔ needle <:+: hay)
  | [], _ => by simp
  | c :: g, hg => by
    have hc : c ≠ 32 := fun h => hg (by simp [h])
    have hg' : (32 : UInt8) ∉ g := fun h => hg (by simp [h])
    rw [List.cons_append, List.infix_cons_iff, spaceless_infix needle hay hn g hg']
    constructor
    · rintro (h | h)
      · cases needle with
        | nil => simp at hn
        | cons d n =>
          simp at hn
          rw [List.cons_prefix_cons] at h
          exact absurd (hn ▸ h.1).symm hc
      · exact h
    · intro h; exact Or.inr h

/-- The record lemma: a spaceless key between two spaces occurs in the record of spaceless keys
exactly when it was given before. -/
theorem record_infix (k : Bytes) (hk : (32 : UInt8) ∉ k) :
    ∀ gs : List Bytes, (∀ g ∈ gs, (32 : UInt8) ∉ g) →
      (isInfixOf ([32] ++ k ++ [32]) ([32] ++ gs.flatMap (· ++ [32])) = true ↔ k ∈ gs)
  | [], _ => by
    rw [isInfixOf_iff]
    simp only [List.flatMap_nil, List.append_nil, List.not_mem_nil, iff_false]
    intro h
    have := h.length_le
    simp at this
  | g :: gs, hgs => by
    have hg : (32 : UInt8) ∉ g := hgs g (by simp)
    have hrest : ∀ x ∈ gs, (32 : UInt8) ∉ x := fun x hx => hgs x (by simp [hx])
    have ih := record_infix k hk gs hrest
    rw [isInfixOf_iff] at ih ⊢
    have hshape : [32] ++ (g :: gs).flatMap (· ++ [32]) =
        (32 : UInt8) :: (g ++ ([32] ++ gs.flatMap (· ++ [32]))) := by simp
    rw [hshape, List.infix_cons_iff, spaceless_infix _ _ (by simp) g hg, ih]
    have hp : ([32] ++ k ++ [32] <+: (32 : UInt8) :: (g ++ ([32] ++ gs.flatMap (· ++ [32]))))
        ↔ k = g := by
      simp only [List.cons_append, List.cons_prefix_cons, true_and]
      exact word_prefix k g _ hk hg
    rw [hp, List.mem_cons]

/-- The shell-variable loop passes exactly when every prefixed name is listed. -/
theorem variablesLoop_none :
    ∀ vars : List Bytes, variablesLoop vars = none ↔ ∀ v ∈ vars, pfx <+: v → v ∈ listedSettings
  | [] => by simp [variablesLoop]
  | v :: rest => by
    rw [variablesLoop, List.forall_mem_cons, ← variablesLoop_none rest, ← startsWith_iff,
      fields_settingsText, ← named_iff]
    by_cases hs : startsWith pfx v = true
    · by_cases hn : named v listedSettings = true
      · simp [hs, hn]
      · simp [hs, hn]
    · simp [hs]

/-- The entry loop passes, from the record of the keys `gs`, exactly when every prefixed key is listed
with `=`, the prefixed keys are distinct, and none of them is in `gs`. -/
theorem entryLoop_none :
    ∀ (block gs : List Bytes), (∀ g ∈ gs, g ∈ listedSettings) →
      (entryLoop listedSettings ([32] ++ gs.flatMap (· ++ [32])) block = none ↔
        (∀ e ∈ block, pfx <+: keyOf e → keyOf e ∈ listedSettings ∧ (61 : UInt8) ∈ e) ∧
        (prefixedKeys block).Nodup ∧ ∀ k ∈ prefixedKeys block, k ∉ gs)
  | [], gs, _ => by simp [entryLoop, prefixedKeys]
  | e :: rest, gs, hgs => by
    rw [entryLoop, List.forall_mem_cons, cut_eq_keyOf]
    by_cases hs : startsWith pfx (keyOf e) = true
    · have hs' : pfx <+: keyOf e := (startsWith_iff _ _).1 hs
      have hpk : prefixedKeys (e :: rest) = keyOf e :: prefixedKeys rest := by
        simp [prefixedKeys, hs']
      rw [hpk]
      by_cases hn : named (keyOf e) listedSettings = true
      · have hn' : keyOf e ∈ listedSettings := (named_iff _ _).1 hn
        by_cases hv : keyOf e ≠ e
        · have h61 : (61 : UInt8) ∈ e := by
            rw [← cut_eq_keyOf] at hv
            exact Classical.byContradiction fun h => hv ((cut_eq_self_iff e).2 h)
          have hspace : ∀ g ∈ gs, (32 : UInt8) ∉ g := fun g hg => listed_no_space g (hgs g hg)
          by_cases ht : isInfixOf ([32] ++ keyOf e ++ [32]) ([32] ++ gs.flatMap (· ++ [32])) = true
          · have hin : keyOf e ∈ gs :=
              (record_infix _ (listed_no_space _ hn') gs hspace).1 ht
            simp only [hs, hn, hv, ht, ite_true, ne_eq, not_false_eq_true]
            simp only [reduceCtorEq, false_iff, not_and]
            intro _ _ hall
            exact (hall (keyOf e) (by simp)) hin
          · have hout : keyOf e ∉ gs := fun h =>
              ht ((record_infix _ (listed_no_space _ hn') gs hspace).2 h)
            have hgs' : ∀ g ∈ gs ++ [keyOf e], g ∈ listedSettings := by
              intro g hg
              rcases List.mem_append.1 hg with h | h
              · exact hgs g h
              · rw [List.mem_singleton] at h; exact h ▸ hn'
            have ih := entryLoop_none rest (gs ++ [keyOf e]) hgs'
            have hrec : [32] ++ gs.flatMap (· ++ [32]) ++ keyOf e ++ [32] =
                [32] ++ (gs ++ [keyOf e]).flatMap (· ++ [32]) := by simp
            simp only [hs, hn, hv, ht, ite_true, ite_false, ne_eq, not_false_eq_true,
              Bool.false_eq_true]
            rw [hrec, ih]
            simp only [List.nodup_cons, List.forall_mem_cons, List.mem_append, List.mem_singleton,
              not_or]
            constructor
            · rintro ⟨h1, h2, h3⟩
              exact ⟨⟨fun _ => ⟨hn', h61⟩, h1⟩, ⟨fun hk => (h3 _ hk).2 rfl, h2⟩,
                ⟨hout, fun k hk => (h3 k hk).1⟩⟩
            · rintro ⟨⟨_, h1⟩, ⟨h2, h3⟩, ⟨_, h4⟩⟩
              exact ⟨h1, h3, fun k hk => ⟨h4 k hk, fun he => h2 (he ▸ hk)⟩⟩
        · have hv' : keyOf e = e := Classical.byContradiction hv
          have h61 : (61 : UInt8) ∉ e := by
            rw [← cut_eq_keyOf] at hv'
            exact (cut_eq_self_iff e).1 hv'
          simp only [hs, hn, hv, ite_true, ite_false]
          simp only [reduceCtorEq, false_iff, not_and]
          intro h
          exact absurd (h.1 hs').2 h61
      · have hn' : keyOf e ∉ listedSettings := fun h => hn ((named_iff _ _).2 h)
        simp only [hs, hn, ite_true, ite_false, Bool.false_eq_true]
        simp only [reduceCtorEq, false_iff, not_and]
        intro h
        exact absurd (h.1 hs').1 hn'
    · have hs' : ¬ pfx <+: keyOf e := fun h => hs ((startsWith_iff _ _).2 h)
      have hpk : prefixedKeys (e :: rest) = prefixedKeys rest := by
        simp [prefixedKeys, hs']
      rw [hpk]
      simp only [hs, ite_false, Bool.false_eq_true]
      rw [entryLoop_none rest gs hgs]
      simp [hs']

/-! ## The theorem -/

/-- The port proceeds on a block the kernel lays out exactly when the block is admissible. -/
theorem proceeds_iff (vars block : List Bytes) (h : NoNul block) :
    refuseUnlistedSettings vars (some (layout block)) = .proceed ↔ Admissible vars block := by
  have he := entryLoop_none block [] (by simp)
  simp only [List.flatMap_nil, List.append_nil, List.not_mem_nil, not_false_eq_true,
    implies_true, and_true] at he
  unfold Admissible
  cases hv : variablesLoop vars with
  | some f =>
    have hv' : ¬ ∀ v ∈ vars, pfx <+: v → v ∈ listedSettings := fun hall => by
      rw [← variablesLoop_none, hv] at hall
      exact absurd hall (by simp)
    simp only [refuseUnlistedSettings, hv, reduceCtorEq, false_iff]
    intro ⟨_, _, _, h4⟩
    exact hv' h4
  | none =>
    have hv' := (variablesLoop_none vars).1 hv
    simp only [refuseUnlistedSettings, hv, entries_layout block h, fields_settingsText]
    by_cases hb : block = []
    · simp [hb]
    · simp only [hb, ite_false]
      cases hl : entryLoop listedSettings [32] block with
      | some f =>
        simp only [reduceCtorEq, false_iff]
        intro ⟨_, h2, h3, _⟩
        have := he.2 ⟨h2, h3⟩
        rw [hl] at this
        exact absurd this (by simp)
      | none =>
        have := he.1 hl
        simp only [true_iff]
        exact ⟨hb, this.1, this.2, hv'⟩

/-- A read that fails never proceeds. -/
theorem never_proceeds_unread (vars : List Bytes) : refuseUnlistedSettings vars none ≠ .proceed := by
  unfold refuseUnlistedSettings
  cases variablesLoop vars <;> simp

/-- Paired blocks are empty together. -/
theorem sameKeys_nil {a b : List Bytes} (h : SameKeys a b) : a = [] ↔ b = [] := by
  cases a <;> cases b <;> simp_all [SameKeys]

/-- What reads only each entry's key and its presence of `=` holds of paired blocks alike. -/
theorem sameKeys_each (P : Bytes → Prop → Prop) :
    ∀ {a b : List Bytes}, SameKeys a b →
      ((∀ e ∈ a, P (keyOf e) ((61 : UInt8) ∈ e)) ↔ (∀ e ∈ b, P (keyOf e) ((61 : UInt8) ∈ e)))
  | [], [], _ => by simp
  | x :: a, y :: b, h => by
    rw [List.forall_mem_cons, List.forall_mem_cons, h.1.1, propext h.1.2, sameKeys_each P h.2]
  | [], _ :: _, h => by simp [SameKeys] at h
  | _ :: _, [], h => by simp [SameKeys] at h

/-- Paired blocks have the same keys, in order. -/
theorem sameKeys_map : ∀ {a b : List Bytes}, SameKeys a b → a.map keyOf = b.map keyOf
  | [], [], _ => rfl
  | x :: a, y :: b, h => by simp [h.1.1, sameKeys_map h.2]
  | [], _ :: _, h => by simp [SameKeys] at h
  | _ :: _, [], h => by simp [SameKeys] at h

/-- Paired blocks are admissible alike. -/
theorem sameKeys_admissible (vars : List Bytes) {a b : List Bytes} (h : SameKeys a b) :
    Admissible vars a ↔ Admissible vars b := by
  have hn := sameKeys_nil h
  have he := sameKeys_each (fun k has => pfx <+: k → k ∈ listedSettings ∧ has) h
  unfold Admissible prefixedKeys
  rw [sameKeys_map h, ne_eq, ne_eq, hn, he]

/-- The settings check that ships satisfies the claim. -/
theorem a_caddy_step_proceeds_exactly_on_listed_settings_each_given_once_with_a_value :
    Claim refuseUnlistedSettings := by
  refine ⟨proceeds_iff, never_proceeds_unread, fun vars a b ha hb h => ?_⟩
  rw [proceeds_iff vars a ha, proceeds_iff vars b hb]
  exact sameKeys_admissible vars h

/-! ## Inputs that leave the verdict as it was, and the issue's cases -/

/-- A listed or unprefixed name, inserted among the shell's variables, leaves the shell-variable loop
as it was. -/
theorem variablesLoop_inert (v : Bytes)
    (hv : startsWith pfx v = false ∨ named v listedSettings = true) :
    ∀ pre post : List Bytes, variablesLoop (pre ++ v :: post) = variablesLoop (pre ++ post)
  | [], post => by
    rcases hv with hv | hv
    · simp [variablesLoop, hv]
    · simp [variablesLoop, fields_settingsText, hv]
  | w :: pre, post => by
    simp only [List.cons_append, variablesLoop, variablesLoop_inert v hv pre post]

/-- A listed or unprefixed name inserted anywhere among the shell's variables leaves the verdict as
it was: the elevation setting the vectors test makes, and an unprefixed name bash makes of a locale
entry, move no verdict. -/
theorem inert_variable (v : Bytes)
    (hv : startsWith pfx v = false ∨ named v listedSettings = true)
    (pre post : List Bytes) (read : Option Bytes) :
    refuseUnlistedSettings (pre ++ v :: post) read = refuseUnlistedSettings (pre ++ post) read := by
  simp only [refuseUnlistedSettings, variablesLoop_inert v hv pre post]

/-- An unprefixed entry appended to the entries leaves the entry loop as it was. -/
theorem entryLoop_inert (listed : List Bytes) (e : Bytes) (he : startsWith pfx (cut e) = false) :
    ∀ (block : List Bytes) (given : Bytes),
      entryLoop listed given (block ++ [e]) = entryLoop listed given block
  | [], given => by simp [entryLoop, he]
  | x :: block, given => by
    simp only [List.cons_append, entryLoop, entryLoop_inert listed e he block]

/-- An unprefixed entry with no NUL, appended to a block that holds an entry, leaves the verdict as
it was: the locale entries the vectors test appends move no verdict. -/
theorem inert_entry (vars block : List Bytes) (e : Bytes) (hb : block ≠ [])
    (he : startsWith pfx (cut e) = false) (h0 : (0 : UInt8) ∉ e) (hn : NoNul block) :
    refuseUnlistedSettings vars (some (layout (block ++ [e]))) =
      refuseUnlistedSettings vars (some (layout block)) := by
  have hn' : NoNul (block ++ [e]) := by
    intro x hx
    rcases List.mem_append.1 hx with h | h
    · exact hn x h
    · rw [List.mem_singleton] at h
      exact h ▸ h0
  simp [refuseUnlistedSettings, entries_layout _ hn', entries_layout _ hn, entryLoop_inert _ e he,
    hb]

-- The issue's four cases. The empty block is refused as unread.
example : refuseUnlistedSettings [] (some (layout [])) = .refuse .unread := by decide +kernel

-- A listed setting with no `=` is given without a value.
example :
    refuseUnlistedSettings [] (some (layout [ascii "DECKSTREAK_DEPLOY_REPO"])) =
      .refuse (.noValue (ascii "DECKSTREAK_DEPLOY_REPO")) := by
  decide +kernel

-- A listed setting given twice is refused at its second entry.
example :
    refuseUnlistedSettings []
        (some (layout [ascii "DECKSTREAK_DEPLOY_REPO=a", ascii "DECKSTREAK_DEPLOY_REPO=b"])) =
      .refuse (.twice (ascii "DECKSTREAK_DEPLOY_REPO")) := by
  decide +kernel

-- A key no shell name can hold is not a setting.
example :
    refuseUnlistedSettings [] (some (layout [pfx ++ [1] ++ ascii "=/x"])) =
      .refuse (.notASetting (pfx ++ [1])) := by
  decide +kernel

/-! ## The witness -/

/-- The recorded counterexample's entry: the prefix, the byte 0x01, then `=/x`. -/
def unspellable : Bytes := pfx ++ [1] ++ ascii "=/x"

/-- The check at dev `dafbbbbb` proceeds on the block that holds only the entry whose key is the
prefix followed by the byte 0x01, with no shell variable, so the claim is false of it. -/
theorem the_shells_variables_alone_admit_an_entry_no_shell_can_spell : ¬ Claim variablesAlone := by
  intro h
  have hnul : NoNul [unspellable] := by unfold NoNul; decide +kernel
  have hadm := (h.1 [] [unspellable] hnul).1 rfl
  have hpfx : pfx <+: keyOf unspellable := (startsWith_iff _ _).1 (by decide +kernel)
  exact absurd (hadm.2.1 unspellable (by simp) hpfx).1 (by decide +kernel)

end Formal.DeployEntryRefusal
