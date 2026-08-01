<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-dialog";
  import type {
    AudioOutputFormat,
    CaptionFormat,
    CaptionQualityMode,
    EnhancementModel,
    EnhancementQuality,
    FillerRemovalMode,
    RadcastCapabilityStatus,
    RadcastAudioListing,
    RadcastAudioOutput,
    RadcastAudioSource,
    RadcastJobStatus,
    RadcastProjectSettings,
    RadcastProcessingPhase,
    RadcastTrimRange,
  } from "../types";
  import {
    clampRadcastSilenceSeconds,
    formatRadcastSilenceSeconds,
  } from "../lib/radcastSettings";

  type Props = {
    selectedProjectId: string | null;
  };

  let { selectedProjectId }: Props = $props();

  let sources = $state<RadcastAudioSource[]>([]);
  let outputs = $state<RadcastAudioOutput[]>([]);
  let selectedSourceId = $state<string | null>(null);
  let sourceLink = $state("");
  let clipStart = $state(0);
  let clipEnd = $state(0);
  let outputFormat = $state<AudioOutputFormat>("mp3");
  let captionFormat = $state<CaptionFormat | null>(null);
  let captionLanguage = $state("en");
  let captionQualityMode = $state<CaptionQualityMode>("reviewed");
  let captionGlossary = $state("");
  let enhancementModel = $state<EnhancementModel>("studio_v18");
  let enhancementQuality = $state<EnhancementQuality>("high");
  let cleanupEnabled = $state(true);
  let shortenPauses = $state(false);
  let maxSilenceSeconds = $state(1.0);
  let removeFillerWords = $state(false);
  let fillerRemovalMode = $state<FillerRemovalMode>("aggressive");
  let trimRangesBySourceId = $state<Record<string, RadcastTrimRange>>({});
  let loading = $state(false);
  let deletingSource = $state(false);
  let processing = $state(false);
  let error = $state<string | null>(null);
  let status = $state<string | null>(null);
  let radcastJob = $state<RadcastJobStatus | null>(null);
  let cancelling = $state(false);
  let settingsLoaded = $state(false);
  let settingsSaveTimer: number | null = null;
  let captionCapability = $state<RadcastCapabilityStatus>({
    caption_available: false,
    caption_detail: "Checking local caption support.",
    optimized_available: false,
    optimized_detail: "Checking local enhancement support.",
    enhancement_models: [
      {
        id: "none",
        label: "Standard cleanup",
        description: "Keeps the original audio and applies only the selected cleanup options.",
        available: true,
        detail: "Available without an additional model.",
      },
      {
        id: "resemble",
        label: "Resemble Enhance",
        description: "Strong speech enhancement that can sound more processed on some recordings.",
        available: false,
        detail: "Checking local enhancement support.",
      },
      {
        id: "deepfilternet",
        label: "DeepFilterNet3",
        description: "Natural-sounding speech enhancement using the official DeepFilterNet3 model.",
        available: false,
        detail: "Checking local enhancement support.",
      },
      {
        id: "studio",
        label: "Studio Cleanup",
        description: "Custom room-tail suppression followed by Resemble Enhance for a drier voice.",
        available: false,
        detail: "Checking local enhancement support.",
      },
      {
        id: "studio_v18",
        label: "RADcast Optimized",
        description: "RADcast's tuned lecture-cleanup path with chunked dereverb and speech restoration.",
        available: false,
        detail: "Checking local enhancement support.",
      },
    ],
  });

  let selectedSource = $derived(
    sources.find((source) => source.id === selectedSourceId) ?? sources[0] ?? null,
  );
  let selectedEnhancementCapability = $derived(
    captionCapability.enhancement_models.find((model) => model.id === enhancementModel) ?? null,
  );
  let sourceAudioUrl = $derived(selectedSource ? convertFileSrc(selectedSource.path) : null);
  let processDisabled = $derived(
    processing ||
    !selectedSource ||
    clipEnd <= clipStart ||
    clipEnd > selectedSource.duration_seconds ||
    (settingsLoaded && selectedEnhancementCapability !== null && !selectedEnhancementCapability.available),
  );

  $effect(() => {
    selectedProjectId;
    void refreshAudio();
  });

  $effect(() => {
    const projectId = selectedProjectId;
    const settings = currentSettings();
    if (!settingsLoaded || !projectId || processing) return;
    if (settingsSaveTimer !== null) window.clearTimeout(settingsSaveTimer);
    settingsSaveTimer = window.setTimeout(() => {
      settingsSaveTimer = null;
      void persistSettings(projectId, settings);
    }, 500);
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

  function enhancementQualityLabel(quality: EnhancementQuality | undefined): string {
    if (quality === "fast") return "Fast";
    if (quality === "high") return "High";
    return "Standard";
  }

  function enhancementModelLabel(model: EnhancementModel): string {
    return captionCapability.enhancement_models.find((item) => item.id === model)?.label ?? model;
  }

  function processingPhaseLabel(phase: RadcastProcessingPhase): string {
    const labels: Record<RadcastProcessingPhase, string> = {
      preparing: "Preparing audio",
      removing_filler_words: "Finding filler words",
      preparing_enhancement: "Preparing enhancement",
      enhancing_audio: "Enhancing speech",
      rendering_audio: "Rendering audio",
      generating_captions: "Generating captions",
      saving_output: "Saving output",
    };
    return labels[phase];
  }

  function formatElapsed(seconds: number): string {
    const safe = Number.isFinite(seconds) ? Math.max(0, Math.floor(seconds)) : 0;
    const minutes = Math.floor(safe / 60);
    const remainder = safe % 60;
    return minutes ? `${minutes}m ${remainder}s` : `${remainder}s`;
  }

  function setSelectedSource(sourceId: string | null) {
    rememberSelectedTrimRange();
    selectedSourceId = sourceId;
    const source = sources.find((item) => item.id === sourceId);
    const savedRange = source ? trimRangesBySourceId[source.id] : null;
    clipStart = savedRange?.clip_start_seconds ?? 0;
    clipEnd = savedRange?.clip_end_seconds ?? source?.duration_seconds ?? 0;
    error = null;
    status = null;
    radcastJob = null;
  }

  function rememberSelectedTrimRange() {
    if (!selectedSourceId) return;
    const source = sources.find((item) => item.id === selectedSourceId);
    if (!source || !Number.isFinite(clipStart) || !Number.isFinite(clipEnd)) return;
    if (clipStart < 0 || clipEnd <= clipStart || clipEnd > source.duration_seconds) return;
    trimRangesBySourceId = {
      ...trimRangesBySourceId,
      [selectedSourceId]: {
        clip_start_seconds: clipStart,
        clip_end_seconds: clipEnd,
      },
    };
  }

  function currentSettings(): RadcastProjectSettings {
    const trimRanges = { ...trimRangesBySourceId };
    if (
      selectedSourceId &&
      selectedSource &&
      Number.isFinite(clipStart) &&
      Number.isFinite(clipEnd) &&
      clipStart >= 0 &&
      clipEnd > clipStart &&
      clipEnd <= selectedSource.duration_seconds
    ) {
      trimRanges[selectedSourceId] = {
        clip_start_seconds: clipStart,
        clip_end_seconds: clipEnd,
      };
    }
    return {
      output_format: outputFormat,
      caption_format: captionFormat,
      caption_language: captionLanguage,
      caption_quality_mode: captionQualityMode,
      caption_glossary: captionGlossary.trim() || null,
      enhancement_model: enhancementModel,
      enhancement_quality: enhancementQuality,
      cleanup_enabled: cleanupEnabled,
      max_silence_seconds: shortenPauses ? clampRadcastSilenceSeconds(maxSilenceSeconds) : null,
      remove_filler_words: removeFillerWords,
      filler_removal_mode: fillerRemovalMode,
      trim_ranges_by_source_id: trimRanges,
    };
  }

  async function persistSettings(projectId: string, settings: RadcastProjectSettings) {
    if (!settingsLoaded || processing) return;
    try {
      await invoke<RadcastProjectSettings>("save_radcast_settings", {
        request: { project_id: projectId, settings },
      });
    } catch (reason: unknown) {
      error = `Could not save RADcast settings: ${toErrorMessage(reason)}`;
    }
  }

  async function refreshAudio() {
    loading = true;
    settingsLoaded = false;
    error = null;
    try {
      const result = await invoke<RadcastAudioListing>("list_radcast_audio", {
        request: { project_id: selectedProjectId },
      });
      const capabilities = await invoke<RadcastCapabilityStatus>("get_radcast_capabilities");
      captionCapability = capabilities;
      outputFormat = result.settings.output_format;
      captionFormat = result.settings.caption_format;
      captionLanguage = result.settings.caption_language;
      captionQualityMode = result.settings.caption_quality_mode;
      captionGlossary = result.settings.caption_glossary ?? "";
      enhancementModel = result.settings.enhancement_model;
      enhancementQuality = result.settings.enhancement_quality;
      cleanupEnabled = result.settings.cleanup_enabled;
      shortenPauses = result.settings.max_silence_seconds !== null;
      maxSilenceSeconds = clampRadcastSilenceSeconds(result.settings.max_silence_seconds ?? 1.0);
      removeFillerWords = result.settings.remove_filler_words;
      fillerRemovalMode = result.settings.filler_removal_mode;
      trimRangesBySourceId = result.settings.trim_ranges_by_source_id ?? {};
      if (!captionCapability.caption_available) {
        captionFormat = null;
        removeFillerWords = false;
      }
      if (!capabilities.enhancement_models.some((model) => model.id === enhancementModel && model.available)) {
        enhancementModel = "none";
      }
      sources = result.sources;
      outputs = result.outputs;
      const nextSource = result.sources.find((source) => source.id === selectedSourceId) ?? result.sources[0] ?? null;
      selectedSourceId = nextSource?.id ?? null;
      const savedRange = nextSource ? trimRangesBySourceId[nextSource.id] : null;
      clipStart = savedRange?.clip_start_seconds ?? 0;
      clipEnd = savedRange?.clip_end_seconds ?? nextSource?.duration_seconds ?? 0;
      settingsLoaded = true;
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

  async function importSourceFromLink() {
    const url = sourceLink.trim();
    if (!url) {
      error = "Paste a OneDrive or SharePoint sharing link first.";
      return;
    }
    error = null;
    try {
      loading = true;
      const source = await invoke<RadcastAudioSource>("import_radcast_audio_from_link", {
        request: {
          project_id: selectedProjectId,
          url,
        },
      });
      sources = [source, ...sources];
      sourceLink = "";
      setSelectedSource(source.id);
      status = "Audio downloaded from OneDrive and saved locally";
    } catch (reason: unknown) {
      error = `Could not import audio link: ${toErrorMessage(reason)}`;
    } finally {
      loading = false;
    }
  }

  async function processAudio() {
    if (processDisabled || !selectedSource) return;
    processing = true;
    error = null;
    status = null;
    try {
      let job = await invoke<RadcastJobStatus>("start_radcast_audio", {
        request: {
          project_id: selectedProjectId,
          source_id: selectedSource.id,
          output_format: outputFormat,
          clip_start_seconds: clipStart,
          clip_end_seconds: clipEnd,
          cleanup_enabled: cleanupEnabled,
          max_silence_seconds: shortenPauses ? clampRadcastSilenceSeconds(maxSilenceSeconds) : null,
          caption_format: captionFormat,
          caption_language: captionLanguage,
          caption_quality_mode: captionQualityMode,
          caption_glossary: captionGlossary.trim() || null,
          enhancement_model: enhancementModel,
          enhancement_quality: enhancementQuality,
          remove_filler_words: removeFillerWords,
          filler_removal_mode: fillerRemovalMode,
        },
      });
      radcastJob = job;
      while (job.state === "running") {
        await new Promise((resolve) => window.setTimeout(resolve, 400));
        job = await invoke<RadcastJobStatus>("get_radcast_audio_job", { jobId: job.id });
        radcastJob = job;
      }
      if (job.state === "failed") {
        throw new Error(job.error ?? "The local audio job failed.");
      }
      if (job.state === "cancelled") {
        status = "Audio processing cancelled";
        radcastJob = null;
        return;
      }
      const output = job.output;
      if (!output) throw new Error("The local audio job completed without an output file.");
      outputs = [output, ...outputs];
      const captionDetail = output.caption_format
        ? ` with ${output.caption_segment_count} ${output.caption_format.toUpperCase()} caption${output.caption_segment_count === 1 ? "" : "s"}`
        : "";
      const captionReviewDetail = output.caption_review_required
        ? `; ${output.caption_low_confidence_segments} caption line${output.caption_low_confidence_segments === 1 ? "" : "s"} flagged for review`
        : "";
      const fillerDetail = output.removed_filler_count > 0
        ? ` and removed ${output.removed_filler_count} filler word${output.removed_filler_count === 1 ? "" : "s"}`
        : "";
      status = `Audio processing complete${captionDetail}${captionReviewDetail}${fillerDetail}`;
      radcastJob = null;
    } catch (reason: unknown) {
      status = null;
      error = `Could not process audio: ${toErrorMessage(reason)}`;
      radcastJob = null;
    } finally {
      processing = false;
    }
  }

  async function deleteSource() {
    if (!selectedSource || loading || processing || deletingSource) return;
    if (!window.confirm(`Remove ${selectedSource.original_filename} from this project?`)) return;

    deletingSource = true;
    error = null;
    try {
      await invoke<void>("delete_radcast_audio", {
        request: {
          project_id: selectedProjectId,
          source_id: selectedSource.id,
        },
      });
      status = "Saved source removed";
      selectedSourceId = null;
      await refreshAudio();
    } catch (reason: unknown) {
      error = `Could not remove source audio: ${toErrorMessage(reason)}`;
    } finally {
      deletingSource = false;
    }
  }

  async function cancelProcessing() {
    if (!radcastJob || !processing || cancelling) return;
    cancelling = true;
    error = null;
    try {
      radcastJob = await invoke<RadcastJobStatus>("cancel_radcast_audio", { jobId: radcastJob.id });
      status = "Cancelling local audio processing...";
    } catch (reason: unknown) {
      error = `Could not cancel audio processing: ${toErrorMessage(reason)}`;
    } finally {
      cancelling = false;
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
  {#if processing && radcastJob}
    <div class="radcast-progress" aria-live="polite">
      <div class="radcast-progress-heading">
        <strong>{processingPhaseLabel(radcastJob.phase)}</strong>
        <span>{radcastJob.percent}%</span>
      </div>
      <progress max="100" value={radcastJob.percent}></progress>
      <small>Elapsed {formatElapsed(radcastJob.elapsed_seconds)}. Everything is being processed locally.</small>
      <button class="secondary-button compact-button" type="button" disabled={cancelling} onclick={() => void cancelProcessing()}>
        {cancelling ? "Cancelling..." : "Cancel processing"}
      </button>
    </div>
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
        <div class="radcast-source-controls">
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
          <button
            class="secondary-button compact-button"
            type="button"
            disabled={loading || processing || deletingSource}
            onclick={() => void deleteSource()}
            title="Remove the selected saved source from this project"
          >
            {deletingSource ? "Removing..." : "Remove source"}
          </button>
        </div>
      {:else}
        <div class="radcast-empty">No audio has been added to this project.</div>
      {/if}

      <div class="radcast-link-import">
        <label class="stack" for="radcast-source-link">
          <span>OneDrive or SharePoint link</span>
          <input
            id="radcast-source-link"
            type="url"
            bind:value={sourceLink}
            placeholder="Paste a sharing link"
            disabled={loading || processing}
          />
        </label>
        <button
          class="secondary-button compact-button"
          type="button"
          disabled={loading || processing || !sourceLink.trim()}
          onclick={() => void importSourceFromLink()}
        >
          Import link
        </button>
        <small class="field-note">RADsuite downloads an accessible file, then keeps processing local.</small>
      </div>

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
        <span>Enhancement profile</span>
        <select bind:value={enhancementModel}>
          {#each captionCapability.enhancement_models as model}
            <option value={model.id} disabled={!model.available}>{model.label}{model.available ? "" : " (not installed)"}</option>
          {/each}
        </select>
        <small class="field-note">{selectedEnhancementCapability?.description ?? "Checking local enhancement support."}</small>
        <small class="field-note">{selectedEnhancementCapability?.detail ?? "Checking local enhancement support."}</small>
      </label>
      <label class="stack settings-compact-field">
        <span>Enhancement quality</span>
        <select bind:value={enhancementQuality} disabled={enhancementModel === "none"}>
          <option value="fast">Fast · best for short clips</option>
          <option value="standard">Standard · balanced</option>
          <option value="high">High · maximum cleanup</option>
        </select>
        <small class="field-note">High uses the original RADcast Optimized quality; Fast and Standard reduce processing time.</small>
      </label>
      <label class="stack settings-compact-field">
        <span>Output format</span>
        <select bind:value={outputFormat}>
          <option value="mp3">MP3</option>
          <option value="wav">WAV</option>
        </select>
      </label>
      <label class="stack settings-compact-field">
        <span>Closed captions</span>
        <select
          value={captionFormat ?? ""}
          disabled={!captionCapability.caption_available}
          onchange={(event) => {
            const value = (event.currentTarget as HTMLSelectElement).value;
            captionFormat = value === "" ? null : (value as CaptionFormat);
          }}
        >
          <option value="">Do not generate</option>
          <option value="srt">SRT</option>
          <option value="vtt">VTT</option>
        </select>
        <small class="field-note">{captionCapability.caption_detail}</small>
      </label>
      {#if captionFormat}
        <label class="stack settings-compact-field">
          <span>Caption language</span>
          <select bind:value={captionLanguage}>
            <option value="en">English</option>
            <option value="mi">Maori</option>
            <option value="auto">Auto-detect</option>
          </select>
        </label>
        <label class="stack settings-compact-field">
          <span>Caption quality</span>
          <select bind:value={captionQualityMode}>
            <option value="fast">Fast</option>
            <option value="accurate">Accurate</option>
            <option value="reviewed">Reviewed</option>
          </select>
          <small class="field-note">Reviewed uses the strongest local model and search settings available.</small>
        </label>
        <label class="stack settings-compact-field">
          <span>Glossary and names</span>
          <textarea bind:value={captionGlossary} rows="3" placeholder="Māori terms, names, or spellings"></textarea>
          <small class="field-note">Optional terms passed to the transcription model as phrase guidance.</small>
        </label>
      {/if}
      <label class="radcast-check">
        <input type="checkbox" bind:checked={cleanupEnabled} />
        <span>
          <strong>Clean up audio</strong>
          <small>Noise reduction and speech-focused loudness balancing.</small>
        </span>
      </label>
      <label class="radcast-check">
        <input type="checkbox" bind:checked={shortenPauses} />
        <span>
          <strong>Shorten long pauses</strong>
          <small>Keep a controlled amount of silence between spoken sections.</small>
        </span>
      </label>
      {#if shortenPauses}
        <div class="settings-compact-field radcast-range-row">
          <div class="radcast-range-label">
            <span>Keep each pause up to</span>
            <strong>{formatRadcastSilenceSeconds(maxSilenceSeconds)}</strong>
          </div>
          <input
            type="range"
            min="0"
            max="4"
            step="0.25"
            value={maxSilenceSeconds}
            aria-label="Keep each pause up to"
            oninput={(event) => {
              maxSilenceSeconds = clampRadcastSilenceSeconds(
                (event.currentTarget as HTMLInputElement).value,
              );
            }}
          />
        </div>
      {/if}
      <label class="radcast-check">
        <input type="checkbox" bind:checked={removeFillerWords} disabled={!captionCapability.caption_available} />
        <span>
          <strong>Remove filler words</strong>
          <small>Remove recognised ums, uhs, and similar speech fillers.</small>
        </span>
      </label>
      {#if removeFillerWords}
        <label class="stack settings-compact-field">
          <span>Filler removal</span>
          <select bind:value={fillerRemovalMode}>
            <option value="normal">Normal</option>
            <option value="aggressive">Aggressive</option>
          </select>
        </label>
      {/if}
      <div class="radcast-processing-note">
        <span class={selectedEnhancementCapability?.available ? "status-dot is-ready" : "status-dot"}></span>
        <span>{selectedEnhancementCapability?.label ?? "Local enhancement"} runs on this computer without a server.</span>
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
              <span>{output.output_format.toUpperCase()} · {formatDuration(output.duration_seconds)}{output.enhancement_model !== "none" ? ` · ${enhancementModelLabel(output.enhancement_model)} · ${enhancementQualityLabel(output.enhancement_quality)}` : ""}{output.cleanup_enabled ? " · Cleaned" : ""}{output.max_silence_seconds ? ` · Pauses ≤ ${output.max_silence_seconds}s` : ""}{output.removed_filler_count > 0 ? ` · ${output.removed_filler_count} fillers removed` : ""}{output.caption_review_required ? ` · Review ${output.caption_low_confidence_segments} caption line${output.caption_low_confidence_segments === 1 ? "" : "s"}` : ""}</span>
            </div>
            <audio controls src={convertFileSrc(output.path)}>
              Your browser does not support audio playback.
            </audio>
            <div class="radcast-output-actions">
              <a class="secondary-button compact-button" href={convertFileSrc(output.path)} download={output.filename}>Download audio</a>
              {#if output.caption_path && output.caption_format}
                <a
                  class="secondary-button compact-button"
                  href={convertFileSrc(output.caption_path)}
                  download={`${output.filename}.${output.caption_format}`}
                >Download {output.caption_format.toUpperCase()}</a>
              {/if}
              {#if output.caption_review_path}
                <a
                  class="secondary-button compact-button"
                  href={convertFileSrc(output.caption_review_path)}
                  download={`${output.filename}.review.txt`}
                >Download caption review</a>
              {/if}
            </div>
          </article>
        {/each}
      </div>
    {:else}
      <div class="radcast-empty">Your processed versions will appear here.</div>
    {/if}
  </section>
</section>
