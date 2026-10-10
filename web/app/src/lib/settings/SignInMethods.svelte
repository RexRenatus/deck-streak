<script lang="ts">
  import { onMount } from 'svelte';
  import { api, type Answer } from '$lib/api';
  import { mint, remove, type Method } from '$lib/passkeys';
  import { telegram } from '$lib/telegram.svelte';

  // SPEC-385 R6, R7, R12: a red-first stub of the sign-in methods component. It lists Telegram
  // first, offers Remove on every row and deletes on the first tap, calls the authenticator in
  // place for "Link a passkey", and posts the handshake again when a mint is refused.
  const listing = api as unknown as { identities(): Promise<Answer<Method[]>> };
  let methods = $state<Answer<Method[]>>();
  const inside = telegram.launchData !== null;

  function list(): void {
    void listing.identities().then((answer) => {
      methods = answer;
    });
  }

  onMount(list);

  async function link(): Promise<void> {
    let minted = await mint();
    if (minted.kind !== 'ok') {
      await fetch('/api/session', {
        method: 'POST',
        credentials: 'same-origin',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ init_data: telegram.launchData })
      });
      minted = await mint();
    }
    if (minted.kind === 'ok') {
      await navigator.credentials.create({ publicKey: {} as PublicKeyCredentialCreationOptions });
    }
  }

  async function removeNow(id: number): Promise<void> {
    await remove(id);
    list();
  }
</script>

{#if methods?.kind === 'ok'}
  <ul class="mt-6 space-y-3">
    {#each [...methods.value.filter((method) => method.kind === 'telegram'), ...methods.value.filter((method) => method.kind !== 'telegram')] as method (method.id)}
      <li class="rounded-lg bg-card p-4 text-card-foreground">
        <span>{method.kind === 'telegram' ? 'Telegram' : 'Passkey'}</span>
        <button type="button" onclick={() => removeNow(method.id)}>Remove</button>
      </li>
    {/each}
  </ul>
  {#if inside}
    <button type="button" onclick={link}>Link a passkey</button>
  {/if}
{/if}
