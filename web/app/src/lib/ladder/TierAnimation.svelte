<script lang="ts">
  import type { FeedItem } from './feed';

  let { item }: { item: FeedItem } = $props();
</script>

<article data-kind={item.kind}></article>
