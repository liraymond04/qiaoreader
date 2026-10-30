<script lang="ts">
  import { onMount } from "svelte";
  import { t, locale } from "svelte-i18n";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";

  import { setLocale } from "$lib/i18n";
  import { SUPPORTED_LOCALES } from "$lib/i18n/locales";

  type DictionaryStatus = {
    ready: boolean;
    dbPath: string;
    dbBytes: number;
    schemaVersion: number;
    entryCount: number;
    senseCount: number;
    sourcePath: string | null;
    sourceSha256: string | null;
    sourceBytes: number | null;
    builtAtUnix: number | null;
    buildMs: number | null;
    skippedLines: number | null;
    cedictVersion: string | null;
    cedictDate: string | null;
    cedictPublisher: string | null;
    cedictLicense: string | null;
  };

  type DictionarySourceInfo = {
    found: boolean;
    path: string | null;
    bytes: number | null;
    searched: string[];
  };

  type ImportProgress = {
    phase: string;
    processed: number;
    total: number;
    entries: number;
  };

  type DictionaryBuildReport = {
    status: DictionaryStatus;
    skipped: boolean;
    entriesImported: number;
    sensesImported: number;
    skippedLines: number;
    warnings: string[];
    elapsedMs: number;
    sourcePath: string;
    sourceSha256: string;
  };

  let status = $state<DictionaryStatus | null>(null);
  let source = $state<DictionarySourceInfo | null>(null);
  let progress = $state<ImportProgress | null>(null);
  let report = $state<DictionaryBuildReport | null>(null);
  let error = $state<string | null>(null);
  let busy = $state(false);

  let activeLocale = $derived($locale ?? "en");
  let selectedLocale = $state("en");

  $effect(() => {
    selectedLocale = $locale ?? "en";
  });

  async function refresh() {
    status = await invoke<DictionaryStatus>("dictionary_status");
    source = await invoke<DictionarySourceInfo>("dictionary_source");
  }

  async function build(force: boolean) {
    busy = true;
    error = null;
    progress = null;
    try {
      report = await invoke<DictionaryBuildReport>("dictionary_build", { force });
      status = report.status;
    } catch (cause) {
      error = describeError(cause);
    } finally {
      busy = false;
    }
  }

  function describeError(cause: unknown): string {
    if (typeof cause === "object" && cause !== null && "message" in cause) {
      return String((cause as { message: unknown }).message);
    }
    return String(cause);
  }

  function num(value: number): string {
    return value.toLocaleString(activeLocale);
  }

  function mb(bytes: number | null | undefined): string {
    if (!bytes) return "—";
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }

  function when(unix: number | null | undefined): string {
    return unix ? new Date(unix * 1000).toLocaleString(activeLocale) : "—";
  }

  onMount(() => {
    refresh().catch((cause) => (error = describeError(cause)));

    let unlisten: (() => void) | undefined;
    listen<ImportProgress>("dictionary://import-progress", (event) => {
      progress = event.payload;
    }).then((stop) => (unlisten = stop));

    return () => unlisten?.();
  });
</script>

