/-!
# Formal.SyncOrder

The orders the sync entries share (SPEC-504, ADR-522): an operation's key, (hlc, origin, seq)
compared lexicographically, and an edit, a key with the value it writes, compared by key and then
by value. Each `lt` is a strict total order: irreflexive, asymmetric, transitive, and total on
distinct elements. This module carries no header line, so the registry lists it as support.
-/

namespace Formal.SyncOrder

/-- An operation's key: its hybrid clock, its origin device and its per-device sequence number. -/
structure Key where
  hlc : Nat
  origin : Nat
  seq : Nat
  deriving DecidableEq, Repr

/-- Key order: by clock, then origin, then sequence number. -/
def Key.lt (a b : Key) : Bool :=
  a.hlc < b.hlc || (a.hlc == b.hlc && (a.origin < b.origin || (a.origin == b.origin && a.seq < b.seq)))

theorem Key.lt_irrefl (a : Key) : Key.lt a a = false := by
  simp [Key.lt]

theorem Key.lt_asymm {a b : Key} (h : Key.lt a b = true) : Key.lt b a = false := by
  simp [Key.lt] at *
  omega

theorem Key.lt_trans {a b c : Key} (h1 : Key.lt a b = true) (h2 : Key.lt b c = true) :
    Key.lt a c = true := by
  simp [Key.lt] at *
  omega

theorem Key.lt_total {a b : Key} (h : a ≠ b) : Key.lt a b = true ∨ Key.lt b a = true := by
  cases a
  cases b
  simp [Key.lt] at *
  omega

/-- An edit: the key of the operation that wrote it and the value it writes. -/
structure Edit where
  hlc : Nat
  origin : Nat
  seq : Nat
  val : Nat
  deriving DecidableEq, Repr

/-- Edit order: by key, then by value, so two edits are equal exactly when neither is less. -/
def Edit.lt (a b : Edit) : Bool :=
  a.hlc < b.hlc || (a.hlc == b.hlc && (a.origin < b.origin || (a.origin == b.origin &&
    (a.seq < b.seq || (a.seq == b.seq && a.val < b.val)))))

theorem Edit.lt_irrefl (a : Edit) : Edit.lt a a = false := by
  simp [Edit.lt]

theorem Edit.lt_asymm {a b : Edit} (h : Edit.lt a b = true) : Edit.lt b a = false := by
  simp [Edit.lt] at *
  omega

theorem Edit.lt_trans {a b c : Edit} (h1 : Edit.lt a b = true) (h2 : Edit.lt b c = true) :
    Edit.lt a c = true := by
  simp [Edit.lt] at *
  omega

theorem Edit.lt_total {a b : Edit} (h : a ≠ b) : Edit.lt a b = true ∨ Edit.lt b a = true := by
  cases a
  cases b
  simp [Edit.lt] at *
  omega

end Formal.SyncOrder
