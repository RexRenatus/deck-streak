### Added

- A law drill is answered once through the vault contract, from the bot or the app: the answer is appended under its heading in the drill note, a second answer is refused, and a drill that already holds an answer stays answered.
- A graded drill pays its post-back XP once, on the study day of the poll that finds it, and an interrupted poll is completed by the next one without a second pay.
- `/drills` and `/drill` in the bot list, filter and answer drills, and the owner's drill routes in the api list, show and answer them.
- The drill answer and grade records are exported and erased with the rest of a learner's data; an erase never deletes a note.
