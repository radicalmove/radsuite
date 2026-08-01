<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-dialog";
  import {
    browserStorage,
    readRadtTsProjectPreferences,
    writeRadtTsProjectPreferences,
    type StorageLike,
  } from "../lib/storage";
  import type {
    RadtTsAudioOutput,
    RadtTsCapabilityStatus,
    RadtTsChunkMode,
    RadtTsJobStatus,
    RadtTsOutputFormat,
    RadtTsQuality,
  } from "../types";
  import {
    buildRadtTsRequest,
    canStartRadtTs,
    clampRadtTsMaxNewTokens,
    createDefaultRadtTsDraft,
    formatRadtTsMaxNewTokens,
    mergeRadtTsVoicePreferences,
    type RadtTsDraft,
  } from "../lib/radtTsWorkflow";

  type Props = {
    selectedProjectId: string | null;
  };

  let { selectedProjectId }: Props = $props();

  let capability = $state<RadtTsCapabilityStatus>({
    available: false,
    executable: null,
    detail: "Checking local voice generation support.",
  });
  let outputs = $state<RadtTsAudioOutput[]>([]);
  let job = $state<RadtTsJobStatus | null>(null);
  let loading = $state(false);
  let processing = $state(false);
  let cancelling = $state(false);
  let error = $state<string | null>(null);
  let status = $state<string | null>(null);
  let preferenceStorage = $state<StorageLike | null>(null);
  let settingsLoaded = $state(false);
  let loadedProjectId = $state<string | null>(null);
  let settingsSaveTimer: number | null = null;
  let draft = $state<RadtTsDraft>(createDefaultRadtTsDraft());
  const builtinSpeakers: Array<{ id: string; label: string; language: string }> = [
    { id: "Aiden", label: "Aiden", language: "English" },
    { id: "Dylan", label: "Dylan", language: "Chinese" },
    { id: "Eric", label: "Eric", language: "Chinese" },
    { id: "Ono_Anna", label: "Ono Anna", language: "Japanese" },
    { id: "Ryan", label: "Ryan", language: "English" },
    { id: "Serena", label: "Serena", language: "Chinese" },
    { id: "Sohee", label: "Sohee", language: "Korean" },
    { id: "Uncle_Fu", label: "Uncle Fu", language: "Chinese" },
    { id: "Vivian", label: "Vivian", language: "Chinese" },
  ];

  let startDisabled = $derived(
    processing || !canStartRadtTs(draft, capability),
  );

  $effect(() => {
    selectedProjectId;
    settingsLoaded = false;
    void refresh();
  });

  $effect(() => {
    const projectId = selectedProjectId;
    if (!settingsLoaded || !projectId || processing) return;
    const preferences = {
      voice: {
        voiceSource: draft.voiceSource,
        referenceAudioPath: draft.referenceAudioPath,
        referenceText: draft.referenceText,
        builtInSpeaker: draft.builtInSpeaker,
        builtInInstruct: draft.builtInInstruct,
        quality: draft.quality,
        chunkMode: draft.chunkMode,
        pauseMinSeconds: draft.pauseMinSeconds,
        pauseMaxSeconds: draft.pauseMaxSeconds,
        pauseSeed: draft.pauseSeed,
        maxNewTokens: draft.maxNewTokens,
        outputFormat: draft.outputFormat,
        outputName: draft.outputName,
      },
    };
    if (settingsSaveTimer !== null) window.clearTimeout(settingsSaveTimer);
    settingsSaveTimer = window.setTimeout(() => {
      settingsSaveTimer = null;
      writeRadtTsProjectPreferences(preferenceStorage, projectId, preferences);
    }, 500);
  });

  function toErrorMessage(reason: unknown): string {
    return reason instanceof Error ? reason.message : String(reason);
  }

  function formatDuration(seconds: number | null): string {
    if (seconds === null || !Number.isFinite(seconds)) return "Duration pending";
    const safe = Math.max(0, Math.floor(seconds));
    return `${Math.floor(safe / 60)}:${(safe % 60).toString().padStart(2, "0")}`;
  }

  function phaseLabel(value: RadtTsJobStatus["phase"]): string {
    if (value === "preparing") return "Preparing voice generation";
    if (value === "generating") return "Generating locally";
    return "Saving output";
  }

  async function refresh() {
    loading = true;
    settingsLoaded = false;
    error = null;
    try {
      preferenceStorage = browserStorage();
      const preferences = readRadtTsProjectPreferences(preferenceStorage, selectedProjectId);
      const baseDraft = loadedProjectId === selectedProjectId
        ? draft
        : createDefaultRadtTsDraft();
      draft = mergeRadtTsVoicePreferences(baseDraft, preferences.voice);
      loadedProjectId = selectedProjectId;
      capability = await invoke<RadtTsCapabilityStatus>("get_radt_ts_capabilities");
      const listing = await invoke<{ outputs: RadtTsAudioOutput[] }>("list_radt_ts_outputs", {
        request: { project_id: selectedProjectId },
      });
      outputs = listing.outputs;
      settingsLoaded = true;
    } catch (reason: unknown) {
      error = `Could not load voice generation: ${toErrorMessage(reason)}`;
    } finally {
      loading = false;
    }
  }

  async function chooseReferenceAudio() {
    error = null;
    try {
      const selected = await open({
        multiple: false,
        directory: false,
        filters: [
          {
            name: "Reference audio",
            extensions: ["wav", "mp3", "m4a", "flac", "ogg", "webm", "aac"],
          },
        ],
      });
      const path = typeof selected === "string" ? selected : selected?.[0];
      if (path) draft.referenceAudioPath = path;
    } catch (reason: unknown) {
      error = `Could not choose reference audio: ${toErrorMessage(reason)}`;
    }
  }

  async function synthesize() {
    if (startDisabled) {
      error = draft.voiceSource === "builtin"
        ? "Enter a script, choose a built-in speaker, and check the pause range."
        : "Enter a script, choose reference audio, authorize voice cloning, and check the pause range.";
      return;
    }

    processing = true;
    error = null;
    status = null;
    try {
      let current = await invoke<RadtTsJobStatus>("start_radt_ts_synthesis", {
        request: buildRadtTsRequest(draft, selectedProjectId),
      });
      job = current;
      while (current.state === "starting" || current.state === "running") {
        await new Promise((resolve) => window.setTimeout(resolve, 500));
        current = await invoke<RadtTsJobStatus>("get_radt_ts_job", { jobId: current.id });
        job = current;
      }
      if (current.state === "failed") {
        throw new Error(current.error ?? "The local voice generation job failed.");
      }
      if (current.state === "cancelled") {
        status = "Voice generation cancelled";
        return;
      }
      if (!current.output) {
        throw new Error("Voice generation completed without an output file.");
      }
      outputs = [current.output, ...outputs.filter((item) => item.id !== current.output?.id)];
      status = "Voice generation complete. Captions are available with the output.";
    } catch (reason: unknown) {
      error = `Could not generate voice audio: ${toErrorMessage(reason)}`;
    } finally {
      processing = false;
      job = null;
    }
  }

  async function cancel() {
    if (!job || !processing || cancelling) return;
    cancelling = true;
    error = null;
    try {
      job = await invoke<RadtTsJobStatus>("cancel_radt_ts_job", { jobId: job.id });
      status = "Cancelling local voice generation...";
    } catch (reason: unknown) {
      error = `Could not cancel voice generation: ${toErrorMessage(reason)}`;
    } finally {
      cancelling = false;
    }
  }
