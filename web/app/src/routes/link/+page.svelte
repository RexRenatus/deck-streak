<script lang="ts">
  import { onMount } from 'svelte';
  import { replaceState } from '$app/navigation';
  import { m } from '$lib/paraglide/messages.js';
  import { redeem, register, webauthn, wordingText, type Wording } from '$lib/passkeys';

  /**
   * The link page (SPEC-385 R3, R6; ADR-399 D2, D3). The methods screen opens it in the browser
   * with a link code in its fragment. On the owner's tap, and only where the browser offers
   * WebAuthn, it redeems the code in a JSON body, clears the fragment once the redeem has answered,
   * and registers a passkey inside the link session the redeem opened. It keeps nothing: the code
   * lives in this page's memory until the page is left.
   */
  type Stage = 'reading' | 'no-code' | 'no-webauthn' | 'ready' | 'working' | 'spent' | 'done';

  let stage = $state<Stage>('reading');
  let message = $state<Wording | null>(null);
  let code = '';
  let redeemed = false;

  onMount(() => {
    code = location.hash.slice(1);
    if (code === '') stage = 'no-code';
    else stage = webauthn() ? 'ready' : 'no-webauthn';
  });

  /** A refused step: a spent code needs a new link, and anything else may start again. */
  function refused(wording: Wording | null): void {
    message = wording;
    stage = wording === 'link_code_spent' ? 'spent' : 'ready';
  }

  /** Redeems the code, once, then registers; a registration refused after it starts again unredeemed. */
  async function create(): Promise<void> {
    stage = 'working';
    message = null;
    if (!redeemed) {
      const answered = await redeem(code);
      // answered either way: the address no longer carries the code
      replaceState(location.pathname, {});
      if (answered.kind !== 'ok') return refused(answered.wording);
      redeemed = true;
    }
    const registered = await register();
    if (registered.kind !== 'ok') return refused(registered.wording);
    stage = 'done';
  }
</script>

<main class="mx-auto max-w-prose px-6 py-12">
  <h1 class="text-3xl font-semibold tracking-tight">{m.link_title()}</h1>
  {#if stage === 'no-code'}
    <p class="mt-6" role="alert">{m.link_no_code()}</p>
  {:else if stage === 'no-webauthn'}
    <p class="mt-6" role="alert">{m.link_open_in_browser()}</p>
  {:else if stage === 'spent'}
    <p class="mt-6" role="alert">{m.link_code_spent()}</p>
  {:else if stage === 'done'}
    <p class="mt-6" role="status">{m.link_done()}</p>
  {:else if stage !== 'reading'}
    {#if message !== null}
      <p class="mt-6" role="alert">{wordingText(message)}</p>
    {/if}
    <button
      type="button"
      class="mt-6 inline-flex min-h-11 items-center rounded-md border px-4"
      disabled={stage !== 'ready'}
      onclick={create}
    >
      {m.link_create()}
    </button>
  {/if}
</main>
