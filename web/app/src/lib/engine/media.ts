// SPEC-350 R14, ADR-361 D12: the Worker's reader of the media directory. It reads what the core
// asks for and no further; every cap, type and order decision stays the core's.
import type { MediaAsk } from './protocol';
import type { MediaFile } from './session';

/** A directory as the reader needs it. */
export interface MediaFolder {
  getDirectoryHandle(name: string): Promise<MediaFolder>;
  getFileHandle(name: string): Promise<{ getFile(): Promise<Blob> }>;
}

/** The storage as the reader needs it: the origin's root directory, when the browser has one. */
export interface MediaStorage {
  getDirectory?(): Promise<MediaFolder>;
}

/** The first `limit` bytes of the file `name` in `folder`, or null when the folder holds no such
 * file or refuses to read it: an absent file is absent, and the core omits it by name. */
async function readOne(folder: MediaFolder, { name, limit }: MediaAsk): Promise<MediaFile | null> {
  try {
    const file = await (await folder.getFileHandle(name)).getFile();
    return { name, bytes: new Uint8Array(await file.slice(0, Number(limit)).arrayBuffer()) };
  } catch {
    return null;
  }
}

/** Each file the core asked for that `directory`, under the storage's root, holds, read no further
 * than its limit, in the order asked. No storage, a storage that refuses, and no directory each
 * read as no file, never as an error: the directory is opened, never created. */
export async function readMedia(
  storage: MediaStorage | undefined,
  directory: string,
  wanted: readonly MediaAsk[]
): Promise<MediaFile[]> {
  const files: MediaFile[] = [];
  try {
    const root = await (storage as Required<MediaStorage>).getDirectory();
    const folder = await root.getDirectoryHandle(directory);
    for (const ask of wanted) {
      const file = await readOne(folder, ask);
      if (file !== null) files.push(file);
    }
  } catch {
    // no storage, a storage that refuses, or no media directory: there is no file to read
  }
  return files;
}
