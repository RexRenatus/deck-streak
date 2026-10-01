-- @phx cites #74
-- @phx theorem probe_ok ramp=report
-- @phx witness probe_bad kills=probe_ok
theorem probe_ok : 1 + 1 = 2 := by decide
theorem probe_bad : ¬ (1 + 1 = 3) := by decide
