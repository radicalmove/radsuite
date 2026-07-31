<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-dialog";
  import type {
    AudioOutputFormat,
    RadcastAudioListing,
    RadcastAudioOutput,
    RadcastAudioSource,
  } from "../types";

  type Props = {
    selectedProjectId: string | null;
  };

  let { selectedProjectId }: Props = $props();

  let sources = $state<RadcastAudioSource[]>([]);
  let outputs = $state<RadcastAudioOutput[]>([]);
  let selectedSourceId = $state<string | null>(null);
  let clipStart = $state(0);
  let clipEnd = $state(0);
  let outputFormat = $state<AudioOutputFormat>("mp3");
  let cleanupEnabled = $state(true);
  let loading = $state(false);
  let processing = $state(false);
  let error = $state<string | null>(null);
  let status = $state<string | null>(null);

  let selectedSource = $derived(
    sources.find((source) => source.id === selectedSourceId) ?? sources[0] ?? null,
  );
  let sourceAudioUrl = $derived(selectedSource ? convertFileSrc(selectedSource.path) : null);
  let processDisabled = $derived(
    processing || !selectedSource || clipEnd <= clipStart || clipEnd > selectedSource.duration_seconds,
  );

  $effect(() => {
    selectedProjectId;
    void refreshAudio();
  });

  function toErrorMessage(reason: unknown): string {
    return reason instanceof Error ? reason.message : String(reason);
  }

  function formatDuration(seconds: number): string {
    const safe = Number.isFinite(seconds) ? Math.max(0, seconds) : 0;
    const minutes = Math.floor(safe / 60);
    const remainder = Math.floor(safe % 60);
    return `${minutes}:${remainder.toString().padStart(2, "0")}`;
  }

  function formatBytes(bytes: number): string {
    if (bytes < 1024 * 1024) return `${Math.max(1, Math.round(bytes / 1024))} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }

  function setSelectedSource(sourceId: string | null) {
    selectedSourceId = sourceId;
    const source = sources.find((item) => item.id === sourceId);
    clipStart = 0;
    clipEnd = source?.duration_seconds ?? 0;
    error = null;
    status = null;
  }

  async function refreshAudio() {
    loading = true;
    error = null;
    try {
      const result = await invoke<RadcastAudioListing>("list_radcast_audio", {
        request: { project_id: selectedProjectId },
      });
      sources = result.sources;
      outputs = result.outputs;
      const nextSource = result.sources.find((source) => source.id === selectedSourceId) ?? result.sources[0] ?? null;
      selectedSourceId = nextSource?.id ?? null;
      clipStart = 0;
      clipEnd = nextSource?.duration_seconds ?? 0;
    } catch (reason: unknown) {
      error = `Could not load RADcast audio: ${toErrorMessage(reason)}`;
    } finally {
      loading = false;
    }
  }

  async function chooseSource() {
    error = null;
    try {
      const selected = await open({
        multiple: false,
        directory: false,
        filters: [
          {
            name: "Audio files",
            extensions: ["wav", "mp3", "m4a", "flac", "ogg", "webm", "aac"],
          },
        ],
      });
      const path = typeof selected === "string" ? selected : selected?.[0];
      if (!path) return;

      loading = true;
      const source = await invoke<RadcastAudioSource>("import_radcast_audio", {
        request: {
          project_id: selectedProjectId,
          path,
          original_filename: null,
        },
      });
      sources = [source, ...sources];
      setSelectedSource(source.id);
      status = "Source audio imported";
    } catch (reason: unknown) {
      error = `Could not import audio: ${toErrorMessage(reason)}`;
    } finally {
      loading = false;
    }
  }

  async function processAudio() {
    if (processDisabled || !selectedSource) return;
    processing = true;
    error = null;
    status = "Processing audio";
    try {
      const output = await invoke<RadcastAudioOutput>("process_radcast_audio", {
        request: {
          project_id: selectedProjectId,
          source_id: selectedSource.id,
          output_format: outputFormat,
          clip_start_seconds: clipStart,
          clip_end_seconds: clipEnd,
          cleanup_enabled: cleanupEnabled,
        },
      });
      outputs = [output, ...outputs];
      status = "Audio processing complete";
    } catch (reason: unknown) {
      status = null;
      error = `Could not process audio: ${toErrorMessage(reason)}`;
    } finally {
      processing = false;
    }
  }
</script>

<section class="radcast-workspace" aria-labelledby="radcast-heading">
  <div class="workspace-header">
    <div>
      <p class="eyebrow">RADcast</p>
      <h2 id="radcast-heading">Audio cleanup</h2>
    </div>
    <button class="secondary-button compact-button" type="button" disabled={loading || processing} onclick={() => void refreshAudio()}>
      Refresh
    </button>
  </div>

  {#if error}
    <div class="notice analysis-notice" role="alert">{error}</div>
  {/if}
  {#if status}
    <div class="notice radcast-status" aria-live="polite">{status}</div>
  {/if}

  <div class="radcast-grid">
    <section class="radcast-panel" aria-labelledby="radcast-source-heading">
      <div class="radcast-panel-heading">
        <div>
          <p class="eyebrow">Source audio</p>
          <h3 id="radcast-source-heading">Choose a recording</h3>
        </div>
        <button class="primary-button compact-button" type="button" disabled={loading || processing} onclick={() => void chooseSource()}>
          Choose audio
        </button>
      </div>

      {#if sources.length}
        <label class="field-label" for="radcast-source-select">Saved project audio</label>
        <select
          id="radcast-source-select"
          class="radcast-source-select"
          value={selectedSourceId ?? ""}
          onchange={(event) => setSelectedSource((event.currentTarget as HTMLSelectElement).value || null)}
        >
          {#each sources as source (source.id)}
            <option value={source.id}>{source.original_filename}</option>
          {/each}
        </select>
      {:else}
        <div class="radcast-empty">No audio has been added to this project.</div>
      {/if}

      {#if selectedSource && sourceAudioUrl}
        <div class="radcast-source-meta">
          <strong>{selectedSource.original_filename}</strong>
          <span>{formatDuration(selectedSource.duration_seconds)} · {formatBytes(selectedSource.byte_size)}</span>
        </div>
        <audio class="radcast-audio-player" controls src={sourceAudioUrl}>
          Your browser does not support audio playback.
        </audio>

        <div class="radcast-trim" aria-labelledby="radcast-trim-heading">
          <div class="radcast-subheading">
            <div>
              <p class="eyebrow">Working range</p>
              <h4 id="radcast-trim-heading">Trim without changing the source</h4>
            </div>
            <span>{formatDuration(Math.max(0, clipEnd - clipStart))}</span>
          </div>
          <div class="radcast-trim-fields">
            <label>
              <span>Start (seconds)</span>
              <input type="number" min="0" max={selectedSource.duration_seconds} step="0.1" bind:value={clipStart} />
            </label>
            <label>
              <span>End (seconds)</span>
              <input type="number" min="0" max={selectedSource.duration_seconds} step="0.1" bind:value={clipEnd} />
            </label>
          </div>
        </div>
      {/if}
    </section>

    <section class="radcast-panel" aria-labelledby="radcast-settings-heading">
      <div class="radcast-panel-heading">
        <div>
          <p class="eyebrow">Processing</p>
          <h3 id="radcast-settings-heading">Create a new version</h3>
        </div>
      </div>
      <label class="stack settings-compact-field">
        <span>Output format</span>
        <select bind:value={outputFormat}>
          <option value="mp3">MP3</option>
          <option value="wav">WAV</option>
        </select>
      </label>
      <label class="radcast-check">
        <input type="checkbox" bind:checked={cleanupEnabled} />
        <span>
          <strong>Clean up audio</strong>
          <small>Noise reduction and speech-focused loudness balancing.</small>
        </span>
      </label>
      <div class="radcast-processing-note">
        <span class="status-dot is-ready"></span>
        <span>Local FFmpeg processing is available on this computer.</span>
      </div>
      <button class="primary-button radcast-process-button" type="button" disabled={processDisabled} onclick={() => void processAudio()}>
        {processing ? "Processing" : "Create audio version"}
      </button>
    </section>
  </div>

  <section class="radcast-outputs" aria-labelledby="radcast-output-heading">
    <div class="radcast-panel-heading">
      <div>
        <p class="eyebrow">Project outputs</p>
        <h3 id="radcast-output-heading">Completed versions</h3>
      </div>
      <span>{outputs.length} version{outputs.length === 1 ? "" : "s"}</span>
    </div>
    {#if outputs.length}
      <div class="radcast-output-list">
        {#each outputs as output (output.id)}
          <article class="radcast-output-row">
            <div class="radcast-output-copy">
              <strong>{output.filename}</strong>
              <span>{output.output_format.toUpperCase()} · {formatDuration(output.duration_seconds)}{output.cleanup_enabled ? " · Cleaned" : ""}</span>
            </div>
            <audio controls src={convertFileSrc(output.path)}>
              Your browser does not support audio playback.
            </audio>
            <a class="secondary-button compact-button" href={convertFileSrc(output.path)} download={output.filename}>Download</a>
          </article>
        {/each}
      </div>
    {:else}
      <div class="radcast-empty">Your processed versions will appear here.</div>
    {/if}
  </section>
</section>
