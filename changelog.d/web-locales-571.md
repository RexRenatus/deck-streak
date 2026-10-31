### Added

- Every message of the web app now exists in all seven locales: the messages of the about, score,
  level, streak, insights, dark fields, quick capture and remote screens that showed in English
  inside the Spanish, French, Japanese, Korean, Simplified Chinese and Traditional Chinese screens
  are written in each of those languages (#1051).
- A census of the web app's message catalogs refuses, by name, a message missing from a locale, a
  blank value, a placeholder that differs from English's, a message English lacks and a value out
  of Unicode NFC, so a later change that adds an English message without its translations is
  refused where it is made (#1051).
