import { m } from './paraglide/messages.js';

/**
 * The web client's passkey ceremonies (SPEC-385 R1 to R8, R11; ADR-399 D1, D2).
 *
 * The browser's own credential API, through a codec this module owns: the server's options carry
 * every byte field as unpadded base64url, and the authenticator's answer goes back in the same
 * encoding, in the JSON shape the server's own tests post. Every call is a same-origin JSON post
 * to a relative path, sent once: a refusal is shown, never retried, and a ceremony's 401 is never
 * a reason to open a session again. The client keeps nothing: no storage write and no log line
 * carries a code, an id, a handle or a response. The flow id and the session are HttpOnly cookies
 * this module never reads.
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

/** Each reason code the server answers, with the wording it meets (SPEC-385 §2a). */
const REFUSALS: Readonly<Record<string, Wording | null>> = {
  not_linked: 'signin_not_linked',
  challenge_invalid: 'passkey_start_again',
  challenge_expired: 'passkey_start_again',
  origin_mismatch: 'passkey_refused',
  uv_required: 'passkey_refused',
  passkey_invalid: 'passkey_refused',
  counter_regressed: 'passkey_refused',
  not_owner: 'passkey_refused',
  too_many_ceremonies: 'passkey_too_many',
  linking_off: 'passkey_off',
  cross_site_request: 'passkey_refused',
  not_json: 'passkey_refused',
  link_code_invalid: 'link_code_spent',
  link_code_expired: 'link_code_spent',
  no_session: 'link_code_spent',
  already_linked: 'link_already',
  reauth_required: 'methods_reopen',
  identity_unknown: null
};

/**
 * The wording an answer of `status` naming `reason` meets: a code of §2a its own, a code no row
 * names the refusal wording, and an answer that names no code, or a server that failed, the
 * screen's unavailable wording.
 */
export function wordingOf(status: number, reason: unknown): Wording | null {
  if (status >= 500 || typeof reason !== 'string') return 'server_unavailable';
  return Object.hasOwn(REFUSALS, reason) ? REFUSALS[reason] : 'passkey_refused';
}

/** The wording an authenticator's refusal meets: the owner's cancel offers a new start. */
export function wordingOfThrown(error: unknown): Wording {
  const name = (error as { readonly name?: unknown } | null | undefined)?.name;
  return name === 'NotAllowedError' ? 'passkey_cancelled' : 'passkey_refused';
}

/** The text `wording` shows, in the page's language. */
export function wordingText(wording: Wording): string {
  return m[wording]();
}

/** Bytes as unpadded base64url. */
export function encode(bytes: ArrayBuffer): string {
  let binary = '';
  for (const byte of new Uint8Array(bytes)) binary += String.fromCharCode(byte);
  return btoa(binary).replaceAll('+', '-').replaceAll('/', '_').replaceAll('=', '');
}

/** Unpadded base64url as bytes. */
export function decode(text: string): Uint8Array<ArrayBuffer> {
  const binary = atob(text.replaceAll('-', '+').replaceAll('_', '/'));
  return Uint8Array.from(binary, (char) => char.charCodeAt(0));
}

/** The `publicKey` options of a start's answer. */
function publicKeyOf(answer: unknown): Json | undefined {
  return (answer as { readonly publicKey?: Json } | null)?.publicKey;
}

/** A list of credential descriptors, each id as bytes; an absent list stays absent. */
function descriptors(list: unknown): unknown {
  return Array.isArray(list)
    ? list.map((item: Json) => ({ ...item, id: decode(String(item.id)) }))
    : list;
}

/**
 * The registration options a start answered, as the browser takes them; null unless they name
 * this page's host as the relying party, so other options never reach the authenticator (R4).
 */
export function creationOptions(
  answer: unknown,
  host: string
): PublicKeyCredentialCreationOptions | null {
  const options = publicKeyOf(answer);
  if ((options?.rp as Json | undefined)?.id !== host) return null;
  const user = options?.user as Json;
  return {
    ...options,
    challenge: decode(String(options?.challenge)),
    user: { ...user, id: decode(String(user.id)) },
    excludeCredentials: descriptors(options?.excludeCredentials)
  } as unknown as PublicKeyCredentialCreationOptions;
}

/** The sign-in options a start answered, as the browser takes them; null unless `rpId` is `host`. */
export function requestOptions(
  answer: unknown,
  host: string
): PublicKeyCredentialRequestOptions | null {
  const options = publicKeyOf(answer);
  if (options?.rpId !== host) return null;
  return {
    ...options,
    challenge: decode(String(options.challenge)),
    allowCredentials: descriptors(options.allowCredentials)
  } as unknown as PublicKeyCredentialRequestOptions;
}

