-- @phx covers crates/progression/src/exchange.rs anchor=bucket digest=sha256:8305d8b6193cdb51675f01e57b2c57aa59946d6083bf84705050e3035a62ceec
-- @phx covers crates/progression/src/exchange.rs anchor=exchange_rates digest=sha256:09ca5039c2ca964983082f8c8bad48acb5b4660e769b30487e408b280699a2f1
-- @phx covers crates/progression/src/board.rs anchor=best_day digest=sha256:ec7443684a6230783ba8add0af536562a61e8e8f3581e9d6dcf94a00670f5e42
-- @phx covers crates/coordination/src/progression/exchange_view.rs anchor=exchange_window digest=sha256:ca531d999ed3714f9e590af65b0e822725a2cd33031d1bf573b024bde517c69d
-- @phx vectors formal/vectors/exchange.jsonl
-- @phx cites #79, #80
-- @phx theorem totals_keep_every_row ramp=report
-- @phx witness rows_without_a_rollup_dropped_violates kills=totals_keep_every_row
-- @phx theorem a_day_counts_once_per_bucket ramp=report
-- @phx witness per_row_denominator_violates kills=a_day_counts_once_per_bucket
-- @phx theorem rate_defined_iff_a_graduation ramp=report
-- @phx witness defined_on_xp_violates kills=rate_defined_iff_a_graduation
-- @phx theorem bucket_ends_at_the_first_colon ramp=report
-- @phx witness last_colon_bucket_violates kills=bucket_ends_at_the_first_colon
-- @phx theorem best_day_is_the_latest_maximum ramp=report
-- @phx witness oldest_on_tie_violates kills=best_day_is_the_latest_maximum
-- @phx theorem window_spans_the_capped_days ramp=report
-- @phx witness uncapped_window_violates kills=window_spans_the_capped_days

/-! The personal board's best day and the XP exchange readout are pure rules (SPEC-075 R1, R4, R5,
R6). A source's bucket is its text up to and including its first `:`. Each bucket keeps its XP, the
distinct days it paid on and the graduations of those days; its rate is defined exactly when a card
graduated. The best day is the first maximum of the rollups read most recent first. The exchange
window spans the asked days, capped, and ends today.

Each claim is stated once as a `Prop` over the function it judges. The theorem is the claim of the
port, and the witness is the negated claim of a wrong variant, each the mutant of one row of
`scripts/mutation-rows.d/S07500-S07599.json`. -/

namespace Formal.Exchange

/-! ## The bucket of a source -/

/-- `bucket`: the source up to and including its first `:`, or the whole source when it has none.
`source.find(':')` and `source.get(..=at)`, read as characters. -/
def bucket : List Char → List Char
  | [] => []
  | c :: cs => if c = ':' then [':'] else c :: bucket cs

/-- The wrong variant (row S07506, `rfind`): the source up to and including its LAST `:`. -/
def bucketLast (s : List Char) : List Char :=
  match s.reverse.dropWhile (fun c => decide (c ≠ ':')) with
  | [] => s
  | r => r.reverse

/-- A bucket is a prefix of its source holding at most one `:`, its last character when the source
has one, and the whole source when it has none. Together these leave one answer: the source up to
its first `:`. -/
def EndsAtFirstColon (f : List Char → List Char) : Prop :=
  ∀ s : List Char, f s <+: s ∧ (f s).count ':' ≤ 1 ∧ (':' ∈ s → (f s).getLast? = some ':') ∧
    (':' ∉ s → f s = s)

/-! ## The exchange readout -/

/-- One XP row, from either XP table: its study day, its source and its XP. -/
structure Row where
  day : Int
  source : List Char
  amount : Int
deriving DecidableEq

/-- A bucket while the rows are folded: its key, its XP, its graduated cards and the distinct days
it paid on. -/
structure Bucket where
  key : List Char
  total : Int
  graduated : Int
  days : List Int
deriving DecidableEq

/-- One bucket's answer. The rate is the total over the graduated cards, defined when `defined`;
the Rust test checks the division. -/
structure Rate where
  source : List Char
  total : Int
  graduated : Int
  defined : Bool
deriving DecidableEq

/-- `graduations.get(&row.study_day).copied().unwrap_or(0)`: a day with no rollup counts 0. -/
def gradsOn (grads : List (Int × Int)) (day : Int) : Int :=
  (grads.lookup day).getD 0

