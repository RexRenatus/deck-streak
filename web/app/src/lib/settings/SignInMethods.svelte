<script lang="ts">
  import { onMount, tick } from 'svelte';
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
   *
   * The focus never rests on nothing (SPEC-463): a press of Remove moves it to Keep, inside a
   * confirm step that is an alert dialog named by its question; Keep and Escape give it back to
   * that row's Remove; a removal lands it on the Remove that took the row's place, else the one
   * before it, else the list. Each Remove is named for its passkey's place and day.
   */
  let listed = $state<Answer<Method[]>>();
  let confirming = $state<number | null>(null);
  let message = $state<Wording | null>(null);
  let working = $state(false);

  /** "Keep", which takes the focus while a step asks. */
  let keeper = $state<HTMLButtonElement>();
  /** The list, which takes the focus from script only, when no passkey is left. */
  let listing = $state<HTMLUListElement>();
  /** Each passkey row's Remove, by the passkey's id; empty while that row's step is open. */
  const removers: Record<number, HTMLButtonElement | null> = {};

  // while a step asks, the focus is on keeping the passkey
  $effect(() => keeper?.focus());

  /** Telegram first, then each passkey in the order the list holds them. */
  function ordered(value: readonly Method[]): Method[] {
    return [
      ...value.filter((method) => method.kind === 'telegram'),
      ...value.filter((method) => method.kind === 'passkey')
    ];
  }

  async function list(): Promise<Answer<Method[]>> {
    const answer = await api.identities();
    listed = answer;
    return answer;
  }

  onMount(() => {
    void list();
  });

  /** The passkeys, in the order the rows show them. */
  function passkeys(value: readonly Method[]): Method[] {
    return ordered(value).filter((method) => method.kind === 'passkey');
  }

  /** A passkey's 1-based place among the passkey rows as shown. */
  function place(value: readonly Method[], id: number): number {
    return passkeys(value).findIndex((method) => method.id === id) + 1;
  }

  /** The day a passkey was added, in the page's language. */
  function day(createdAt: number): string {
    return new Intl.DateTimeFormat(getLocale(), { dateStyle: 'medium' }).format(createdAt);
  }

  /** The line a row shows for the day a passkey was added. */
  function added(createdAt: number): string {
    return m.methods_added({ date: day(createdAt) });
  }

  /** Puts the question away, and gives the focus back to that row's Remove. */
  async function keep(id: number): Promise<void> {
    confirming = null;
    await tick();
    removers[id]!.focus();
  }

  /** Escape, with a step open, presses its Keep; a pending removal holds Keep, so it stays. */
  function escape(event: KeyboardEvent): void {
    if (event.key === 'Escape') keeper?.click();
  }

  /** Moves the focus to the Remove at `index` among the passkeys left (the last, when `index` is past them), else to the list. */
  function land(value: readonly Method[], index: number): void {
    const left = passkeys(value);
    if (left.length === 0) listing!.focus();
    else removers[left[Math.min(index, left.length - 1)].id]!.focus();
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

  /**
   * Removes the passkey `id`, once, which the rows show at `index` among the passkeys; a passkey
   * already gone is read away with the list. A refusal gives the focus back to its Remove; a
   * list read again lands it by `land`.
   */
  async function removeNow(id: number, index: number): Promise<void> {
    working = true;
    message = null;
    const removed = await remove(id);
    working = false;
    confirming = null;
    if (removed.kind === 'refused' && removed.wording !== null) {
      message = removed.wording;
      await tick();
      removers[id]!.focus();
    } else {
      const again = await list();
      await tick();
      if (again.kind === 'ok') land(again.value, index);
    }
  }
</script>

<svelte:window onkeydown={escape} />

{#if listed === undefined}
  <p class="mt-6" role="status">{m.loading()}</p>
{:else if listed.kind === 'reopen'}
  <p class="mt-6" role="alert">{m.reopen_from_telegram()}</p>
{:else if listed.kind === 'unavailable'}
  <p class="mt-6" role="alert">{m.server_unavailable()}</p>
{:else}
  <ul
    bind:this={listing}
    tabindex="-1"
    aria-label={m.methods_title()}
    class="mt-6 space-y-3"
  >
    {#each ordered(listed.value) as method (method.id)}
      <li class="rounded-lg bg-card p-4 text-card-foreground">
        {#if method.kind === 'telegram'}
          <p>{m.methods_telegram()}</p>
        {:else}
          <p>{m.methods_passkey()}</p>
          <p class="text-sm">{added(method.createdAt)}</p>
          {@const position = place(listed.value, method.id)}
          {@const name = m.methods_remove_passkey({ position, date: day(method.createdAt) })}
          {#if confirming === method.id}
            <div role="alertdialog" aria-labelledby="passkey-{method.id}-question">
              <p id="passkey-{method.id}-question" class="mt-3">{m.methods_remove_confirm()}</p>
              <div class="mt-3 flex gap-3">
                <button
                  type="button"
                  class="inline-flex min-h-11 items-center rounded-md border px-4"
                  aria-label={name}
                  disabled={working}
                  onclick={() => removeNow(method.id, position - 1)}
                >
                  {m.methods_remove()}
                </button>
                <button
                  bind:this={keeper}
                  type="button"
                  class="inline-flex min-h-11 items-center rounded-md border px-4"
                  disabled={working}
                  onclick={() => keep(method.id)}
                >
                  {m.methods_remove_keep()}
                </button>
              </div>
            </div>
          {:else}
            <button
              bind:this={removers[method.id]}
              type="button"
              class="mt-3 inline-flex min-h-11 items-center rounded-md border px-4"
              aria-label={name}
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
