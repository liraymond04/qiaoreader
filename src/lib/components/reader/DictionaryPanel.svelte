<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { locale, t } from "svelte-i18n";
  import BookMarked from "@lucide/svelte/icons/book-marked";

  import { Badge } from "$lib/components/ui/badge/index.js";
  import { Button } from "$lib/components/ui/button/index.js";
  import { Separator } from "$lib/components/ui/separator/index.js";
  import {
    buildDictionary,
    dictionarySource,
    dictionaryStatus,
  } from "$lib/dictionary/api";
  import type {
    DictionaryBuildReport,
    DictionarySourceInfo,
    DictionaryStatus,
    ImportProgress,
  } from "$lib/dictionary/types";

  let { onBuilt }: { onBuilt?: () => void } = $props();

  let status = $state<DictionaryStatus | null>(null);
  let source = $state<DictionarySourceInfo | null>(null);
  let progress = $state<ImportProgress | null>(null);
  let report = $state<DictionaryBuildReport | null>(null);
  let busy = $state(false);
  let error = $state<string | null>(null);
  let unlisten: (() => void) | undefined;

  onMount(() => {
    void refresh();

    listen<ImportProgress>("dictionary://import-progress", (event) => {
      progress = event.payload;
    }).then((stop) => (unlisten = stop));

    return () => unlisten?.();
  });

  async function refresh() {
    try {
      [status, source] = await Promise.all([
        dictionaryStatus(),
        dictionarySource(),
      ]);
      error = null;
    } catch (cause) {
      error = describe(cause);
    }
  }

  async function build(force: boolean) {
    busy = true;
    error = null;
    progress = null;
    try {
      report = await buildDictionary(force);
      status = report.status;
      onBuilt?.();
    } catch (cause) {
      error = describe(cause);
    } finally {
      busy = false;
    }
  }

  function describe(cause: unknown): string {
    if (typeof cause === "object" && cause !== null && "message" in cause) {
      return String((cause as { message: unknown }).message);
    }
    return String(cause);
  }

  function num(value: number): string {
    return value.toLocaleString($locale ?? "en");
  }
</script>

<details class="group shrink-0 border-t">
  <summary
    class="flex cursor-pointer list-none select-none items-center gap-2 px-4 py-3 text-sm hover:bg-accent/40"
  >
    <BookMarked class="size-4 text-muted-foreground" />
    <span class="font-medium">{$t("dict.title")}</span>
    {#if status}
      <Badge variant={status.ready ? "secondary" : "outline"} class="ml-auto">
        {status.ready ? $t("dict.status.ready") : $t("dict.status.notBuilt")}
      </Badge>
    {/if}
  </summary>

  <div class="max-h-[45vh] space-y-3 overflow-y-auto px-4 pb-4">
    {#if error}
      <p class="text-xs text-destructive">{error}</p>
    {/if}

    {#if status}
      <dl class="grid grid-cols-2 gap-x-3 gap-y-1 text-xs">
        <dt class="text-muted-foreground">{$t("dict.entries")}</dt>
        <dd>{num(status.entryCount)}</dd>
        <dt class="text-muted-foreground">{$t("dict.senses")}</dt>
        <dd>{num(status.senseCount)}</dd>
        <dt class="text-muted-foreground">{$t("dict.schema")}</dt>
        <dd>v{status.schemaVersion}</dd>
        <dt class="text-muted-foreground">{$t("dict.lastBuild")}</dt>
        <dd>
          {status.builtAtUnix
            ? new Date(status.builtAtUnix * 1000).toLocaleDateString(
                $locale ?? "en",
              )
            : "—"}
        </dd>
      </dl>
      <p class="truncate text-xs text-muted-foreground" title={status.dbPath}>
        {status.dbPath}
      </p>
    {/if}

    {#if source}
      {#if source.found}
        <p class="truncate text-xs text-muted-foreground" title={source.path ?? ""}>
          {source.path}
        </p>
      {:else}
        <p class="text-xs text-destructive">{$t("dict.source.notFound")}</p>
      {/if}
    {/if}

    <Separator />

    <div class="flex flex-wrap gap-2">
      <Button size="sm" onclick={() => build(false)} disabled={busy}>
        {busy ? $t("dict.action.working") : $t("dict.action.build")}
      </Button>
      <Button
        size="sm"
        variant="outline"
        onclick={() => build(true)}
        disabled={busy}
      >
        {$t("dict.action.force")}
      </Button>
      <Button size="sm" variant="ghost" onclick={() => refresh()} disabled={busy}>
        {$t("dict.action.refresh")}
      </Button>
    </div>

    {#if progress}
      <p class="text-xs text-muted-foreground">
        {$t(progress.total > 0 ? "dict.progress" : "dict.progress.noTotal", {
          values: {
            phase: $t(`phase.${progress.phase}`),
            processed: num(progress.processed),
            total: num(progress.total),
            entries: num(progress.entries),
          },
        })}
      </p>
    {/if}

    {#if report}
      <p class="text-xs text-muted-foreground">
        {$t(report.skipped ? "dict.report.uptodate" : "dict.report.imported", {
          values: {
            entries: num(report.entriesImported),
            senses: num(report.sensesImported),
            ms: num(report.elapsedMs),
          },
        })}
      </p>
      {#if report.warnings.length}
        <details>
          <summary class="cursor-pointer text-xs text-muted-foreground">
            {$t("dict.warnings", {
              values: { count: report.warnings.length },
            })}
          </summary>
          <ul class="mt-1 list-disc space-y-1 pl-4 text-xs text-muted-foreground">
            {#each report.warnings as warning}
              <li>{warning}</li>
            {/each}
          </ul>
        </details>
      {/if}
    {/if}
  </div>
</details>