<main class="container">
  <header class="header">
    <div>
      <h1>QiaoReader</h1>
      <p class="subtitle">{$t("app.subtitle")}</p>
    </div>

    <label class="locale">
      <span>{$t("language.label")}</span>
      <select
        bind:value={selectedLocale}
        onchange={(event) => setLocale(event.currentTarget.value)}
      >
        {#each SUPPORTED_LOCALES as item}
          <option value={item.code}>{item.label}</option>
        {/each}
      </select>
    </label>
  </header>

  {#if error}
    <p class="error">{error}</p>
  {/if}

  <section class="card">
    <h2>{$t("dict.title")}</h2>

    {#if status}
      <dl>
        <dt>{$t("dict.status")}</dt>
        <dd class={status.ready ? "ok" : "warn"}>
          {status.ready ? $t("dict.status.ready") : $t("dict.status.notBuilt")}
        </dd>
        <dt>{$t("dict.entries")}</dt>
        <dd>{num(status.entryCount)}</dd>
        <dt>{$t("dict.senses")}</dt>
        <dd>{num(status.senseCount)}</dd>
        <dt>{$t("dict.schema")}</dt>
        <dd>v{status.schemaVersion}</dd>
        <dt>{$t("dict.database")}</dt>
        <dd class="mono">{status.dbPath} ({mb(status.dbBytes)})</dd>
        <dt>{$t("dict.lastBuild")}</dt>
        <dd>
          {when(status.builtAtUnix)}
          {status.buildMs ? ` · ${num(status.buildMs)} ms` : ""}
        </dd>
        <dt>{$t("dict.cedict")}</dt>
        <dd>
          {#if status.cedictDate}{status.cedictDate}{/if}
          {#if status.cedictPublisher} · {status.cedictPublisher}{/if}
          {#if status.cedictLicense}
            · <a href={status.cedictLicense}>CC BY-SA 4.0</a>
          {/if}
        </dd>
        {#if status.skippedLines}
          <dt>{$t("dict.skippedLines")}</dt>
          <dd class="warn">{num(status.skippedLines)}</dd>
        {/if}
      </dl>
    {:else}
      <p>{$t("common.loading")}</p>
    {/if}

    <h3>{$t("dict.source.title")}</h3>
    {#if source}
      {#if source.found}
        <p class="mono">{source.path} ({mb(source.bytes)})</p>
      {:else}
        <p class="warn">{$t("dict.source.notFound")}</p>
        <details>
          <summary>
            {$t("dict.source.probed", {
              values: { count: source.searched.length },
            })}
          </summary>
          <ul class="searched">
            {#each source.searched as candidate}
              <li class="mono">{candidate}</li>
            {/each}
          </ul>
        </details>
      {/if}
    {/if}

    <div class="actions">
      <button onclick={() => build(false)} disabled={busy || !source?.found}>
        {busy ? $t("dict.action.working") : $t("dict.action.build")}
      </button>
      <button onclick={() => build(true)} disabled={busy || !source?.found}>
        {$t("dict.action.force")}
      </button>
      <button onclick={() => refresh()} disabled={busy}>
        {$t("dict.action.refresh")}
      </button>
    </div>

    {#if progress}
      <p class="progress">
        {$t(progress.total > 0 ? "dict.progress" : "dict.progress.noTotal", {
          values: {
            phase: $t(`phase.${progress.phase}`),
            processed: num(progress.processed),
            total: num(progress.total),
            entries: num(progress.entries),
          },
        })}
      </p>
      <progress
        value={progress.total > 0 ? progress.processed : undefined}
        max={progress.total > 0 ? progress.total : undefined}
      ></progress>
    {/if}

    {#if report}
      <p class="ok">
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
          <summary>
            {$t("dict.warnings", {
              values: { count: report.warnings.length },
            })}
          </summary>
          <ul>
            {#each report.warnings as warning}
              <li>{warning}</li>
            {/each}
          </ul>
        </details>
      {/if}
    {/if}
  </section>
</main>

<style>
  :global(:root) {
    font-family: Inter, Avenir, Helvetica, Arial, sans-serif;
    font-size: 16px;
    line-height: 1.5;
    color: #0f0f0f;
    background-color: #f6f6f6;
  }

  @media (prefers-color-scheme: dark) {
    :global(:root) {
      color: #f6f6f6;
      background-color: #2f2f2f;
    }
  }

  .container {
    max-width: 880px;
    margin: 0 auto;
    padding: 6vh 1.5rem 4rem;
  }

  .header {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-start;
    justify-content: space-between;
    gap: 1rem;
  }

  h1 {
    margin-bottom: 0.25rem;
  }

  .subtitle {
    margin-top: 0;
    opacity: 0.7;
  }

  .locale {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.9rem;
    opacity: 0.85;
  }

  .locale select {
    /* Let the native control pick a matching background and text colour;
       forcing one without the other makes the label invisible on WebKitGTK. */
    color-scheme: light;
    padding: 0.35rem 0.5rem;
    border-radius: 6px;
    border: 1px solid rgba(128, 128, 128, 0.5);
    font: inherit;
  }

  .card {
    margin-top: 2rem;
    padding: 1.5rem;
    border-radius: 12px;
    border: 1px solid rgba(128, 128, 128, 0.35);
  }

  h2,
  h3 {
    margin-top: 0;
  }

  h3 {
    margin-bottom: 0.5rem;
    font-size: 1rem;
    opacity: 0.8;
  }

  dl {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 0.35rem 1.25rem;
    margin: 0 0 1.5rem;
  }

  dt {
    opacity: 0.7;
  }

  dd {
    margin: 0;
    overflow-wrap: anywhere;
  }

  .mono {
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
    font-size: 0.85em;
  }

  .ok {
    color: #15803d;
  }

  .warn {
    color: #b45309;
  }

  .error {
    color: #b91c1c;
    font-weight: 600;
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 0.75rem;
    margin: 1.25rem 0 1rem;
  }

  button {
    padding: 0.55rem 1.1rem;
    border-radius: 8px;
    border: 1px solid rgba(128, 128, 128, 0.5);
    background: rgba(128, 128, 128, 0.12);
    color: inherit;
    font: inherit;
    cursor: pointer;
  }

  button:hover:not(:disabled) {
    border-color: #396cd8;
  }

  button:disabled {
    opacity: 0.5;
    cursor: default;
  }

  progress {
    width: 100%;
  }

  .progress {
    margin: 1rem 0 0.25rem;
    font-size: 0.9rem;
  }

  .searched {
    padding-left: 1.25rem;
    font-size: 0.8rem;
    opacity: 0.85;
  }

  a {
    color: #396cd8;
  }
</style>