</script>

<section class="radtts-workspace" aria-labelledby="radtts-heading">
  <div class="workspace-header">
    <div>
      <p class="eyebrow">RADTTS</p>
      <h2 id="radtts-heading">Voice generation</h2>
    </div>
    <button class="secondary-button compact-button" type="button" disabled={loading || processing} onclick={() => void refresh()}>
      Refresh
    </button>
  </div>

  {#if error}
    <div class="notice analysis-notice" role="alert">{error}</div>
  {/if}
  {#if status}
    <div class="notice radtts-status" aria-live="polite">{status}</div>
  {/if}
  {#if !capability.available}
    <div class="notice radtts-capability" role="status">{capability.detail}</div>
  {/if}
  {#if processing && job}
    <div class="radtts-progress" aria-live="polite">
      <div class="radtts-progress-heading">
        <strong>{phaseLabel(job.phase)}</strong>
        <span>Running locally</span>
      </div>
      <progress></progress>
      <small>RADTTS does not expose percentage progress yet, so this status remains indeterminate.</small>
      <button class="secondary-button compact-button" type="button" disabled={cancelling} onclick={() => void cancel()}>
        {cancelling ? "Cancelling..." : "Cancel generation"}
      </button>
    </div>
  {/if}

  <div class="radtts-grid">
    <section class="radtts-panel" aria-labelledby="radtts-script-heading">
      <div class="radtts-panel-heading">
        <div>
          <p class="eyebrow">Script</p>
          <h3 id="radtts-script-heading">What should be spoken?</h3>
        </div>
      </div>
      <label class="stack">
        <span>Script text</span>
        <textarea rows="15" bind:value={draft.text} placeholder="Paste the narration script here."></textarea>
      </label>
      <label class="stack">
        <span>Output name</span>
        <input type="text" bind:value={draft.outputName} placeholder="lesson-intro" />
      </label>
    </section>

    <section class="radtts-panel" aria-labelledby="radtts-settings-heading">
      <div class="radtts-panel-heading">
        <div>
          <p class="eyebrow">Voice and settings</p>
          <h3 id="radtts-settings-heading">Create a local version</h3>
        </div>
      </div>
      <label class="stack">
        <span>Voice source</span>
        <select bind:value={draft.voiceSource}>
          <option value="reference">Authorised reference voice</option>
          <option value="builtin">Built-in voice</option>
        </select>
      </label>
      {#if draft.voiceSource === "reference"}
        <label class="stack">
          <span>Reference voice audio</span>
          <div class="radtts-reference-row">
            <input type="text" bind:value={draft.referenceAudioPath} placeholder="Choose a clear voice sample" />
            <button class="secondary-button compact-button" type="button" disabled={processing} onclick={() => void chooseReferenceAudio()}>
              Choose audio
            </button>
          </div>
          <small class="field-note">Use a clear sample of the voice you are authorised to reproduce.</small>
        </label>
        <label class="stack">
          <span>Reference transcript (optional)</span>
          <textarea
            rows="3"
            bind:value={draft.referenceText}
            placeholder="Type the words spoken in the reference audio"
          ></textarea>
          <small class="field-note">This can improve pronunciation and timing when the voice sample contains a known script.</small>
        </label>
        <label class="radtts-check">
          <input type="checkbox" bind:checked={draft.acknowledgeVoiceClone} />
          <span>
            <strong>I have permission to use this voice</strong>
            <small>Required before reference-voice synthesis can start.</small>
          </span>
        </label>
      {:else}
        <label class="stack">
          <span>Built-in speaker</span>
          <select bind:value={draft.builtInSpeaker}>
            {#each builtinSpeakers as speaker (speaker.id)}
              <option value={speaker.id}>{speaker.label} · {speaker.language}</option>
            {/each}
          </select>
          <small class="field-note">Uses RADTTS CustomVoice models. No reference recording or voice-clone permission is required.</small>
        </label>
        <label class="stack">
          <span>Voice instruction (optional)</span>
          <textarea
            rows="3"
            bind:value={draft.builtInInstruct}
            placeholder="For example: warm, clear, and measured"
          ></textarea>
          <small class="field-note">Describe the delivery style you want the built-in speaker to use.</small>
        </label>
      {/if}
      <label class="stack">
        <span>Quality</span>
        <select bind:value={draft.quality}>
          <option value="fast">Fast</option>
          <option value="high">High quality</option>
        </select>
      </label>
      <label class="stack">
        <span>Chunking</span>
        <select bind:value={draft.chunkMode}>
          <option value="sentence">Sentence pauses</option>
          <option value="single">Single continuous passage</option>
        </select>
      </label>
      <div class="radtts-pause-fields">
        <label class="stack">
          <span>Shortest pause</span>
          <input type="number" min="0.1" step="0.05" bind:value={draft.pauseMinSeconds} />
        </label>
        <label class="stack">
          <span>Longest pause</span>
          <input type="number" min="0.1" step="0.05" bind:value={draft.pauseMaxSeconds} />
        </label>
      </div>
      <label class="stack">
        <span>Pause pattern seed</span>
        <input
          type="number"
          step="1"
          value={draft.pauseSeed}
          placeholder="Random each time"
          oninput={(event) => {
            draft.pauseSeed = (event.currentTarget as HTMLInputElement).value;
          }}
        />
        <small class="field-note">Optional. Enter an integer to repeat the same sentence-pause pattern.</small>
      </label>
      <div class="settings-compact-field radtts-range-row">
        <div class="radtts-range-label">
          <span>Generation budget</span>
          <strong>{formatRadtTsMaxNewTokens(draft.maxNewTokens)}</strong>
        </div>
        <input
          type="range"
          min="64"
          max="8192"
          step="1"
          value={draft.maxNewTokens}
          aria-label="Generation budget"
          oninput={(event) => {
            draft.maxNewTokens = clampRadtTsMaxNewTokens(
              (event.currentTarget as HTMLInputElement).value,
            );
          }}
        />
        <small class="field-note">A larger budget supports longer scripts but may take longer to generate locally.</small>
      </div>
      <label class="stack">
        <span>Output format</span>
        <select bind:value={draft.outputFormat}>
          <option value="mp3">MP3</option>
          <option value="wav">WAV</option>
        </select>
      </label>
      <div class="radtts-processing-note">
        <span class="status-dot" class:is-ready={capability.available}></span>
        <span>{capability.available ? "The RADTTS engine will run entirely on this computer." : capability.detail}</span>
      </div>
      <button class="primary-button radtts-process-button" type="button" disabled={startDisabled} onclick={() => void synthesize()}>
        {processing ? "Generating" : "Generate voice audio"}
      </button>
    </section>
  </div>

  <section class="radtts-outputs" aria-labelledby="radtts-output-heading">
    <div class="radtts-panel-heading">
      <div>
        <p class="eyebrow">Project outputs</p>
        <h3 id="radtts-output-heading">Generated versions</h3>
      </div>
      <span>{outputs.length} version{outputs.length === 1 ? "" : "s"}</span>
    </div>
    {#if outputs.length}
      <div class="radtts-output-list">
        {#each outputs as output (output.id)}
          <article class="radtts-output-row">
            <div class="radtts-output-copy">
              <strong>{output.filename}</strong>
              <span>{output.output_format.toUpperCase()} · {formatDuration(output.duration_seconds)}</span>
            </div>
            <audio controls src={convertFileSrc(output.path)}>
              Your browser does not support audio playback.
            </audio>
            <div class="radtts-output-actions">
              <a class="secondary-button compact-button" href={convertFileSrc(output.path)} download={output.filename}>Download audio</a>
              {#each output.caption_paths as captionPath (captionPath)}
                <a class="secondary-button compact-button" href={convertFileSrc(captionPath)} download>Download captions</a>
              {/each}
            </div>
          </article>
        {/each}
      </div>
    {:else}
      <div class="radtts-empty">Your generated voice versions will appear here.</div>
    {/if}
  </section>
</section>
