-- @phx covers crates/companion/src/decide.rs anchor=decide digest=sha256:d04804c51bd28b4ad0cda4238f8880afcf5e9c255c37f2cc84e8572888c62136
-- @phx covers crates/companion/src/decide.rs anchor=negotiate digest=sha256:237842bcfd29e9a163de5967d4e528060da51dc8b4724498545cab1c7f493079
-- @phx vectors formal/vectors/companion-decide.jsonl
-- @phx cites #997
-- @phx theorem decide_admits_exactly_when_every_condition_holds ramp=report
-- @phx witness a_decide_that_skips_the_epoch_admits_a_stale_reply kills=decide_admits_exactly_when_every_condition_holds
-- @phx theorem decide_names_the_session_and_status_refusals ramp=report
-- @phx witness a_decide_without_the_session_admits_an_unnegotiated_reply kills=decide_names_the_session_and_status_refusals
-- @phx theorem negotiate_admits_only_a_present_trained_companion_at_major_one ramp=report
-- @phx witness a_negotiate_by_the_minor_admits_another_major kills=negotiate_admits_only_a_present_trained_companion_at_major_one
-- @phx witness a_mode_blind_negotiate_admits_a_fixture kills=negotiate_admits_only_a_present_trained_companion_at_major_one

/-!
# Formal.CompanionDecide

The scheduling companion's two decisions (SPEC-501), `decide` and `negotiate` in
`crates/companion/src/decide.rs`, as total functions. `decide` is the one function whose `Ok`
lets a due date be written; `negotiate` is the handshake that gives a session its model.

**The claim.** For every input:
- `decide_admits_exactly_when_every_condition_holds`: `decide` is `ok d` exactly when the session
  negotiated a model, the reply is a due date with status ok for the asked card and the asked
  press, from the session's epoch and the negotiated model, its interval is 1 to 36,500 days, its
  due day is the press's study day plus the interval, and `d` is that card, interval and day;
- `decide_names_the_session_and_status_refusals`: with no negotiated model it refuses as not
  negotiated; otherwise a reply whose status is absent refuses as absent, and any other status but
  ok refuses as that status, so no failed or absent companion yields a due date;
- `negotiate_admits_only_a_present_trained_companion_at_major_one`: the handshake is `ok m`
  exactly when the status is ok, the major is 1 and the model is trained, and `m` is the reply's
  model id.

**What is ported.** The checks in their source order. A model id is 256 bits that are only
compared, so the port reads it as one natural number. The card, the press's id and the due day
are integers; the epoch, the interval and the major are naturals. The due day's checked `i32`
addition is the port's `Int` addition: a due day is an `i32`, so when the sum leaves the `i32`
range it cannot equal the due day, and the source's overflow refusal is the port's inequality.
Every reply that is not a due date reads by its status alone, as the source's `Reply::status`
does, so the port has one constructor for them.

**The witnesses** are ports that each break one claim at one input: a decision that skips the
epoch check and admits a stale reply, a decision that decides without a negotiated model, a
handshake that compares the minor where the major belongs, and a handshake that never reads the
model.
-/

namespace Formal.CompanionDecide

/-- `Status`: what a reply says about the companion. -/
inductive Status where
  | ok
  | absent
  | fixtureRefuses
  | busy
  | error
  | conflict
  | unsupported
  deriving DecidableEq, Repr

/-- `Model`: a trained model or a fixture. -/
inductive Model where
  | trained
  | fixture
  deriving DecidableEq, Repr

/-- `Refusal`: the named reasons `decide` and `negotiate` give. -/
inductive Refusal where
  | notNegotiated
  | absent
  | mismatch
  | fixture
  | status (s : Status)
  | staleEpoch
  | wrongCard
  | wrongStage
  | modelChanged
  | intervalOutOfRange
  | dueDayInconsistent
  deriving DecidableEq, Repr

/-- A session: its epoch, and the negotiated model, if any. -/
structure Session where
  epoch : Nat
  model : Option Nat

/-- The press asked about: its card, its review id and its study day. -/
structure Asked where
  card : Int
  tMs : Int
  studyDay : Int

/-- A reply: a due date, or any other reply, which reads by its status alone. -/
inductive Reply where
  | due (status : Status) (card tMs : Int) (interval : Nat) (dueDay : Int) (model : Nat)
  | other (status : Status)

