<script lang="ts">
  import type { Answer } from '$lib/api';
  import type { WalletView } from './wallet';

  let { wallet }: { wallet: Answer<WalletView> | undefined } = $props();
</script>

<header data-wallet={wallet?.kind}></header>
