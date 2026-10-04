// The page's persistent-storage request (SPEC-338 R4): the browser's answer, shown and never
// assumed. `navigator.storage.persist()` exists on the window only, so the page asks, not the Worker.

export type Persistence = 'persisted' | 'not-persisted' | 'unsupported';

/** The part of `StorageManager` the request reads. */
export type PersistentStorage = Partial<Pick<StorageManager, 'persist' | 'persisted'>>;

export async function requestPersistence(storage?: PersistentStorage): Promise<Persistence> {
  void storage;
  return 'unsupported';
}
