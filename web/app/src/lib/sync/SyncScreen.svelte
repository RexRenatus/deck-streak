<script lang="ts">
  // SPEC-377 R8, R10, R11; ADR-388 D11, D13. The sync screen.
  import { m } from '$lib/paraglide/messages.js';
  import ChoiceScreen from './ChoiceScreen.svelte';
  import type { SyncClient } from './status';

  let {
    client,
    lost,
    fetch = globalThis.fetch
  }: { client: () => Promise<SyncClient>; lost: () => boolean; fetch?: typeof globalThis.fetch } = $props();

</script>

<main class="mx-auto flex max-w-prose flex-col gap-4 px-6 py-12">
  <h1 class="text-2xl font-semibold tracking-tight">{m.sync_title()}</h1>
  <form
    class="flex flex-col gap-3"
    onsubmit={(event) => {
      event.preventDefault();
      void client().then(() => fetch);
    }}
  >
    <input type="text" />
    <input type="password" />
    <button type="submit" class="min-h-11 rounded-md border px-4">{m.sync_sign_in()}</button>
  </form>
  <ChoiceScreen client={{ choiceCount: async () => ({ status: 'absent', counts: null, snapshot: null }), choiceConfirm: async () => ({ status: 'absent', outcome: 'unsent' }), choiceCancel: async () => null }} lost={lost()} ondone={() => undefined} />
</main>
