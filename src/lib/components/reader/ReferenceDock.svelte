<script lang="ts">
  import { t } from "svelte-i18n";
  import Info from "@lucide/svelte/icons/info";
  import Palette from "@lucide/svelte/icons/palette";
  import XIcon from "@lucide/svelte/icons/x";

  import { Badge } from "$lib/components/ui/badge/index.js";
  import { Button } from "$lib/components/ui/button/index.js";
  import { ScrollArea } from "$lib/components/ui/scroll-area/index.js";
  import { rubySyllables, toneColor } from "$lib/dictionary/pinyin";
  import type { DictWord } from "$lib/dictionary/types";
  import type { DisplayMode } from "$lib/reader/display";
  import { cn } from "$lib/utils.js";

  import DictionaryPanel from "./DictionaryPanel.svelte";
  import PinyinText from "./PinyinText.svelte";

  let {
    words,
    mode,
    toneColors,
    revealed,
    selectedWordIndex,
    hoveredWordIndex,
    onModeChange,
    onToggleTone,
    onSelectWord,
    onHoverWord,
    onDetails,
    onClose,
    onDictionaryBuilt,
  }: {
    words: DictWord[];
    mode: DisplayMode;
    toneColors: boolean;
    revealed: number[];
    selectedWordIndex: number | null;
    hoveredWordIndex: number | null;
    onModeChange: (mode: DisplayMode) => void;
    onToggleTone: () => void;
    onSelectWord: (index: number) => void;
    onHoverWord: (index: number | null) => void;
    onDetails: (index: number) => void;
    onClose?: () => void;
    onDictionaryBuilt?: () => void;
  } = $props();

  function primaryEntry(word: DictWord) {
    return word.entries[0];
  }

  function definition(word: DictWord): string {
    const entry = primaryEntry(word);
    if (!entry) return "";
    return entry.senses
      .slice(0, 2)
      .map((sense) => sense.definition)
      .join("; ");
  }

  function pinyinVisible(index: number): boolean {
    return mode !== "hidden" || revealed.includes(index);
  }

  function definitionVisible(index: number): boolean {
    return mode === "full" || revealed.includes(index);
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

  <div class="flex shrink-0 items-center gap-2 border-b px-2 py-2">
    <select
      onchange={(event) =>
        onModeChange(event.currentTarget.value as DisplayMode)}
      aria-label={$t("display.label")}
      class="h-10 min-w-0 flex-1 cursor-pointer rounded-md border border-input bg-background px-2 text-sm text-foreground shadow-xs outline-none focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50"
    >
      <option value="hidden" selected={mode === "hidden"}>
        {$t("display.hidden")}
      </option>
      <option value="pinyin" selected={mode === "pinyin"}>
        {$t("display.pinyin")}
      </option>
      <option value="full" selected={mode === "full"}>
        {$t("display.full")}
      </option>
    </select>

    <Button
      variant={toneColors ? "secondary" : "ghost"}
      size="icon"
      class="size-10 shrink-0"
      aria-pressed={toneColors}
      aria-label={$t("display.toneColors")}
      title={$t("display.toneColors")}
      onclick={onToggleTone}
    >
      <Palette />
    </Button>
  </div>

  <ScrollArea class="min-h-0 flex-1">
    <div class="flex flex-col gap-1 p-2" role="list">
      {#if words.length === 0}
        <p class="px-3 py-10 text-center text-sm text-muted-foreground">
          {$t("dock.empty")}
        </p>
      {:else}
        {#each words as word, index}
          {@const entry = primaryEntry(word)}
          {@const ruby = rubySyllables(word)}
          {@const showPinyin = pinyinVisible(index)}
          {@const showDefinition = definitionVisible(index)}
          {@const selected = selectedWordIndex === index}
          {@const hovered = hoveredWordIndex === index && !selected}

          <div
            id={`word-card-${index}`}
            role="listitem"
            class={cn(
              "group relative rounded-md transition-colors hover:bg-accent/60",
              selected && "bg-accent",
              hovered && "bg-accent/40",
            )}
            onpointerenter={(event) => {
              if (event.pointerType === "mouse") onHoverWord(index);
            }}
            onpointerleave={() => onHoverWord(null)}
          >
            <button
              type="button"
              aria-expanded={mode === "full" ? undefined : showDefinition}
              aria-pressed={selected}
              onclick={() => onSelectWord(index)}
              onfocus={() => onHoverWord(index)}
              onblur={() => onHoverWord(null)}
              class="flex w-full flex-col gap-1.5 rounded-md px-3 py-3 text-left focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
            >
              {#if ruby}
                <span class="flex flex-wrap items-end gap-x-0.5 gap-y-1 pr-10">
                  {#each ruby as syllable}
                    <span class="relative inline-flex flex-col items-center pt-5">
                      <span
                        class={cn(
                          "absolute top-0 left-1/2 -translate-x-1/2 whitespace-nowrap text-[0.8rem] leading-none tracking-tight transition-[filter,opacity]",
                          !showPinyin &&
                            "pointer-events-none select-none opacity-50 blur-[4px]",
                        )}
                        style:color={toneColors && showPinyin
                          ? toneColor(syllable.tone)
                          : undefined}
                        aria-hidden={!showPinyin}
                      >
                        {syllable.pinyin}
                      </span>
                      <span class="font-reader text-[1.75rem] leading-none">
                        {syllable.char}
                      </span>
                    </span>
                  {/each}
                </span>
              {:else}
                <span class="block pr-10 font-reader text-[1.75rem] leading-none">
                  {word.text}
                </span>
                <span
                  class={cn(
                    "mt-1 block pr-8 text-[0.8rem] text-muted-foreground transition-[filter,opacity]",
                    !showPinyin &&
                      "pointer-events-none select-none opacity-50 blur-[4px]",
                  )}
                  aria-hidden={!showPinyin}
                >
                  {#if entry}
                    <PinyinText {entry} {toneColors} />
                  {/if}
                </span>
              {/if}

              <span
                class={cn(
                  "line-clamp-2 text-[0.8rem] leading-relaxed text-muted-foreground transition-[filter,opacity]",
                  !showDefinition &&
                    "pointer-events-none select-none opacity-50 blur-[4px]",
                )}
                aria-hidden={!showDefinition}
              >
                {definition(word)}
              </span>
            </button>

            <Button
              variant="ghost"
              size="icon"
              class="absolute right-1.5 top-1.5 !size-11 text-muted-foreground opacity-60 transition-opacity hover:opacity-100 focus-visible:opacity-100 sm:opacity-40 sm:group-hover:opacity-100 sm:group-focus-within:opacity-100"
              onclick={() => onDetails(index)}
              aria-label={$t("dock.details", {
                values: { word: word.text },
              })}
            >
              <Info class="size-4" />
            </Button>
          </div>
        {/each}
      {/if}
    </div>
  </ScrollArea>

  <DictionaryPanel onBuilt={onDictionaryBuilt} />
</div>
