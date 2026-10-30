<script lang="ts">
  import { t } from "svelte-i18n";

  import { Skeleton } from "$lib/components/ui/skeleton/index.js";
  import type { TextSegment } from "$lib/dictionary/types";
  import { cn } from "$lib/utils.js";

  let {
    segments,
    loading = false,
    selectedWordIndex = null,
    onSelectWord,
  }: {
    segments: TextSegment[];
    loading?: boolean;
    selectedWordIndex?: number | null;
    onSelectWord: (index: number) => void;
  } = $props();

  function select(segment: TextSegment) {
    if (segment.wordIndex === null) return;
    onSelectWord(segment.wordIndex);
  }
</script>

<article class="mx-auto w-full max-w-2xl px-5 py-8 sm:px-8 sm:py-12">
  <p class="mb-6 text-xs font-medium uppercase tracking-[0.2em] text-muted-foreground">
    {$t("reader.sample")}
  </p>

  {#if loading}
    <div class="space-y-3" aria-label={$t("reader.loading")}>
      {#each ["w-[96%]", "w-[88%]", "w-[93%]", "w-[72%]", "w-[90%]", "w-[84%]", "w-[61%]"] as width}
        <Skeleton class={cn("h-7 rounded-sm", width)} />
      {/each}
    </div>
  {:else}
    <div class="whitespace-pre-wrap font-reader text-[1.35rem] leading-[2.05] sm:text-2xl sm:leading-[2.15]">
      {#each segments as segment}
        {#if segment.wordIndex !== null}
          <button
            type="button"
            aria-pressed={segment.wordIndex === selectedWordIndex}
            class={cn(
              "inline cursor-pointer rounded-[3px] px-px underline decoration-dotted decoration-border underline-offset-[0.35em] transition-colors hover:bg-accent hover:decoration-foreground/50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/60",
              segment.wordIndex === selectedWordIndex && "bg-primary/10",
            )}
            onclick={() => select(segment)}
          >
            {segment.text}
          </button>
        {:else}
          <span>{segment.text}</span>
        {/if}
      {/each}
    </div>
  {/if}
</article>
