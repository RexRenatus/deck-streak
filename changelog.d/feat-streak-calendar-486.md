### Added

- The streak screen draws a calendar of the last 26 weeks for each track, as whole weeks from Monday, the window the predecessor serves, with a freeze, skip or break marker on the day it belongs to. `GET /api/streak` serves the days: each carries its date, whether it was a study day and its markers. A freeze sits on the missed day it covered, never on the day it was spent; a break sits on the day the run's own replay records it broken; a skip sits on a declared skip day. The law track serves skip and break markers.
