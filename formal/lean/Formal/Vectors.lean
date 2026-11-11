import Formal.AgentPinGroupsVectors
import Formal.ChestVectors
import Formal.CoinSharesVectors
import Formal.CompanionDecideVectors
import Formal.DeployEntryRefusalVectors
import Formal.DocsMermaidReadVectors
import Formal.DueReorderVectors
import Formal.ExchangeVectors
import Formal.Fsrs7MemoryStateVectors
import Formal.Fsrs7PortStateVectors
import Formal.Fsrs7ScheduleVectors
import Formal.Fsrs7StockVectors
import Formal.HabitWritingVectors
import Formal.IdSlotVectors
import Formal.MediaBindVectors
import Formal.MediaGcVectors
import Formal.NextMilestoneVectors
import Formal.OpenLapseVectors
import Formal.PathGateVectors
import Formal.RecordClassBindingVectors
import Formal.ReleaseCarryVectors
import Formal.ReleaseGatesVectors
import Formal.ReplayHistoryVectors
import Formal.RetiredListShapeVectors
import Formal.RoadToC2Vectors
import Formal.SensitiveDeckVectors
import Formal.SkipTariffVectors
import Formal.SyncVectors
import Formal.TableCensusReachVectors
import Formal.TokenBonusVectors
import Formal.TokenProgressVectors
import Formal.WalletVectors
import Formal.XpReconciliationVectors

/-!
# Formal.Vectors

The vector writer. Run from `formal/lean` as `lake env lean --run Formal/Vectors.lean <Entry>`, it
prints the entry's vectors as JSON lines on stdout: one header line naming the covered item and the
digest the entry recorded for it, then one line per input with the port's own answer.
The formal checker byte-compares the output with the committed file, and `--write-vectors` writes
it; the file is never edited by hand.

Each entry's writer lives in a support module of its own, and `main` gains a one-line arm for it,
so two entries' arms merge by keeping both. This module carries no header line, so the registry
lists it as support.
-/

def main (args : List String) : IO UInt32 := do
  match args with
  | ["AgentPinGroups"] => Formal.AgentPinGroupsVectors.run
  | ["Chest"] => Formal.ChestVectors.run
  | ["CoinShares"] => Formal.CoinSharesVectors.run
  | ["CompanionDecide"] => Formal.CompanionDecideVectors.run
  | ["DeployEntryRefusal"] => Formal.DeployEntryRefusalVectors.run
  | ["DocsMermaidRead"] => Formal.DocsMermaidReadVectors.run
  | ["DueReorder"] => Formal.DueReorderVectors.run
  | ["Exchange"] => Formal.ExchangeVectors.run
  | ["Fsrs7MemoryState"] => Formal.Fsrs7MemoryStateVectors.run
  | ["Fsrs7PortState"] => Formal.Fsrs7PortStateVectors.run
  | ["Fsrs7Schedule"] => Formal.Fsrs7ScheduleVectors.run
  | ["Fsrs7Stock"] => Formal.Fsrs7StockVectors.run
  | ["HabitWriting"] => Formal.HabitWritingVectors.run
  | ["IdSlot"] => Formal.IdSlotVectors.run
  | ["MediaBind"] => Formal.MediaBindVectors.run
  | ["MediaGc"] => Formal.MediaGcVectors.run
  | ["NextMilestone"] => Formal.NextMilestoneVectors.run
  | ["OpenLapse"] => Formal.OpenLapseVectors.run
  | ["PathGate"] => Formal.PathGateVectors.run
  | ["RecordClassBinding"] => Formal.RecordClassBindingVectors.run
  | ["ReleaseCarry"] => Formal.ReleaseCarryVectors.run
  | ["ReleaseGates"] => Formal.ReleaseGatesVectors.run
  | ["ReplayHistory"] => Formal.ReplayHistoryVectors.run
  | ["RetiredListShape"] => Formal.RetiredListShapeVectors.run
  | ["RoadToC2"] => Formal.RoadToC2Vectors.run
  | ["SensitiveDeck"] => Formal.SensitiveDeckVectors.run
  | ["SkipTariff"] => Formal.SkipTariffVectors.run
  | ["SyncClock"] => Formal.SyncVectors.runClock
  | ["ReviewKey"] => Formal.SyncVectors.runKey
  | ["FieldMerge"] => Formal.SyncVectors.runMerge
  | ["PushJudge"] => Formal.SyncVectors.runJudge
  | ["TableCensusReach"] => Formal.TableCensusReachVectors.run
  | ["TokenBonus"] => Formal.TokenBonusVectors.run
  | ["TokenProgress"] => Formal.TokenProgressVectors.run
  | ["Wallet"] => Formal.WalletVectors.run
  | ["XpReconciliation"] => Formal.XpReconciliationVectors.run
  | _ =>
    IO.eprintln s!"usage: lean --run Formal/Vectors.lean <Entry>; no vectors for {args}"
    return 2
