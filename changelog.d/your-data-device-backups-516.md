### Added

- A data screen on the web and on iPhone and iPad: the web screen downloads a JSON copy of your
  data, the same document the bot's `/export` sends, under your signed-in session; both screens
  back up this device's collection now, list this device's backups and export one; and both show
  the server's backup age, or say it is unknown. Inside Telegram on a phone the web screen names
  `/export` in case Telegram does not save the download. The iPhone and iPad app now keeps at
  most three backups of each kind, by the same rule as the web (SPEC-516, ADR-534).
