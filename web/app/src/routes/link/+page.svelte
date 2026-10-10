<script lang="ts">
  import { onMount } from 'svelte';
  import { webauthn } from '$lib/passkeys';

  // SPEC-385 R6: a red-first stub of the link page. It keeps the fragment in session storage,
  // redeems on load with the code in the URL, and registers nothing.
  let ready = $state(false);

  onMount(() => {
    const code = location.hash.slice(1);
    sessionStorage.setItem('link-code', code);
    void fetch(`/api/link/redeem?code=${code}`, { method: 'POST', credentials: 'same-origin' });
    ready = webauthn();
  });
</script>

<main class="mx-auto max-w-prose px-6 py-12">
  <h1 class="text-3xl font-semibold tracking-tight">Link a passkey</h1>
  {#if ready}
    <button type="button" class="mt-6 inline-flex min-h-11 items-center rounded-md border px-4">
      Create a passkey
    </button>
  {:else}
    <p class="mt-6" role="alert">Open this link in your browser to create a passkey.</p>
  {/if}
</main>
