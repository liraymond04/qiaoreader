<script lang="ts">
  import { t } from "svelte-i18n";
  import XIcon from "@lucide/svelte/icons/x";

  import { Badge } from "$lib/components/ui/badge/index.js";
  import { Button } from "$lib/components/ui/button/index.js";
  import { ScrollArea } from "$lib/components/ui/scroll-area/index.js";
  import type { DictWord } from "$lib/dictionary/types";
  import { cn } from "$lib/utils.js";

  import DictionaryPanel from "./DictionaryPanel.svelte";

  let {
    words,
    selectedWordIndex,
    onSelectWord,
    onClose,
    onDictionaryBuilt,
  }: {
    words: DictWord[];
    selectedWordIndex: number | null;
    onSelectWord: (index: number) => void;
    onClose?: () => void;
    onDictionaryBuilt?: () => void;
  } = $props();

  function pinyin(word: DictWord): string {
    return word.entries[0]?.pinyinDisplay ?? "";
  }

  function definition(word: DictWord): string {
    const entry = word.entries[0];
    if (!entry) return "";
    return entry.senses
      .slice(0, 2)
      .map((sense) => sense.definition)
      .join("; ");
  }
</script>

<div class="flex h-full min-h-0 flex-col">
  <header class="flex shrink-0 items-center gap-2 border-b px-4 py-3">
    <h2 class="text-sm font-semibold">{$t("dock.title")}</h2>
    <Badge variant="secondary" class="font-normal">
      {$t("dock.words", { values: { count: words.length } })}
    </Badge>
    {#if onClose}
      <Button
        variant="ghost"
        size="icon-sm"
        class="ml-auto"
        onclick={onClose}
        aria-label={$t("dock.close")}
      >
        <XIcon />
      </Button>
    {/if}
  </header>

  <ScrollArea class="min-h-0 flex-1">
    <div class="flex flex-col gap-1 p-2">
      {#if words.length === 0}
        <p class="px-3 py-10 text-center text-sm text-muted-foreground">
          {$t("dock.empty")}
        </p>
      {:else}
        {#each words as word, index}
          <button
            type="button"
            aria-pressed={selectedWordIndex === index}
            onclick={() => onSelectWord(index)}
            class={cn(
              "flex w-full flex-col gap-1 rounded-md px-3 py-2.5 text-left transition-colors hover:bg-accent focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50",
              selectedWordIndex === index && "bg-accent",
            )}
          >
            <span class="flex items-baseline justify-between gap-3">
              <span class="font-reader text-xl leading-none">{word.text}</span>
              <span class="shrink-0 text-xs text-muted-foreground">
                {pinyin(word)}
              </span>
            </span>
            <span class="line-clamp-2 text-xs leading-relaxed text-muted-foreground">
              {definition(word)}
            </span>
          </button>
        {/each}
      {/if}
    </div>
  </ScrollArea>

  <DictionaryPanel onBuilt={onDictionaryBuilt} />
</div>
