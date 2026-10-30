<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";

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

  function mb(bytes: number | null | undefined): string {
    if (!bytes) return "—";
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }

  function when(unix: number | null | undefined): string {
    return unix ? new Date(unix * 1000).toLocaleString() : "—";
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
  <h1>QiaoReader</h1>
  <p class="subtitle">
    Backend diagnostic — temporary surface while Milestone 1 lands. The reader
    UI replaces this in Milestone 2.
  </p>

  {#if error}
    <p class="error">{error}</p>
  {/if}

  <section class="card">
    <h2>Dictionary</h2>

    {#if status}
      <dl>
        <dt>Status</dt>
        <dd class={status.ready ? "ok" : "warn"}>
          {status.ready ? "ready" : "not built"}
        </dd>
        <dt>Entries</dt>
        <dd>{status.entryCount.toLocaleString()}</dd>
        <dt>Senses</dt>
        <dd>{status.senseCount.toLocaleString()}</dd>
        <dt>Schema</dt>
        <dd>v{status.schemaVersion}</dd>
        <dt>Database</dt>
        <dd class="mono">{status.dbPath} ({mb(status.dbBytes)})</dd>
        <dt>Last build</dt>
        <dd>
          {when(status.builtAtUnix)}
          {status.buildMs ? ` · ${status.buildMs} ms` : ""}
        </dd>
        <dt>CC-CEDICT</dt>
        <dd>
          {#if status.cedictDate}{status.cedictDate}{/if}
          {#if status.cedictPublisher} · {status.cedictPublisher}{/if}
          {#if status.cedictLicense}
            · <a href={status.cedictLicense}>CC BY-SA 4.0</a>
          {/if}
        </dd>
        {#if status.skippedLines}
          <dt>Skipped lines</dt>
          <dd class="warn">{status.skippedLines}</dd>
        {/if}
      </dl>
    {:else}
      <p>Loading…</p>
    {/if}

    <h3>Source file</h3>
    {#if source}
      {#if source.found}
        <p class="mono">{source.path} ({mb(source.bytes)})</p>
      {:else}
        <p class="warn">
          Not found. Run <code>bun run dict:fetch</code>, or pass an explicit
          <code>sourcePath</code>.
        </p>
        <details>
          <summary>Locations probed ({source.searched.length})</summary>
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
        {busy ? "Working…" : "Build dictionary"}
      </button>
      <button onclick={() => build(true)} disabled={busy || !source?.found}>
        Force rebuild
      </button>
      <button onclick={() => refresh()} disabled={busy}>Refresh status</button>
    </div>

    {#if progress}
      <p class="progress">
        {progress.phase}: {progress.processed.toLocaleString()}
        {#if progress.total > 0}
          / {progress.total.toLocaleString()}
        {/if}
        lines · {progress.entries.toLocaleString()} entries
      </p>
      <progress
        value={progress.total > 0 ? progress.processed : undefined}
        max={progress.total > 0 ? progress.total : undefined}
      ></progress>
    {/if}

    {#if report}
      <p class="ok">
        {report.skipped
          ? "Database already up to date."
          : `Imported ${report.entriesImported.toLocaleString()} entries / ${report.sensesImported.toLocaleString()} senses in ${report.elapsedMs} ms.`}
      </p>
      {#if report.warnings.length}
        <details>
          <summary>{report.warnings.length} warning(s)</summary>
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

  h1 {
    margin-bottom: 0.25rem;
  }

  .subtitle {
    margin-top: 0;
    opacity: 0.7;
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

  .mono,
  code {
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
