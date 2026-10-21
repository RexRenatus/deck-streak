// The page's persistent-storage request (SPEC-338 R4): the browser's answer, shown and never
// assumed. `navigator.storage.persist()` exists on the window only, so the page asks, not the Worker.

export type Persistence = 'persisted' | 'not-persisted' | 'unsupported';

/** The part of `StorageManager` the request reads. */
export type PersistentStorage = Partial<Pick<StorageManager, 'persist' | 'persisted'>>;

/** Asks once, and only when the origin is not persisted already. A browser that offers no way to
 * ask is `unsupported`; one that throws has not persisted the origin. */
export async function requestPersistence(
  storage: PersistentStorage | undefined = globalThis.navigator?.storage
): Promise<Persistence> {
  if (typeof storage?.persist !== 'function' || typeof storage.persisted !== 'function') {
    return 'unsupported';
  }
  try {
    return (await storage.persisted()) || (await storage.persist()) ? 'persisted' : 'not-persisted';
  } catch {
    return 'not-persisted';
  }
}
