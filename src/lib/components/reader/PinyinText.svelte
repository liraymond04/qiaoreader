<script lang="ts">
  import type { DictEntry } from "$lib/dictionary/types";
  import { pinyinTokens, toneColor } from "$lib/dictionary/pinyin";
  import { cn } from "$lib/utils.js";

  let {
    entry,
    toneColors = true,
    className = "",
  }: {
    entry: DictEntry;
    toneColors?: boolean;
    className?: string;
  } = $props();
</script>

<span class={cn("inline-flex flex-wrap items-baseline gap-x-1", className)}>
  {#each pinyinTokens(entry) as token}
    <span style:color={toneColors ? toneColor(token.tone) : undefined}>
      {token.text}
    </span>
  {/each}
</span>