/-- The due date a press may write. -/
structure NewDue where
  card : Int
  interval : Nat
  dueDay : Int
  deriving DecidableEq, Repr

/-- `Reply::status`. -/
def Reply.status : Reply → Status
  | .due s _ _ _ _ _ => s
  | .other s => s

/-- `INTERVAL_DAYS_MAX`. -/
def intervalMax : Nat := 36500

/-- The refusal of a reply that is not a due date, by its status. -/
def byStatus (s : Status) : Refusal :=
  if s = .absent then .absent else if s = .ok then .wrongStage else .status s

/-- The checks of a due date against the press, after the session's model `m`. -/
def checkDue (s : Session) (a : Asked) (e : Nat) (m : Nat)
    (st : Status) (card tMs : Int) (iv : Nat) (dd : Int) (md : Nat) : Except Refusal NewDue :=
  if st = .absent then .error .absent
  else if st ≠ .ok then .error (.status st)
  else if e ≠ s.epoch then .error .staleEpoch
  else if card ≠ a.card then .error .wrongCard
  else if tMs ≠ a.tMs then .error .wrongStage
  else if md ≠ m then .error .modelChanged
  else if iv < 1 ∨ intervalMax < iv then .error .intervalOutOfRange
  else if dd ≠ a.studyDay + iv then .error .dueDayInconsistent
  else .ok ⟨card, iv, dd⟩

/-- `decide`: the session's model, then the reply's kind, then the due date's checks. -/
def decideReply (s : Session) (a : Asked) (e : Nat) (r : Reply) : Except Refusal NewDue :=
  match s.model with
  | none => .error .notNegotiated
  | some m =>
    match r with
    | .other st => .error (byStatus st)
    | .due st card tMs iv dd md => checkDue s a e m st card tMs iv dd md

/-- The checks with the epoch's skipped: the wrong variant. -/
def checkDueWithoutTheEpoch (_s : Session) (a : Asked) (_e : Nat) (m : Nat)
    (st : Status) (card tMs : Int) (iv : Nat) (dd : Int) (md : Nat) : Except Refusal NewDue :=
  if st = .absent then .error .absent
  else if st ≠ .ok then .error (.status st)
  else if card ≠ a.card then .error .wrongCard
  else if tMs ≠ a.tMs then .error .wrongStage
  else if md ≠ m then .error .modelChanged
  else if iv < 1 ∨ intervalMax < iv then .error .intervalOutOfRange
  else if dd ≠ a.studyDay + iv then .error .dueDayInconsistent
  else .ok ⟨card, iv, dd⟩

/-- The wrong variant: a decision that never compares the reply's epoch with the session's. -/
def decideWithoutTheEpoch (s : Session) (a : Asked) (e : Nat) (r : Reply) :
    Except Refusal NewDue :=
  match s.model with
  | none => .error .notNegotiated
  | some m =>
    match r with
    | .other st => .error (byStatus st)
    | .due st card tMs iv dd md => checkDueWithoutTheEpoch s a e m st card tMs iv dd md

/-- The wrong variant: a decision that, with no negotiated model, takes the reply's own. -/
def decideWithoutTheSession (s : Session) (a : Asked) (e : Nat) (r : Reply) :
    Except Refusal NewDue :=
  match r with
  | .other st => .error (byStatus st)
  | .due st card tMs iv dd md => checkDue s a e (s.model.getD md) st card tMs iv dd md

/-- The hello a handshake reads. -/
structure Hello where
  status : Status
  major : Nat
  minor : Nat
  model : Model
  modelId : Nat

/-- `negotiate`: the status, then the major, then the model. -/
def negotiate (h : Hello) : Except Refusal Nat :=
  if h.status = .ok then
    if h.major ≠ 1 then .error .mismatch
    else if h.model ≠ .trained then .error .fixture
    else .ok h.modelId
  else if h.status = .absent then .error .absent
  else .error (.status h.status)

/-- The wrong variant: a handshake that compares the minor where the major belongs. -/
def negotiateByTheMinor (h : Hello) : Except Refusal Nat :=
  if h.status = .ok then
    if h.minor ≠ 0 then .error .mismatch
    else if h.model ≠ .trained then .error .fixture
    else .ok h.modelId
  else if h.status = .absent then .error .absent
  else .error (.status h.status)

