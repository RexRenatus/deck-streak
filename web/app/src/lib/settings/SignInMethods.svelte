<script lang="ts">
  import { onMount } from 'svelte';
  import { api, type Answer } from '$lib/api';
  import { m } from '$lib/paraglide/messages.js';
  import { getLocale } from '$lib/paraglide/runtime.js';
  import { mint, remove, wordingText, type Method, type Wording } from '$lib/passkeys';
  import { telegram } from '$lib/telegram.svelte';

  /**
   * The owner's sign-in methods (SPEC-385 R6, R7, R10, R12; ADR-399 D2, D3), as
   * `GET /api/identities` lists them: Telegram first and never removable, then each passkey with
   * the day it was added and a removal behind a confirm step. Inside Telegram, "Link a passkey"
   * mints a code and opens the link page in the browser through the Mini App's link opener; no
   * ceremony runs in Telegram's frame. A mint and a removal are each sent once, and a refusal is
   * shown, never retried. The settings screen embeds this component later (#57).
   */
  let listed = $state<Answer<Method[]>>();
  let confirming = $state<number | null>(null);
  let message = $state<Wording | null>(null);
  let working = $state(false);

  /** Telegram first, then each passkey in the order the list holds them. */
  function ordered(value: readonly Method[]): Method[] {
    return [
      ...value.filter((method) => method.kind === 'telegram'),
      ...value.filter((method) => method.kind === 'passkey')
    ];
  }

  async function list(): Promise<void> {
    listed = await api.identities();
  }

  onMount(() => {
    void list();
  });

  /** The day a passkey was added, in the page's language. */
  function added(createdAt: number): string {
    const format = new Intl.DateTimeFormat(getLocale(), { dateStyle: 'medium' });
    return m.methods_added({ date: format.format(createdAt) });
  }

  /** Mints a code, once, and opens the link page in the browser with it. */
  async function link(): Promise<void> {
    working = true;
    message = null;
    const minted = await mint();
    working = false;
    if (minted.kind === 'ok') telegram.openLink(`${location.origin}/link#${minted.value}`);
    else message = minted.wording;
  }

  /** Removes the passkey `id`, once; a passkey already gone is read away with the list. */
  async function removeNow(id: number): Promise<void> {
    working = true;
    message = null;
    const removed = await remove(id);
    working = false;
    confirming = null;
    if (removed.kind === 'refused' && removed.wording !== null) message = removed.wording;
    else await list();
  }
</script>

{#if listed === undefined}
  <p class="mt-6" role="status">{m.loading()}</p>
{:else if listed.kind === 'reopen'}
  <p class="mt-6" role="alert">{m.reopen_from_telegram()}</p>
{:else if listed.kind === 'unavailable'}
  <p class="mt-6" role="alert">{m.server_unavailable()}</p>
{:else}
  <ul class="mt-6 space-y-3">
    {#each ordered(listed.value) as method (method.id)}
      <li class="rounded-lg bg-card p-4 text-card-foreground">
        {#if method.kind === 'telegram'}
          <p>{m.methods_telegram()}</p>
        {:else}
          <p>{m.methods_passkey()}</p>
          <p class="text-sm">{added(method.createdAt)}</p>
          {#if confirming === method.id}
            <p class="mt-3">{m.methods_remove_confirm()}</p>
            <div class="mt-3 flex gap-3">
              <button
                type="button"
                class="inline-flex min-h-11 items-center rounded-md border px-4"
                disabled={working}
                onclick={() => removeNow(method.id)}
              >
                {m.methods_remove()}
              </button>
              <button
                type="button"
                class="inline-flex min-h-11 items-center rounded-md border px-4"
                disabled={working}
                onclick={() => (confirming = null)}
              >
                {m.methods_remove_keep()}
              </button>
            </div>
          {:else}
            <button
              type="button"
              class="mt-3 inline-flex min-h-11 items-center rounded-md border px-4"
              disabled={working}
              onclick={() => (confirming = method.id)}
            >
              {m.methods_remove()}
            </button>
          {/if}
        {/if}
      </li>
    {/each}
  </ul>
{/if}

{#if message !== null}
  <p class="mt-6" role="alert">{wordingText(message)}</p>
{/if}

{#if telegram.launchData !== null}
  <button
    type="button"
    class="mt-6 inline-flex min-h-11 items-center rounded-md border px-4"
    disabled={working}
    onclick={link}
  >
    {m.methods_link_passkey()}
  </button>
{/if}
