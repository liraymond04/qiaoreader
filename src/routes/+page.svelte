<script lang="ts">
  import { onMount, tick } from "svelte";
  import { browser } from "$app/environment";
  import { t } from "svelte-i18n";
  import BookOpen from "@lucide/svelte/icons/book-open";
  import PanelRightClose from "@lucide/svelte/icons/panel-right-close";
  import PanelRightOpen from "@lucide/svelte/icons/panel-right-open";

  import LocalePicker from "$lib/components/LocalePicker.svelte";
  import ReaderCanvas from "$lib/components/reader/ReaderCanvas.svelte";
  import ReferenceDock from "$lib/components/reader/ReferenceDock.svelte";
  import WordDetailsDialog from "$lib/components/reader/WordDetailsDialog.svelte";
  import { Button } from "$lib/components/ui/button/index.js";
  import {
    Content as SheetContent,
    Description as SheetDescription,
    Header as SheetHeader,
    Root as Sheet,
    Title as SheetTitle,
  } from "$lib/components/ui/sheet/index.js";
  import {
    Content as TooltipContent,
    Root as Tooltip,
    Trigger as TooltipTrigger,
  } from "$lib/components/ui/tooltip/index.js";
  import { processRawText } from "$lib/dictionary/api";
  import type { ProcessedText } from "$lib/dictionary/types";
  import {
    DEFAULT_DISPLAY_MODE,
    isDisplayMode,
    type DisplayMode,
  } from "$lib/reader/display";

  const SAMPLE_TEXT =
    "秋天的傍晚，我沿着河边慢慢走回家。远处的山被夕阳染成了金色，风吹过稻田，带来一阵清香。我想起小时候，祖母也常在这样的傍晚，坐在门口给我讲故事。";
  const MODE_KEY = "qiaoreader.displayMode";
  const TONE_KEY = "qiaoreader.toneColors";

  let processed = $state<ProcessedText | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let displayMode = $state<DisplayMode>(loadDisplayMode());
  let toneColors = $state(loadToneColors());
  let revealed = $state<number[]>([]);
  let selectedWordIndex = $state<number | null>(null);
  let hoveredWordIndex = $state<number | null>(null);
  let detailsWordIndex = $state<number | null>(null);
  let dockOpen = $state(true);
  let sheetOpen = $state(false);
  let isCompact = $state(
    browser ? window.matchMedia("(max-width: 639px)").matches : false,
  );

  let segments = $derived(processed?.segments ?? []);
  let words = $derived(processed?.words ?? []);
  let detailsWord = $derived(
    detailsWordIndex === null ? null : (words[detailsWordIndex] ?? null),
  );
  let dockVisible = $derived(isCompact ? sheetOpen : dockOpen);

  function loadDisplayMode(): DisplayMode {
    if (!browser) return DEFAULT_DISPLAY_MODE;
    const stored = localStorage.getItem(MODE_KEY);
    return isDisplayMode(stored) ? stored : DEFAULT_DISPLAY_MODE;
  }

  function loadToneColors(): boolean {
    if (!browser) return true;
    return localStorage.getItem(TONE_KEY) !== "false";
  }

  function changeMode(next: DisplayMode) {
    displayMode = next;
    if (browser) localStorage.setItem(MODE_KEY, next);
    revealed = [];
    selectedWordIndex = null;
    hoveredWordIndex = null;
  }

  function toggleToneColors() {
    toneColors = !toneColors;
    if (browser) localStorage.setItem(TONE_KEY, String(toneColors));
  }

  function addRevealed(index: number) {
    if (!revealed.includes(index)) {
      revealed = [...revealed, index];
    }
  }

  function removeRevealed(index: number) {
    revealed = revealed.filter((value) => value !== index);
  }

  function openDock() {
    if (isCompact) {
      sheetOpen = true;
    } else {
      dockOpen = true;
    }
  }

  async function scrollToCard(index: number) {
    await tick();
    requestAnimationFrame(() => {
      document.getElementById(`word-card-${index}`)?.scrollIntoView({
        block: "nearest",
        behavior: window.matchMedia("(prefers-reduced-motion: reduce)")
          .matches
          ? "auto"
          : "smooth",
      });
    });
  }

  // Reader tap/click: select + reveal, or hide/unhighlight when already shown.
  function selectWord(index: number) {
    const fullyVisible =
      displayMode === "full" || revealed.includes(index);

    if (selectedWordIndex === index && fullyVisible) {
      selectedWordIndex = null;
      if (displayMode !== "full") removeRevealed(index);
      return;
    }

    selectedWordIndex = index;
    if (displayMode !== "full") addRevealed(index);
    openDock();
    void scrollToCard(index);
  }

  // Card tap/click: toggle reveal for this card; reveal implies highlight.
  function toggleCard(index: number) {
    if (displayMode === "full") {
      selectedWordIndex = selectedWordIndex === index ? null : index;
      return;
    }

    if (revealed.includes(index)) {
      removeRevealed(index);
      if (selectedWordIndex === index) selectedWordIndex = null;
    } else {
      addRevealed(index);
      selectedWordIndex = index;
      void scrollToCard(index);
    }
  }

  function openDetails(index: number) {
    detailsWordIndex = index;
  }

  function closeDetails() {
    detailsWordIndex = null;
  }

  async function loadSample() {
    loading = true;
    error = null;
    selectedWordIndex = null;
    hoveredWordIndex = null;
    revealed = [];
    detailsWordIndex = null;
    try {
      processed = await processRawText(SAMPLE_TEXT);
    } catch (cause) {
      error = describe(cause);
    } finally {
      loading = false;
    }
  }

  function describe(cause: unknown): string {
    if (typeof cause === "object" && cause !== null && "message" in cause) {
      return String((cause as { message: unknown }).message);
    }
    return String(cause);
  }

  function toggleDock() {
    if (isCompact) {
      sheetOpen = !sheetOpen;
    } else {
      dockOpen = !dockOpen;
    }
  }

  onMount(() => {
    void loadSample();

    const media = window.matchMedia("(max-width: 639px)");
    const apply = () => {
      isCompact = media.matches;
    };
    apply();
    media.addEventListener("change", apply);
    return () => media.removeEventListener("change", apply);
  });
