/**
 * The quick capture (SPEC-118 R10, R11; ADR-118): what the Mini App sends to
 * `POST /api/inbox/captures`, and how it reads the answer.
 *
 * A capture carries its own retry key, `capture_id`. The server writes a capture once per key, so
 * a capture sent again after a lost answer is answered with the name it was first saved under,
 * even after UTC midnight (ADR-118's capture-key amendment). The page therefore mints a fresh key
 * for each new capture and sends the same key when it retries one.
 */

/** Where a quick capture is posted. */
export const CAPTURE_PATH = '/api/inbox/captures';

/** The text's bound after trimming, in characters: the server's `QUICK_TEXT_CHARS`. */
export const QUICK_TEXT_CHARS = 4000;

/** A quick capture's kind: a plain note, or a journal line the curator files as one. */
export type CaptureKind = 'text' | 'journal';

/** One quick capture as the screen sends it. */
export interface CaptureRequest {
  /** The retry key: 32 lowercase hex characters, fresh for each capture. */
  readonly captureId: string;
  readonly kind: CaptureKind;
  readonly text: string;
}

/** The server's answer to a capture it holds. */
export interface Saved {
  /** The stub's file name in the vault's inbox. */
  readonly name: string;
  /** True when this key was already saved, so nothing new was written. */
  readonly alreadyCaptured: boolean;
}

/** A fresh retry key: a random UUID's 32 hex digits, which the server keeps as they are. */
export function newCaptureId(): string {
  return crypto.randomUUID().replaceAll('-', '');
}

/**
 * Whether `text` is within the bound as the server counts it: 1 to 4000 characters after the
 * trim, counted as characters, so a character outside the basic plane counts once.
 */
export function fits(text: string): boolean {
  const characters = [...text.trim()].length;
  return characters >= 1 && characters <= QUICK_TEXT_CHARS;
}

/** The request's JSON body, in the route's own names. */
export function captureBody(request: CaptureRequest): string {
  return JSON.stringify({
    capture_id: request.captureId,
    kind: request.kind,
    text: request.text
  });
}

/**
 * The capture a response of `status` with `body` names, or null when it names none: 201 with the
 * new stub's name, or 200 with the name an earlier send of the same key was saved under.
 */
export function parseSaved(status: number, body: unknown): Saved | null {
  if (body === null || typeof body !== 'object' || Array.isArray(body)) return null;
  const fields = body as Record<string, unknown>;
  const name = fields.name;
  if (typeof name !== 'string' || name === '') return null;
  if (status === 201) return { name, alreadyCaptured: false };
  if (status === 200 && fields.already_captured === true) return { name, alreadyCaptured: true };
  return null;
}