/-- One row into its bucket: `bucket.total += row.amount`, then the day's graduations only when
`bucket.days.insert(row.study_day)` finds the day new. -/
def addRow (grads : List (Int × Int)) (r : Row) (b : Bucket) : Bucket :=
  if r.day ∈ b.days then { b with total := b.total + r.amount }
  else { b with total := b.total + r.amount, graduated := b.graduated + gradsOn grads r.day,
                days := r.day :: b.days }

/-- The wrong variant (row S07505): every row adds its day's graduations, a repeated day too. -/
def addRowPerRow (grads : List (Int × Int)) (r : Row) (b : Bucket) : Bucket :=
  { b with total := b.total + r.amount, graduated := b.graduated + gradsOn grads r.day,
           days := if r.day ∈ b.days then b.days else r.day :: b.days }

/-- The byte order of the map's `&str` keys: UTF-8 orders as the code points do. -/
def keyLt : List Char → List Char → Bool
  | [], [] => false
  | [], _ :: _ => true
  | _ :: _, [] => false
  | a :: as, b :: bs => if a.toNat < b.toNat then true else if a = b then keyLt as bs else false

/-- Whether the map holds `k`. -/
def hasKey (k : List Char) : List Bucket → Bool
  | [] => false
  | b :: bs => decide (b.key = k) || hasKey k bs

/-- `f` applied to the bucket keyed `k`. -/
def update (f : Bucket → Bucket) (k : List Char) : List Bucket → List Bucket
  | [] => []
  | b :: bs => if b.key = k then f b :: bs else b :: update f k bs

/-- A new bucket, in key order. -/
def insertSorted (nb : Bucket) : List Bucket → List Bucket
  | [] => [nb]
  | b :: bs => if keyLt nb.key b.key then nb :: b :: bs else b :: insertSorted nb bs

/-- `buckets.entry(key).or_default()`, then `f` on the entry: the bucket keyed `k` when the map
holds it, else a new empty one in key order. -/
def upsert (f : Bucket → Bucket) (k : List Char) (l : List Bucket) : List Bucket :=
  if hasKey k l then update f k l else insertSorted (f ⟨k, 0, 0, []⟩) l

/-- The fold over the rows, each into the bucket of its source. -/
def foldRows (add : List (Int × Int) → Row → Bucket → Bucket) (grads : List (Int × Int))
    (rows : List Row) (acc : List Bucket) : List Bucket :=
  rows.foldl (fun acc r => upsert (add grads r) (bucket r.source) acc) acc

/-- One bucket's answer: `let rate_defined = graduated_cards > 0;`. -/
def finish (b : Bucket) : Rate :=
  ⟨b.key, b.total, b.graduated, decide (0 < b.graduated)⟩