</script>

<div class="flex h-dvh flex-col overflow-hidden bg-background text-foreground">
  <header
    class="flex h-14 shrink-0 items-center gap-2 border-b bg-background px-3 sm:px-4"
  >
    <div class="flex min-w-0 items-center gap-2">
      <BookOpen class="size-5 text-muted-foreground" />
      <h1 class="truncate text-sm font-semibold tracking-tight">QiaoReader</h1>
    </div>

    <div class="ml-auto flex items-center gap-2">
      <LocalePicker />

      <Tooltip>
        <TooltipTrigger>
          {#snippet child({ props })}
            <Button
              {...props}
              variant="outline"
              size="icon-lg"
              class="size-11"
              onclick={toggleDock}
              aria-label={dockVisible ? $t("dock.hide") : $t("dock.show")}
              aria-expanded={dockVisible}
            >
              {#if dockVisible}
                <PanelRightClose />
              {:else}
                <PanelRightOpen />
              {/if}
            </Button>
          {/snippet}
        </TooltipTrigger>
        <TooltipContent>
          {dockVisible ? $t("dock.hide") : $t("dock.show")}
        </TooltipContent>
      </Tooltip>
    </div>
  </header>

  {#if error}
    <div
      class="border-b border-destructive/20 bg-destructive/10 px-4 py-2 text-sm text-destructive"
    >
      {$t("reader.error")} {error}
    </div>
  {/if}

  <main class="flex min-h-0 flex-1">
    <section class="min-w-0 flex-1 overflow-y-auto">
      <ReaderCanvas
        {segments}
        {loading}
        {selectedWordIndex}
        {hoveredWordIndex}
        onSelectWord={selectWord}
        onHoverWord={(index) => (hoveredWordIndex = index)}
      />
    </section>

    {#if !isCompact && dockOpen}
      <aside class="flex w-1/4 shrink-0 flex-col border-l bg-sidebar">
        <ReferenceDock
          {words}
          mode={displayMode}
          {toneColors}
          {revealed}
          {selectedWordIndex}
          {hoveredWordIndex}
          onModeChange={changeMode}
          onToggleTone={toggleToneColors}
          onSelectWord={toggleCard}
          onHoverWord={(index) => (hoveredWordIndex = index)}
          onDetails={openDetails}
          onClose={() => (dockOpen = false)}
          onDictionaryBuilt={loadSample}
        />
      </aside>
    {/if}
  </main>

  {#if isCompact}
    <Sheet bind:open={sheetOpen}>
      <SheetContent
        side="right"
        class="w-[88%] max-w-sm p-0"
        showCloseButton={false}
      >
        <SheetHeader class="sr-only">
          <SheetTitle>{$t("dock.title")}</SheetTitle>
          <SheetDescription>
            {$t("dock.words", { values: { count: words.length } })}
          </SheetDescription>
        </SheetHeader>

        <ReferenceDock
          {words}
          mode={displayMode}
          {toneColors}
          {revealed}
          {selectedWordIndex}
          {hoveredWordIndex}
          onModeChange={changeMode}
          onToggleTone={toggleToneColors}
          onSelectWord={toggleCard}
          onHoverWord={(index) => (hoveredWordIndex = index)}
          onDetails={openDetails}
          onClose={() => (sheetOpen = false)}
          onDictionaryBuilt={loadSample}
        />
      </SheetContent>
    </Sheet>
  {/if}

  <WordDetailsDialog
    word={detailsWord}
    {toneColors}
    onClose={closeDetails}
  />
</div>
