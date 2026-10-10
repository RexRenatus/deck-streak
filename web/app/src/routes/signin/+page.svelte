<script lang="ts">
  import { goto } from '$app/navigation';
  import { TODAY } from '$lib/routes';
  import { m } from '$lib/paraglide/messages.js';
  import { signIn, webauthn, wordingText, type Wording } from '$lib/passkeys';

  /**
   * The sign-in page (SPEC-385 R5, R8, R11; ADR-399 D2). Outside Telegram it offers a passkey where
   * the browser offers WebAuthn, and says to open a browser that can where it does not. A sign-in
   * opens Today at its fixed path; no target is read from the URL. A refusal is shown and offers a
   * new start: its finish is never posted again.
   */
  const offered = webauthn();
  let working = $state(false);
  let message = $state<Wording | null>(null);

  async function start(): Promise<void> {
    working = true;
    message = null;
    const outcome = await signIn();
    if (outcome.kind === 'ok') {
      await goto(TODAY);
      return;
    }
    message = outcome.wording;
    working = false;
  }
</script>

<main class="mx-auto max-w-prose px-6 py-12">
  <h1 class="text-3xl font-semibold tracking-tight">{m.signin_title()}</h1>
  {#if offered}
    {#if message !== null}
      <p class="mt-6" role="alert">{wordingText(message)}</p>
    {/if}
    <button
      type="button"
      class="mt-6 inline-flex min-h-11 items-center rounded-md border px-4"
      disabled={working}
      onclick={start}
    >
      {m.signin_with_passkey()}
    </button>
  {:else}
    <p class="mt-6" role="alert">{m.signin_no_webauthn()}</p>
  {/if}
</main>
