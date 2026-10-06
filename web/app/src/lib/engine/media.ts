// SPEC-350 R14, ADR-361 D12: the Worker's reader of the media directory.

/** A directory as the reader needs it. */
export interface MediaFolder {
  getDirectoryHandle(name: string): Promise<MediaFolder>;
  getFileHandle(name: string): Promise<{ getFile(): Promise<Blob> }>;
}

export async function readMedia(..._args: unknown[]): Promise<unknown[]> {
  return [];
}
