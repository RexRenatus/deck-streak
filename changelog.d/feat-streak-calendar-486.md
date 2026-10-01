### Added

- The streak screen draws a five-week calendar for each track, with a freeze, skip or break marker on the day it belongs to. `GET /api/streak` serves the days: each carries its date, whether it was a study day and its markers. A freeze sits on the missed day it covered, never on the day it was spent; a break sits on the day the run broke; a skip sits on a declared skip day. The law track serves skip and break markers.
