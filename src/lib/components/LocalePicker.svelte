<script lang="ts">
  import { t, locale } from "svelte-i18n";

  import { setLocale } from "$lib/i18n";
  import { SUPPORTED_LOCALES } from "$lib/i18n/locales";

  let selected = $state("en");

  $effect(() => {
    selected = $locale ?? "en";
  });
</script>

<select
  bind:value={selected}
  onchange={(event) => setLocale(event.currentTarget.value)}
  aria-label={$t("language.label")}
  class="h-9 cursor-pointer rounded-md border border-input bg-background px-2 text-sm text-foreground shadow-xs outline-none focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50"
>
  {#each SUPPORTED_LOCALES as item}
    <option value={item.code}>{item.label}</option>
  {/each}
</select>