/-- The wrong variant: a handshake that never reads the model. -/
def negotiateModeBlind (h : Hello) : Except Refusal Nat :=
  if h.status = .ok then
    if h.major ≠ 1 then .error .mismatch
    else .ok h.modelId
  else if h.status = .absent then .error .absent
  else .error (.status h.status)

/-- Every condition of an admitted due date. -/
def Admits (s : Session) (a : Asked) (e : Nat) (r : Reply) (d : NewDue) : Prop :=
  ∃ m, s.model = some m ∧ r = .due .ok a.card a.tMs d.interval d.dueDay m ∧ e = s.epoch ∧
    d.card = a.card ∧ 1 ≤ d.interval ∧ d.interval ≤ intervalMax ∧
    d.dueDay = a.studyDay + d.interval

/-- The claim, stated once: a decision admits exactly when every condition holds. -/
def AdmitsExactly (dec : Session → Asked → Nat → Reply → Except Refusal NewDue) : Prop :=
  ∀ s a e r d, dec s a e r = .ok d ↔ Admits s a e r d

/-- The claim, stated once: a decision names the session's and the status's refusals. -/
def NamesTheRefusals (dec : Session → Asked → Nat → Reply → Except Refusal NewDue) : Prop :=
  ∀ s a e r,
    (s.model = none → dec s a e r = .error .notNegotiated) ∧
    (s.model ≠ none → r.status = .absent → dec s a e r = .error .absent) ∧
    (s.model ≠ none → r.status ≠ .ok → r.status ≠ .absent →
      dec s a e r = .error (.status r.status))

/-- The claim, stated once: a handshake admits exactly a present trained companion at major 1. -/
def OnlyTrainedAtMajorOne (neg : Hello → Except Refusal Nat) : Prop :=
  ∀ h m, neg h = .ok m ↔ (h.status = .ok ∧ h.major = 1 ∧ h.model = .trained ∧ m = h.modelId)

/-- The due date's checks admit exactly when each of them holds. -/
theorem checkDue_ok (s : Session) (a : Asked) (e m : Nat) (st : Status) (card tMs : Int)
    (iv : Nat) (dd : Int) (md : Nat) (d : NewDue) :
    checkDue s a e m st card tMs iv dd md = .ok d ↔
      (st = .ok ∧ e = s.epoch ∧ card = a.card ∧ tMs = a.tMs ∧ md = m ∧ 1 ≤ iv ∧
        iv ≤ intervalMax ∧ dd = a.studyDay + iv ∧ NewDue.mk card iv dd = d) := by
  unfold checkDue
  rcases Decidable.em (st = .absent) with h1 | h1
  · subst h1
    simp
  rcases (Decidable.em (st = .ok)).symm with h2 | h2
  · simp [h1, h2]
  subst h2
  rcases (Decidable.em (e = s.epoch)).symm with h3 | h3
  · simp [h3]
  rcases (Decidable.em (card = a.card)).symm with h4 | h4
  · simp [h3, h4]
  rcases (Decidable.em (tMs = a.tMs)).symm with h5 | h5
  · simp [h3, h4, h5]
  rcases (Decidable.em (md = m)).symm with h6 | h6
  · simp [h3, h4, h5, h6]
  rcases Decidable.em (iv < 1 ∨ intervalMax < iv) with h7 | h7
  · have h7' : ¬ (1 ≤ iv ∧ iv ≤ intervalMax) := by
      simp only [intervalMax] at h7 ⊢
      omega
    simp only [h3, h4, h5, h6, h7, ne_eq, not_true_eq_false, ite_false, ite_true, reduceCtorEq,
      false_iff, not_and, true_and]
    intro hle hge
    exact absurd ⟨hle, hge⟩ h7'
  have hz : iv ≠ 0 := by
    omega
  have hm : ¬ intervalMax < iv := by
    simp only [intervalMax] at h7 ⊢
    omega
  rcases (Decidable.em (dd = a.studyDay + iv)).symm with h8 | h8
  · simp [h3, h4, h5, h6, h8, hz, hm]
  have hle : 1 ≤ iv := by
    omega
  have hge : iv ≤ intervalMax := by
    simp only [intervalMax] at hm ⊢
    omega
  simp [h3, h4, h5, h6, h8, hz, hm, hle, hge]