/-- The wrong variant (row S07503's neighbour): a rate defined whenever the bucket has XP. -/
def finishOnXp (b : Bucket) : Rate :=
  ⟨b.key, b.total, b.graduated, decide (0 < b.total)⟩

/-- The readout with a given row step and answer. -/
def ratesWith (add : List (Int × Int) → Row → Bucket → Bucket) (fin : Bucket → Rate)
    (rows : List Row) (grads : List (Int × Int)) : List Rate :=
  (foldRows add grads rows []).map fin

/-- `exchange_rates`: each bucket's rate, in key order. -/
def exchangeRates (rows : List Row) (grads : List (Int × Int)) : List Rate :=
  ratesWith addRow finish rows grads

/-- The wrong variant whose rows each add their day's graduations (row S07505). -/
def exchangeRatesPerRow (rows : List Row) (grads : List (Int × Int)) : List Rate :=
  ratesWith addRowPerRow finish rows grads

/-- The wrong variant whose rate is defined whenever its bucket has XP. -/
def exchangeRatesOnXp (rows : List Row) (grads : List (Int × Int)) : List Rate :=
  ratesWith addRow finishOnXp rows grads

/-- The wrong variant: rows joined to the rollups, so a row on a day with no rollup is lost. -/
def exchangeRatesJoined (rows : List Row) (grads : List (Int × Int)) : List Rate :=
  exchangeRates (rows.filter fun r => (grads.lookup r.day).isSome) grads

/-- Every row's XP is in some bucket's total, once. -/
def KeepsEveryRow (f : List Row → List (Int × Int) → List Rate) : Prop :=
  ∀ rows grads, ((f rows grads).map (·.total)).sum = (rows.map (·.amount)).sum

/-- A row on a day its bucket already paid on leaves every bucket's graduated cards unchanged. -/
def DayOncePerBucket (f : List Row → List (Int × Int) → List Rate) : Prop :=
  ∀ rows grads (r : Row), (∃ r' ∈ rows, r'.day = r.day ∧ bucket r'.source = bucket r.source) →
    (f (rows ++ [r]) grads).map (fun x => (x.source, x.graduated)) =
      (f rows grads).map (fun x => (x.source, x.graduated))

/-- A rate is defined exactly when a card graduated. -/
def DefinedIffGraduation (f : List Row → List (Int × Int) → List Rate) : Prop :=
  ∀ rows grads, ∀ x ∈ f rows grads, x.defined = true ↔ 0 < x.graduated

/-! ## The best day -/

/-- One rollup as the board reads it: its study day and its score. -/
structure DayScore where
  day : Int
  score : Int
deriving DecidableEq

/-- `for total in rest { if total.score > best.score { best = *total; } }`. -/
def bestFrom (best : DayScore) : List DayScore → DayScore
  | [] => best
  | t :: ts => bestFrom (if best.score < t.score then t else best) ts

/-- `best_day`: none of no rollup, else the first maximum. -/
def bestDay : List DayScore → Option DayScore
  | [] => none
  | t :: ts => some (bestFrom t ts)

/-- The wrong variant (row S07502, `>=`): a tie moves the best day to the later, older row. -/
def bestFromOnTie (best : DayScore) : List DayScore → DayScore
  | [] => best
  | t :: ts => bestFromOnTie (if best.score ≤ t.score then t else best) ts

/-- The wrong variant's `best_day`. -/
def bestDayOnTie : List DayScore → Option DayScore
  | [] => none
  | t :: ts => some (bestFromOnTie t ts)

/-- Over rollups read most recent first, the best day is one of them, none scores more, and of the
days that score as much it is the latest; there is none only when there is no rollup. -/
def LatestMaximum (f : List DayScore → Option DayScore) : Prop :=
  ∀ ts : List DayScore, ts.Pairwise (fun x y => y.day < x.day) →
    (f ts = none ↔ ts = []) ∧
    ∀ b, f ts = some b →
      b ∈ ts ∧ (∀ t ∈ ts, t.score ≤ b.score) ∧ (∀ t ∈ ts, t.score = b.score → t.day ≤ b.day)

/-! ## The exchange window -/

/-- `EXCHANGE_WINDOW_CAP`. -/
def windowCap : Int := 3650

/-- `exchange_window`: none for no positive day count, else the capped span ending today. -/
def exchangeWindow (days today : Int) : Option (Int × Int) :=
  if days ≤ 0 then none else some (today - (min days windowCap - 1), today)

/-- The wrong variant: the span is never capped. -/
def exchangeWindowUncapped (days today : Int) : Option (Int × Int) :=
  if days ≤ 0 then none else some (today - (days - 1), today)

/-- No positive day count asks for every day; a positive one spans that many days, at most the
cap, ending today. -/
def SpansCappedDays (f : Int → Int → Option (Int × Int)) : Prop :=
  ∀ days today : Int, (days ≤ 0 → f days today = none) ∧
    (0 < days → ∃ first, f days today = some (first, today) ∧ today - first + 1 = min days 3650)

/-! ## Lemmas -/

theorem key_addRow (g : List (Int × Int)) (r : Row) (b : Bucket) : (addRow g r b).key = b.key := by
  unfold addRow; split <;> rfl

theorem total_addRow (g : List (Int × Int)) (r : Row) (b : Bucket) :
    (addRow g r b).total = b.total + r.amount := by
  unfold addRow; split <;> rfl

theorem day_mem_addRow (g : List (Int × Int)) (r : Row) (b : Bucket) :
    r.day ∈ (addRow g r b).days := by
  unfold addRow; split
  · assumption
  · simp

theorem mem_days_addRow (g : List (Int × Int)) (r : Row) (b : Bucket) (d : Int)
    (h : d ∈ b.days) : d ∈ (addRow g r b).days := by
  unfold addRow; split
  · exact h
  · simp [h]

theorem graduated_addRow_present (g : List (Int × Int)) (r : Row) (b : Bucket)
    (h : r.day ∈ b.days) : (addRow g r b).graduated = b.graduated := by
  simp [addRow, h]

/-- The days of the first bucket keyed `k`, or none. -/
def daysOf (k : List Char) : List Bucket → List Int
  | [] => []
  | b :: bs => if b.key = k then b.days else daysOf k bs

theorem hasKey_of_mem_daysOf (k : List Char) (l : List Bucket) (d : Int)
    (h : d ∈ daysOf k l) : hasKey k l = true := by
  induction l with
  | nil => simp [daysOf] at h
  | cons b bs ih =>
    by_cases hk : b.key = k
    · simp [hasKey, hk]
    · simp only [daysOf, hk, ite_false] at h
      simp [hasKey, hk, ih h]

theorem daysOf_of_not_hasKey (k : List Char) (l : List Bucket) (h : hasKey k l = false) :
    daysOf k l = [] := by
  induction l with
  | nil => rfl
  | cons b bs ih =>
    simp only [hasKey, Bool.or_eq_false_iff, decide_eq_false_iff_not] at h
    simp [daysOf, h.1, ih h.2]

theorem sum_update (g : List (Int × Int)) (r : Row) (k : List Char) (l : List Bucket)
    (h : hasKey k l = true) :
    ((update (addRow g r) k l).map (·.total)).sum = (l.map (·.total)).sum + r.amount := by
  induction l with
  | nil => simp [hasKey] at h
  | cons b bs ih =>
    by_cases hk : b.key = k
    · simp only [update, hk, ite_true, List.map_cons, List.sum_cons, total_addRow]
      omega
    · simp only [hasKey, hk, decide_false, Bool.false_or] at h
      simp only [update, hk, ite_false, List.map_cons, List.sum_cons, ih h]
      omega

theorem sum_insertSorted (nb : Bucket) (l : List Bucket) :
    ((insertSorted nb l).map (·.total)).sum = (l.map (·.total)).sum + nb.total := by
  induction l with
  | nil => simp [insertSorted]
  | cons b bs ih =>
    unfold insertSorted
    split
    · simp only [List.map_cons, List.sum_cons]; omega
    · simp only [List.map_cons, List.sum_cons, ih]; omega

theorem sum_upsert (g : List (Int × Int)) (r : Row) (k : List Char) (l : List Bucket) :
    ((upsert (addRow g r) k l).map (·.total)).sum = (l.map (·.total)).sum + r.amount := by
  unfold upsert
  split
  · exact sum_update g r k l (by assumption)
  · rw [sum_insertSorted, total_addRow]; simp

theorem sum_foldRows (g : List (Int × Int)) (rows : List Row) (acc : List Bucket) :
    ((foldRows addRow g rows acc).map (·.total)).sum =
      (acc.map (·.total)).sum + (rows.map (·.amount)).sum := by
  induction rows generalizing acc with
  | nil => simp [foldRows]
  | cons x xs ih =>
    have e : foldRows addRow g (x :: xs) acc =
        foldRows addRow g xs (upsert (addRow g x) (bucket x.source) acc) := rfl
    rw [e, ih, sum_upsert]
    simp only [List.map_cons, List.sum_cons]
    omega

theorem mem_daysOf_update (g : List (Int × Int)) (r : Row) (k : List Char) (l : List Bucket)
    (h : hasKey k l = true) : r.day ∈ daysOf k (update (addRow g r) k l) := by
  induction l with
  | nil => simp [hasKey] at h
  | cons b bs ih =>
    by_cases hk : b.key = k
    · simp only [update, hk, ite_true, daysOf, key_addRow]
      exact day_mem_addRow g r b
    · simp only [hasKey, hk, decide_false, Bool.false_or] at h
      simp only [update, hk, ite_false, daysOf]
      exact ih h

theorem daysOf_insertSorted_self (nb : Bucket) (l : List Bucket) (h : hasKey nb.key l = false) :
    daysOf nb.key (insertSorted nb l) = nb.days := by
  induction l with
  | nil => simp [insertSorted, daysOf]
  | cons b bs ih =>
    simp only [hasKey, Bool.or_eq_false_iff, decide_eq_false_iff_not] at h
    unfold insertSorted
    split
    · simp [daysOf]
    · simp only [daysOf, h.1, ite_false]
      exact ih h.2

theorem daysOf_insertSorted_other (nb : Bucket) (k : List Char) (l : List Bucket)
    (h : nb.key ≠ k) : daysOf k (insertSorted nb l) = daysOf k l := by
  induction l with
  | nil => simp [insertSorted, daysOf, h]
  | cons b bs ih =>
    unfold insertSorted
    split
    · simp [daysOf, h]
    · by_cases hb : b.key = k
      · simp [daysOf, hb]
      · simp only [daysOf, hb, ite_false]
        exact ih

theorem mem_daysOf_upsert (g : List (Int × Int)) (r : Row) (k : List Char) (l : List Bucket) :
    r.day ∈ daysOf k (upsert (addRow g r) k l) := by
  unfold upsert
  split
  · exact mem_daysOf_update g r k l (by assumption)
  · rename_i hk
    have e := daysOf_insertSorted_self (addRow g r ⟨k, 0, 0, []⟩) l
      (by rw [key_addRow]; simpa using hk)
    rw [key_addRow] at e
    rw [e]
    exact day_mem_addRow g r _

theorem daysOf_mono_update (g : List (Int × Int)) (r : Row) (k k' : List Char) (l : List Bucket)
    (d : Int) (h : d ∈ daysOf k' l) : d ∈ daysOf k' (update (addRow g r) k l) := by
  induction l with
  | nil => simp [daysOf] at h
  | cons b bs ih =>
    by_cases hk : b.key = k
    · simp only [update, hk, ite_true]
      by_cases hk' : b.key = k'
      · simp only [daysOf, key_addRow, hk', ite_true] at h ⊢
        exact mem_days_addRow g r b d h
      · simp only [daysOf, key_addRow, hk', ite_false] at h ⊢
        exact h
    · simp only [update, hk, ite_false]
      by_cases hk' : b.key = k'
      · simp only [daysOf, hk', ite_true] at h ⊢
        exact h
      · simp only [daysOf, hk', ite_false] at h ⊢
        exact ih h

theorem daysOf_mono_upsert (g : List (Int × Int)) (r : Row) (k k' : List Char) (l : List Bucket)
    (d : Int) (h : d ∈ daysOf k' l) : d ∈ daysOf k' (upsert (addRow g r) k l) := by
  unfold upsert
  split
  · exact daysOf_mono_update g r k k' l d h
  · rename_i hk
    by_cases e : k = k'
    · subst e
      rw [daysOf_of_not_hasKey k l (by simpa using hk)] at h
      simp at h
    · rw [daysOf_insertSorted_other _ _ _ (by rw [key_addRow]; exact e)]
      exact h

theorem daysOf_mono_foldRows (g : List (Int × Int)) (rows : List Row) (acc : List Bucket)
    (k : List Char) (d : Int) (h : d ∈ daysOf k acc) :
    d ∈ daysOf k (foldRows addRow g rows acc) := by
  induction rows generalizing acc with
  | nil => exact h
  | cons x xs ih =>
    exact ih _ (daysOf_mono_upsert g x (bucket x.source) k acc d h)

theorem mem_daysOf_foldRows (g : List (Int × Int)) (rows : List Row) (acc : List Bucket)
    (r' : Row) (h : r' ∈ rows) : r'.day ∈ daysOf (bucket r'.source) (foldRows addRow g rows acc) := by
  induction rows generalizing acc with
  | nil => simp at h
  | cons x xs ih =>
    have e : foldRows addRow g (x :: xs) acc =
        foldRows addRow g xs (upsert (addRow g x) (bucket x.source) acc) := rfl
    rw [e]
    rcases List.mem_cons.mp h with hx | hx
    · subst hx
      exact daysOf_mono_foldRows g xs _ _ _ (mem_daysOf_upsert g r' (bucket r'.source) acc)
    · exact ih _ hx

theorem proj_update_present (g : List (Int × Int)) (r : Row) (k : List Char) (l : List Bucket)
    (h : r.day ∈ daysOf k l) :
    (update (addRow g r) k l).map (fun b => (b.key, b.graduated)) =
      l.map (fun b => (b.key, b.graduated)) := by
  induction l with
  | nil => simp [daysOf] at h
  | cons b bs ih =>
    by_cases hk : b.key = k
    · simp only [daysOf, hk, ite_true] at h
      simp [update, hk, key_addRow, graduated_addRow_present g r b h]
    · simp only [daysOf, hk, ite_false] at h
      simp only [update, hk, ite_false, List.map_cons, ih h]

theorem proj_upsert_present (g : List (Int × Int)) (r : Row) (k : List Char) (l : List Bucket)
    (h : r.day ∈ daysOf k l) :
    (upsert (addRow g r) k l).map (fun b => (b.key, b.graduated)) =
      l.map (fun b => (b.key, b.graduated)) := by
  unfold upsert
  simp only [hasKey_of_mem_daysOf k l r.day h, ↓reduceIte]
  exact proj_update_present g r k l h

theorem bestFrom_spec (a : DayScore) (ts : List DayScore)
    (hp : ts.Pairwise (fun x y => y.day < x.day)) (hd : ∀ t ∈ ts, t.day < a.day) :
    (bestFrom a ts = a ∨ bestFrom a ts ∈ ts) ∧ a.score ≤ (bestFrom a ts).score ∧
      (∀ t ∈ ts, t.score ≤ (bestFrom a ts).score) ∧
      (a.score = (bestFrom a ts).score → a.day ≤ (bestFrom a ts).day) ∧
      (∀ t ∈ ts, t.score = (bestFrom a ts).score → t.day ≤ (bestFrom a ts).day) := by
  induction ts generalizing a with
  | nil => simp [bestFrom]
  | cons t ts ih =>
    rw [List.pairwise_cons] at hp
    obtain ⟨ht, hp⟩ := hp
    have hta : t.day < a.day := hd t (by simp)
    have hda : ∀ u ∈ ts, u.day < a.day := fun u hu => hd u (by simp [hu])
    by_cases hs : a.score < t.score
    · have e : bestFrom a (t :: ts) = bestFrom t ts := by simp [bestFrom, hs]
      rw [e]
      obtain ⟨m, h1, h2, h3, h4⟩ := ih t hp ht
      refine ⟨?_, by omega, ?_, fun h => by omega, ?_⟩
      · rcases m with m | m
        · right; rw [m]; simp
        · right; simp [m]
      · intro u hu
        rcases List.mem_cons.mp hu with rfl | hu
        · exact h1
        · exact h2 u hu
      · intro u hu hs2
        rcases List.mem_cons.mp hu with rfl | hu
        · exact h3 hs2
        · exact h4 u hu hs2
    · have e : bestFrom a (t :: ts) = bestFrom a ts := by simp [bestFrom, hs]
      rw [e]
      obtain ⟨m, h1, h2, h3, h4⟩ := ih a hp hda
      refine ⟨?_, h1, ?_, h3, ?_⟩
      · rcases m with m | m
        · left; exact m
        · right; simp [m]
      · intro u hu
        rcases List.mem_cons.mp hu with rfl | hu
        · omega
        · exact h2 u hu
      · intro u hu hs2
        rcases List.mem_cons.mp hu with rfl | hu
        · have := h3 (by omega)
          omega
        · exact h4 u hu hs2

/-! ## The claims of the port -/

theorem totals_keep_every_row : KeepsEveryRow exchangeRates := by
  intro rows grads
  have h := sum_foldRows grads rows []
  simp only [List.map_nil, List.sum_nil, Int.zero_add] at h
  simp only [exchangeRates, ratesWith, List.map_map]
  exact h

theorem a_day_counts_once_per_bucket : DayOncePerBucket exchangeRates := by
  intro rows grads r ⟨r', hr', hday, hkey⟩
  have hm := mem_daysOf_foldRows grads rows [] r' hr'
  rw [hday, hkey] at hm
  have e : foldRows addRow grads (rows ++ [r]) [] =
      upsert (addRow grads r) (bucket r.source) (foldRows addRow grads rows []) := by
    simp [foldRows, List.foldl_append]
  simp only [exchangeRates, ratesWith, List.map_map, e]
  exact proj_upsert_present grads r (bucket r.source) _ hm

theorem rate_defined_iff_a_graduation : DefinedIffGraduation exchangeRates := by
  intro rows grads x hx
  simp only [exchangeRates, ratesWith, List.mem_map] at hx
  obtain ⟨b, _, rfl⟩ := hx
  simp [finish]

theorem bucket_ends_at_the_first_colon : EndsAtFirstColon bucket := by
  intro s
  induction s with
  | nil => simp [bucket]
  | cons c cs ih =>
    by_cases hc : c = ':'
    · subst hc
      simp [bucket]
    · obtain ⟨h1, h2, h3, h4⟩ := ih
      simp only [bucket, hc, ite_false]
      refine ⟨List.cons_prefix_cons.mpr ⟨rfl, h1⟩, ?_, ?_, ?_⟩
      · simp [hc]
        exact h2
      · intro hm
        have hm' : ':' ∈ cs := by
          rcases List.mem_cons.mp hm with h | h
          · exact absurd h.symm hc
          · exact h
        have hl := h3 hm'
        cases hb : bucket cs with
        | nil => rw [hb] at hl; simp at hl
        | cons y ys => rw [hb] at hl; simp [List.getLast?_cons_cons, hl]
      · intro hm
        have hm' : ':' ∉ cs := fun h => hm (List.mem_cons_of_mem c h)
        rw [h4 hm']

theorem best_day_is_the_latest_maximum : LatestMaximum bestDay := by
  intro ts hp
  cases ts with
  | nil => simp [bestDay]
  | cons t ts =>
    rw [List.pairwise_cons] at hp
    obtain ⟨m, h1, h2, h3, h4⟩ := bestFrom_spec t ts hp.2 hp.1
    refine ⟨by simp [bestDay], fun b hb => ?_⟩
    simp only [bestDay, Option.some.injEq] at hb
    subst hb
    refine ⟨?_, ?_, ?_⟩
    · rcases m with m | m
      · rw [m]; simp
      · simp [m]
    · intro u hu
      rcases List.mem_cons.mp hu with rfl | hu
      · exact h1
      · exact h2 u hu
    · intro u hu hs
      rcases List.mem_cons.mp hu with rfl | hu
      · exact h3 hs
      · exact h4 u hu hs

theorem window_spans_the_capped_days : SpansCappedDays exchangeWindow := by
  intro days today
  refine ⟨fun h => by simp [exchangeWindow, h], fun h => ?_⟩
  refine ⟨today - (min days windowCap - 1), ?_, ?_⟩
  · simp [exchangeWindow]
    omega
  · simp only [windowCap]
    omega

/-! ## The witnesses: each wrong variant violates its claim -/

theorem rows_without_a_rollup_dropped_violates : ¬ KeepsEveryRow exchangeRatesJoined := by
  intro h
  have := h [⟨5, ['a'], 10⟩] []
  revert this
  decide

theorem per_row_denominator_violates : ¬ DayOncePerBucket exchangeRatesPerRow := by
  intro h
  have := h [⟨1, ['a'], 1⟩] [(1, 2)] ⟨1, ['a'], 1⟩ ⟨⟨1, ['a'], 1⟩, by simp, rfl, rfl⟩
  revert this
  decide

theorem defined_on_xp_violates : ¬ DefinedIffGraduation exchangeRatesOnXp := by
  intro h
  have := h [⟨1, ['a'], 5⟩] [] ⟨['a'], 5, 0, true⟩ (by decide)
  simp at this

theorem last_colon_bucket_violates : ¬ EndsAtFirstColon bucketLast := by
  intro h
  have := (h ['a', ':', 'b', ':']).2.1
  revert this
  decide

theorem oldest_on_tie_violates : ¬ LatestMaximum bestDayOnTie := by
  intro h
  have := ((h [⟨2, 5⟩, ⟨1, 5⟩] (by decide)).2 ⟨1, 5⟩ rfl).2.2 ⟨2, 5⟩ (by simp) rfl
  simp at this

theorem uncapped_window_violates : ¬ SpansCappedDays exchangeWindowUncapped := by
  intro h
  obtain ⟨first, h1, h2⟩ := (h 3651 0).2 (by decide)
  simp [exchangeWindowUncapped] at h1
  omega

end Formal.Exchange
