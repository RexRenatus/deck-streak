<script lang="ts">
  // SPEC-516 R1, R4 to R7; ADR-534 D1, D4 to D6. The data screen's route, over the app's one engine,
  // its one API client and the platform Telegram's wrapper read.
  import { api } from '$lib/api';
  import DataScreen from '$lib/data/DataScreen.svelte';
  import { studyEngine } from '$lib/study/engine';
  import { telegram } from '$lib/telegram.svelte';
</script>

<DataScreen client={() => studyEngine().client()} data={api} platform={telegram.platform} />