theorem decide_admits_exactly_when_every_condition_holds : AdmitsExactly decideReply := by
  intro s a e r d
  rcases s with ⟨se, sm⟩
  cases sm with
  | none =>
    simp [decideReply, Admits]
  | some m =>
    cases r with
    | other st =>
      simp [decideReply, Admits]
    | due st card tMs iv rdd md =>
      simp only [decideReply, checkDue_ok, Admits]
      constructor
      · intro h
        refine ⟨m, rfl, ?_⟩
        obtain ⟨h1, h2, h3, h4, h5, h6, h7, h8, h9⟩ := h
        subst h9
        simp_all
      · rintro ⟨m', hm, hr, he, hc, h6, h7, hd⟩
        cases hm
        cases hr
        rcases d with ⟨dc, di, ddd⟩
        simp_all

theorem decide_names_the_session_and_status_refusals : NamesTheRefusals decideReply := by
  intro s a e r
  rcases s with ⟨se, sm⟩
  refine ⟨?_, ?_, ?_⟩
  · intro h
    simp only at h
    subst h
    rfl
  · intro hs hst
    cases sm with
    | none => exact absurd rfl hs
    | some m =>
      cases r with
      | other st =>
        simp only [Reply.status] at hst
        subst hst
        rfl
      | due st card tMs iv dd md =>
        simp only [Reply.status] at hst
        subst hst
        rfl
  · intro hs hok hab
    cases sm with
    | none => exact absurd rfl hs
    | some m =>
      cases r with
      | other st =>
        simp only [Reply.status] at hok hab
        simp [decideReply, byStatus, Reply.status, hok, hab]
      | due st card tMs iv dd md =>
        simp only [Reply.status] at hok hab
        simp [decideReply, checkDue, Reply.status, hok, hab]

theorem negotiate_admits_only_a_present_trained_companion_at_major_one :
    OnlyTrainedAtMajorOne negotiate := by
  intro h m
  rcases h with ⟨st, major, minor, model, mid⟩
  simp only [negotiate]
  constructor
  · intro hn
    split at hn
    · split at hn
      · cases hn
      · split at hn
        · cases hn
        · rename_i h1 h2 h3
          cases hn
          refine ⟨h1, ?_, ?_, rfl⟩
          · simp_all
          · cases model <;> simp_all
    · split at hn
      · cases hn
      · cases hn
  · rintro ⟨h1, h2, h3, rfl⟩
    simp_all

theorem a_decide_that_skips_the_epoch_admits_a_stale_reply :
    ¬ AdmitsExactly decideWithoutTheEpoch := by
  intro h
  have := (h ⟨7, some 5⟩ ⟨42, 1000, 100⟩ 6 (.due .ok 42 1000 3 103 5) ⟨42, 3, 103⟩).mp
    (by simp [decideWithoutTheEpoch, checkDueWithoutTheEpoch, intervalMax])
  obtain ⟨_, _, _, he, _⟩ := this
  simp at he

theorem a_decide_without_the_session_admits_an_unnegotiated_reply :
    ¬ NamesTheRefusals decideWithoutTheSession := by
  intro h
  have := (h ⟨7, none⟩ ⟨42, 1000, 100⟩ 7 (.due .ok 42 1000 3 103 5)).1 rfl
  simp [decideWithoutTheSession, checkDue, intervalMax] at this

theorem a_negotiate_by_the_minor_admits_another_major :
    ¬ OnlyTrainedAtMajorOne negotiateByTheMinor := by
  intro h
  have := (h ⟨.ok, 2, 0, .trained, 5⟩ 5).mp (by simp [negotiateByTheMinor])
  simp at this

theorem a_mode_blind_negotiate_admits_a_fixture :
    ¬ OnlyTrainedAtMajorOne negotiateModeBlind := by
  intro h
  have := (h ⟨.ok, 1, 0, .fixture, 5⟩ 5).mp (by simp [negotiateModeBlind])
  simp at this

end Formal.CompanionDecide
