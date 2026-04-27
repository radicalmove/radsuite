<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";

  type EngineStatus = {
    id: string;
    label: string;
    available: boolean;
    detail: string;
  };

  type AppStatus = {
    app_name: string;
    database_ready: boolean;
    sync_configured: boolean;
    engines: EngineStatus[];
  };

  const fallbackStatus: AppStatus = {
    app_name: "RADsuite",
    database_ready: false,
    sync_configured: false,
    engines: [],
  };

  let status = $state<AppStatus>(fallbackStatus);
  let error = $state<string | null>(null);

  onMount(() => {
    invoke<AppStatus>("get_app_status")
      .then((nextStatus) => {
        status = nextStatus;
        error = null;
      })
      .catch((reason: unknown) => {
        error = reason instanceof Error ? reason.message : String(reason);
      });
  });
</script>

<main class="app-shell">
  <header class="topbar">
    <div>
      <p class="eyebrow">Internal alpha</p>
      <h1>{status.app_name}</h1>
    </div>
    <div class="status-strip" aria-label="Application status">
      <span class="pill" class:pill-active={status.database_ready}>Local DB</span>
      <span class="pill" class:pill-active={status.sync_configured}>Sync</span>
    </div>
  </header>

  {#if error}
    <div class="notice">Command bridge unavailable: {error}</div>
  {/if}

  <section class="workspace-grid">
    <section class="panel project-panel" aria-labelledby="projects-heading">
      <div class="panel-heading">
        <p class="eyebrow">Workspace</p>
        <h2 id="projects-heading">Projects</h2>
      </div>
      <div class="empty-state">
        <strong>No local projects yet</strong>
        <span>Project sync and creation controls land after the foundation shell.</span>
      </div>
    </section>

    <section class="panel" aria-labelledby="engines-heading">
      <div class="panel-heading">
        <p class="eyebrow">Native runtimes</p>
        <h2 id="engines-heading">Engines</h2>
      </div>
      <div class="engine-list">
        {#each status.engines as engine (engine.id)}
          <article class="engine-row">
            <div>
              <strong>{engine.label}</strong>
              <span>{engine.detail}</span>
            </div>
            <span class="pill" class:pill-active={engine.available}>
              {engine.available ? "Ready" : "Missing"}
            </span>
          </article>
        {/each}
      </div>
    </section>
  </section>
</main>
