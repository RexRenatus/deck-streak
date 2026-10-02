<script lang="ts">
  import type { Answer } from '$lib/api';
  import { m } from '$lib/paraglide/messages.js';
  import type { WalletView } from './wallet';

  // The wallet header (SPEC-082 R17): the owner's balance on every screen, as a link to the
  // wallet's history. Until a balance arrives, or when none can, the link names the wallet.
  let { wallet }: { wallet: Answer<WalletView> | undefined } = $props();
</script>

<header class="mx-auto flex max-w-prose justify-end px-6 pt-4">
  <a href="/wallet" class="inline-flex min-h-11 items-center font-medium">
    {#if wallet?.kind === 'ok'}
      {m.wallet_balance({ balance: wallet.value.balance })}
    {:else}
      {m.wallet_title()}
    {/if}
  </a>
</header>
