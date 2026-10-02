<script lang="ts">
  import { untrack } from 'svelte';
  import { api, type Answer } from '$lib/api';
  import { signed, type Movement, type WalletView } from '$lib/economy/wallet';
  import { m } from '$lib/paraglide/messages.js';

  // The wallet screen (SPEC-082 R15, R17; ADR-315 ruling 3): the balance, today's loss limit and
  // the coin movements in the server's order, newest first, a page at a time. The screen computes
  // none of these numbers.
  let wallet = $state<Answer<WalletView>>();
  // The movements shown so far, and the cursor of the next, older page.
  let movements = $state<Movement[]>([]);
  let next = $state<number | null>(null);
  // Whether an older page was asked for and did not arrive.
  let olderFailed = $state(false);

  // Ask once the screen is on the page, as Today does: untracked, and skipped by a server render.
  $effect(() => {
    void untrack(() => api.wallet()).then((answer) => {
      // The page's lines first, then the answer the screen shows them by.
      if (answer.kind === 'ok') {
        movements = [...answer.value.movements];
        next = answer.value.next;
      }
      wallet = answer;
    });
  });

  /** Appends the page after the last movement shown; the lines shown stay when it does not arrive. */
  async function older() {
    if (next === null) return;
    const answer = await api.wallet(next);
    if (answer.kind === 'ok') {
      olderFailed = false;
      movements = [...movements, ...answer.value.movements];
      next = answer.value.next;
    } else {
      olderFailed = true;
    }
  }

  /** A movement's source in plain words, or its code when it has none. */
  function sourceOf(source: string): string {
    return source === 'mint' ? m.wallet_source_mint() : source;
  }
</script>

<main class="mx-auto max-w-prose px-6 py-12">
  <h1 class="text-3xl font-semibold tracking-tight">{m.wallet_title()}</h1>

  <div class="mt-8 rounded-lg bg-card p-4 text-card-foreground">
    {#if wallet === undefined}
      <div role="status">{m.loading()}</div>
    {:else if wallet.kind === 'ok'}
      <p class="text-xl font-semibold">{m.wallet_balance({ balance: wallet.value.balance })}</p>
      <p class="mt-1">
        {m.wallet_loss_cap({ left: wallet.value.lossCapLeft, cap: wallet.value.lossCap })}
      </p>

      <section class="mt-6" aria-labelledby="wallet-movements">
        <h2 id="wallet-movements" class="text-sm font-medium">{m.wallet_movements()}</h2>
        {#if movements.length === 0}
          <p class="mt-2">{m.wallet_none()}</p>
        {:else}
          <ul class="mt-2 space-y-2">
            {#each movements as movement (movement.id)}
              <li data-movement-id={movement.id} class="flex flex-wrap gap-x-3">
                <time datetime={movement.studyDay}>{movement.studyDay}</time>
                <span>{sourceOf(movement.source)}</span>
                <span class="ml-auto font-medium tabular-nums">{signed(movement.amount)}</span>
              </li>
            {/each}
          </ul>
        {/if}
        {#if olderFailed}
          <div class="mt-2" role="alert">{m.server_unavailable()}</div>
        {/if}
        {#if next !== null}
          <button
            type="button"
            class="mt-4 inline-flex min-h-11 items-center rounded-md border px-4"
            onclick={older}
          >
            {m.wallet_older()}
          </button>
        {/if}
      </section>
    {:else if wallet.kind === 'reopen'}
      <div role="alert">{m.reopen_from_telegram()}</div>
    {:else}
      <div role="alert">{m.server_unavailable()}</div>
    {/if}
  </div>

  <nav class="mt-8">
    <a href="/">{m.back_to_today()}</a>
  </nav>
</main>
