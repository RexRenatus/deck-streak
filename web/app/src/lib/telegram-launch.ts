/**
 * The launch gate (SPEC-400; ADR-414), as a stub that keeps its inputs and admits nothing, so the
 * unit checks are red by their assertions before the gate exists.
 */

/** Whether Telegram launched the page, read from `hash` and the tab's `storage`. The stub reads neither. */
export function isLaunch(hash: string, storage: Pick<Storage, 'getItem'> | undefined): boolean {
  void hash;
  void storage;
  return false;
}

/** Admits the launch into `host` before the router's first navigation. The stub does nothing. */
export function admitLaunch(host: Window): Promise<void> {
  void host;
  return Promise.resolve();
}
