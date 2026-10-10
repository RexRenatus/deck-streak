/**
 * The web client's passkey ceremonies (SPEC-385 R1 to R8, R11; ADR-399 D1, D2): a red-first stub.
 * It keeps every input of the ceremony client and none of its behaviour: it converts nothing,
 * checks no relying party, never signals, posts with no content type and maps no refusal.
 */

/** The wording a refusal meets (SPEC-385 §2a): a message key, or the screen's unavailable one. */
export type Wording =
  | 'signin_not_linked'
  | 'passkey_refused'
  | 'passkey_start_again'
  | 'passkey_too_many'
  | 'passkey_off'
  | 'passkey_cancelled'
  | 'link_code_spent'
  | 'link_already'
  | 'methods_reopen'
  | 'server_unavailable';

/** How a ceremony call ended: its value, or the wording its refusal meets (null: read the list again). */
export type Outcome<T> =
  | { readonly kind: 'ok'; readonly value: T }
  | { readonly kind: 'refused'; readonly wording: Wording | null };

/** One of the owner's ways in, as `GET /api/identities` lists it (SPEC-385 R12). */
export type Method =
  | { readonly id: number; readonly kind: 'telegram' }
  | { readonly id: number; readonly kind: 'passkey'; readonly createdAt: number };

type Json = Record<string, unknown>;

/** The stub maps no refusal. */
export function wordingOf(status: number, reason: unknown): Wording | null {
  void status;
  void reason;
  return null;
}

/** The stub maps no refusal. */
export function wordingOfThrown(error: unknown): Wording {
  void error;
  return null as unknown as Wording;
}

/** The stub converts nothing. */
export function encode(bytes: ArrayBuffer): string {
  return bytes as unknown as string;
}

/** The stub converts nothing. */
export function decode(text: string): Uint8Array<ArrayBuffer> {
  return text as unknown as Uint8Array<ArrayBuffer>;
}

/** The stub checks no relying party and converts nothing. */
export function creationOptions(
  answer: unknown,
  host: string
): PublicKeyCredentialCreationOptions | null {
  void host;
  return (answer as { publicKey: PublicKeyCredentialCreationOptions }).publicKey;
}

/** The stub checks no relying party and converts nothing. */
export function requestOptions(
  answer: unknown,
  host: string
): PublicKeyCredentialRequestOptions | null {
  void host;
  return (answer as { publicKey: PublicKeyCredentialRequestOptions }).publicKey;
}

/** The stub converts nothing. */
export function registrationJSON(credential: PublicKeyCredential): Json {
  return { ...credential, extensions: {} };
}

/** The stub converts nothing. */
export function assertionJSON(credential: PublicKeyCredential): Json {
  return { ...credential, extensions: {} };
}

/** Whether this browser offers WebAuthn, the credential API every ceremony calls. */
export function webauthn(): boolean {
  return typeof globalThis.PublicKeyCredential === 'function' && navigator.credentials !== undefined;
}

/** The stub posts with no content type. */
async function send(path: string, body: unknown, method = 'POST'): Promise<Response | null> {
  try {
    return await fetch(path, { method, credentials: 'same-origin', body: JSON.stringify(body) });
  } catch {
    return null;
  }
}

async function ended<T>(
  response: Response | null,
  read: (body: unknown) => T | null
): Promise<Outcome<T>> {
  if (response === null) return { kind: 'refused', wording: null };
  const body: unknown = await response.json().catch(() => null);
  if (!response.ok) {
    return { kind: 'refused', wording: wordingOf(response.status, (body as Json | null)?.reason) };
  }
  return { kind: 'ok', value: read(body) as T };
}

const done = (): true => true;

/** Redeems a link code. */
export async function redeem(code: string): Promise<Outcome<true>> {
  return ended(await send('/api/link/redeem', { code }), done);
}

/** Registers a passkey, checking no relying party. */
export async function register(): Promise<Outcome<true>> {
  const started = await ended(await send('/api/passkeys/register/start', {}), (answer) =>
    creationOptions(answer, location.hostname)
  );
  if (started.kind !== 'ok') return started;
  let response: Json;
  try {
    const created = await navigator.credentials.create({ publicKey: started.value });
    response = registrationJSON(created as PublicKeyCredential);
  } catch (error) {
    return { kind: 'refused', wording: wordingOfThrown(error) };
  }
  return ended(await send('/api/passkeys/register/finish', response), done);
}

/** Signs in with a passkey, and never signals. */
export async function signIn(): Promise<Outcome<true>> {
  const started = await ended(await send('/api/passkeys/sign-in/start', {}), (answer) =>
    requestOptions(answer, location.hostname)
  );
  if (started.kind !== 'ok') return started;
  let response: Json;
  try {
    const asserted = await navigator.credentials.get({ publicKey: started.value });
    response = assertionJSON(asserted as PublicKeyCredential);
  } catch (error) {
    return { kind: 'refused', wording: wordingOfThrown(error) };
  }
  const finished = await ended(await send('/api/passkeys/sign-in/finish', response), done);
  if (finished.kind !== 'ok') return finished;
  return { kind: 'ok', value: true };
}

/** Mints a link code. */
export async function mint(): Promise<Outcome<string>> {
  return ended(await send('/api/link/code', {}), (body) => String((body as Json | null)?.code));
}

/** Removes a passkey. */
export async function remove(id: number): Promise<Outcome<true>> {
  return ended(await send(`/api/identities/${id}`, {}, 'DELETE'), done);
}