/** A new credential as the registration finish posts it, with `extensions` as `{}`. */
export function registrationJSON(credential: PublicKeyCredential): Json {
  const response = credential.response as AuthenticatorAttestationResponse;
  return {
    id: credential.id,
    rawId: encode(credential.rawId),
    type: credential.type,
    response: {
      attestationObject: encode(response.attestationObject),
      clientDataJSON: encode(response.clientDataJSON)
    },
    extensions: {}
  };
}

/** An assertion as the sign-in finish posts it, with `extensions` as `{}`. */
export function assertionJSON(credential: PublicKeyCredential): Json {
  const response = credential.response as AuthenticatorAssertionResponse;
  return {
    id: credential.id,
    rawId: encode(credential.rawId),
    type: credential.type,
    response: {
      authenticatorData: encode(response.authenticatorData),
      clientDataJSON: encode(response.clientDataJSON),
      signature: encode(response.signature),
      userHandle: response.userHandle === null ? null : encode(response.userHandle)
    },
    extensions: {}
  };
}

/** The body of `GET /api/identities`, Telegram and each passkey, or null when it is not one. */
export function parseMethods(body: unknown): Method[] | null {
  if (!Array.isArray(body)) return null;
  const methods: Method[] = [];
  for (const item of body as (Json | null)[]) {
    const id = item?.id;
    const created = item?.created_at;
    if (typeof id !== 'number') return null;
    if (item?.kind === 'telegram') methods.push({ id, kind: 'telegram' });
    else if (item?.kind === 'passkey' && typeof created === 'number') {
      methods.push({ id, kind: 'passkey', createdAt: created });
    } else return null;
  }
  return methods;
}

/** Whether this browser offers WebAuthn, the credential API every ceremony calls. */
export function webauthn(): boolean {
  return typeof globalThis.PublicKeyCredential === 'function' && navigator.credentials !== undefined;
}

/** A refusal meeting `wording`. */
function refused(wording: Wording | null): Outcome<never> {
  return { kind: 'refused', wording };
}

/** A same-origin JSON request of `body` to the relative `path`, sent once; null when no answer came. */
async function send(path: string, body: unknown, method = 'POST'): Promise<Response | null> {
  try {
    return await fetch(path, {
      method,
      credentials: 'same-origin',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify(body)
    });
  } catch {
    return null;
  }
}

/** How `response` ended: the value `read` takes from its body, or the wording its refusal meets. */
async function ended<T>(
  response: Response | null,
  read: (body: unknown) => T | null
): Promise<Outcome<T>> {
  if (response === null) return refused('server_unavailable');
  const body: unknown = await response.json().catch(() => null);
  if (!response.ok) return refused(wordingOf(response.status, (body as Json | null)?.reason));
  const value = read(body);
  return value === null ? refused('passkey_refused') : { kind: 'ok', value };
}

/** A finish's answer, which carries nothing the page shows. */
const done = (): true => true;

/** Redeems a link code in a JSON body (R6); the server opens a `link` session. */
export async function redeem(code: string): Promise<Outcome<true>> {
  return ended(await send('/api/link/redeem', { code }), done);
}

/** Registers a passkey inside the `link` session: one start, one create, one finish (R1, R4, R5). */
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
    return refused(wordingOfThrown(error));
  }
  return ended(await send('/api/passkeys/register/finish', response), done);
}

/** The signal a browser may offer, telling its authenticator which credentials the owner holds. */
interface Signals {
  readonly signalAllAcceptedCredentials?: (options: {
    readonly rpId: string;
    readonly userId: unknown;
    readonly allAcceptedCredentialIds: unknown;
  }) => Promise<void>;
}

/**
 * Signs in with one of the owner's passkeys: one start, one get, one finish (R1, R4, R5, R8).
 * After it, the accepted credentials are signalled with the request's relying party, only where
 * the browser offers the signal.
 */
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
    return refused(wordingOfThrown(error));
  }
  const finished = await ended(await send('/api/passkeys/sign-in/finish', response), (body) => body);
  if (finished.kind !== 'ok') return finished;
  const signals = PublicKeyCredential as unknown as Signals;
  if (typeof signals.signalAllAcceptedCredentials === 'function') {
    const accepted = finished.value as Json;
    try {
      await signals.signalAllAcceptedCredentials({
        rpId: String(started.value.rpId),
        userId: accepted.user_handle,
        allAcceptedCredentialIds: accepted.credential_ids
      });
    } catch {
      // a signal the browser refuses changes nothing: the owner is signed in
    }
  }
  return { kind: 'ok', value: true };
}

/** Mints a link code from a fresh Telegram session, sent once (R6, R7). */
export async function mint(): Promise<Outcome<string>> {
  return ended(await send('/api/link/code', {}), (body) => {
    const code = (body as Json | null)?.code;
    return typeof code === 'string' ? code : null;
  });
}

/** Removes the passkey `id`, sent once (R7, R12). */
export async function remove(id: number): Promise<Outcome<true>> {
  return ended(await send(`/api/identities/${id}`, {}, 'DELETE'), done);
}
