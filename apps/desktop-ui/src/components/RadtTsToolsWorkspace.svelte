<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-dialog";
  import type {
    RadtTsCapabilityStatus,
    RadtTsMediaJobStatus,
    RadtTsMediaOutput,
  } from "../types";
  import {
    buildClipRequest,
    buildTranscriptionRequest,
    canStartClip,
    canStartTranscription,
    type RadtTsClipDraft,
    type RadtTsTranscriptionDraft,
  } from "../lib/radtTsToolsWorkflow";

  type Props = {
    selectedProjectId: string | null;
  };

  let { selectedProjectId }: Props = $props();
  let capability = $state<RadtTsCapabilityStatus>({
    available: false,
    executable: null,
    detail: "Checking local RADTTS support.",
  });
  let outputs = $state<RadtTsMediaOutput[]>([]);
  let transcription = $state<RadtTsTranscriptionDraft>({
    audioPath: "",
    name: "lecture-transcript",
    model: "small",
    language: "",
    beamSize: 5,
  });
  let clip = $state<RadtTsClipDraft>({
    audioPath: "",
    segmentsJsonPath: "",
    outputName: "lesson-clip",
    boundaryMode: "phrases",
    startPhrase: "",
    endPhrase: "",
    startTime: 0,
    endTime: 30,
    verificationMode: "strict",
    outputFormat: "mp3",
  });
  let loading = $state(false);
  let processing = $state(false);
  let cancelling = $state(false);
  let job = $state<RadtTsMediaJobStatus | null>(null);
  let error = $state<string | null>(null);
  let status = $state<string | null>(null);

  let transcriptOutputs = $derived(
    outputs.filter((output) => output.kind === "transcription"),
  );
  let canTranscribe = $derived(
    !processing && canStartTranscription(transcription, capability),
  );
  let canClip = $derived(!processing && canStartClip(clip, capability));

  $effect(() => {
    selectedProjectId;
    void refresh();
  });

  function toErrorMessage(reason: unknown): string {
    return reason instanceof Error ? reason.message : String(reason);
  }

  function phaseLabel(value: RadtTsMediaJobStatus["phase"]): string {
    if (value === "preparing") return "Preparing local media tool";
    if (value === "transcribing") return "Transcribing audio";
    if (value === "extracting_clip") return "Extracting verified clip";
    return "Saving output";
  }

  function outputLabel(output: RadtTsMediaOutput): string {
    return output.kind === "transcription" ? "Transcript" : "Clip";
  }

  async function refresh() {
    loading = true;
    error = null;
    try {
      capability = await invoke<RadtTsCapabilityStatus>("get_radt_ts_capabilities");
      const listing = await invoke<{ outputs: RadtTsMediaOutput[] }>(
        "list_radt_ts_media_outputs",
        { request: { project_id: selectedProjectId } },
      );
      outputs = listing.outputs;
      if (!clip.segmentsJsonPath && listing.outputs.length > 0) {
        const latest = listing.outputs.find((output) => output.kind === "transcription");
        const segments = latest?.artifacts.find((artifact) => artifact.label === "Timed segments");
        if (segments) clip.segmentsJsonPath = segments.path;
      }
    } catch (reason: unknown) {
      error = `Could not load transcription tools: ${toErrorMessage(reason)}`;
    } finally {
      loading = false;
    }
  }

  async function chooseAudio(target: "transcription" | "clip") {
    error = null;
    try {
      const selected = await open({
        multiple: false,
        directory: false,
        filters: [{ name: "Audio files", extensions: ["wav", "mp3", "m4a", "flac", "ogg", "webm", "aac"] }],
      });
      const path = typeof selected === "string" ? selected : selected?.[0];
      if (!path) return;
      if (target === "transcription") transcription.audioPath = path;
      else clip.audioPath = path;
    } catch (reason: unknown) {
      error = `Could not choose audio: ${toErrorMessage(reason)}`;
    }
  }

  async function chooseSegments() {
    error = null;
    try {
      const selected = await open({
        multiple: false,
        directory: false,
        filters: [{ name: "Transcript segments", extensions: ["json"] }],
      });
      const path = typeof selected === "string" ? selected : selected?.[0];
      if (path) clip.segmentsJsonPath = path;
    } catch (reason: unknown) {
      error = `Could not choose transcript segments: ${toErrorMessage(reason)}`;
    }
  }

  async function finishJob(initial: RadtTsMediaJobStatus): Promise<RadtTsMediaOutput | null> {
    let current = initial;
    job = current;
    while (current.state === "starting" || current.state === "running") {
      await new Promise((resolve) => window.setTimeout(resolve, 500));
      current = await invoke<RadtTsMediaJobStatus>("get_radt_ts_media_job", { jobId: current.id });
      job = current;
    }
    if (current.state === "failed") {
      throw new Error(current.error ?? "The local media job failed.");
    }
    if (current.state === "cancelled") {
      status = "RADTTS processing cancelled";
      return null;
    }
    if (!current.output) throw new Error("RADTTS completed without an output file.");
    outputs = [current.output, ...outputs.filter((item) => item.id !== current.output?.id)];
    status = `${outputLabel(current.output)} created locally`;
    return current.output;
  }

  async function startTranscription() {
    if (!canTranscribe) {
      error = "Choose an audio file, enter a transcript name, and check the local RADTTS runtime.";
      return;
    }
    processing = true;
    error = null;
    status = null;
    try {
      const initial = await invoke<RadtTsMediaJobStatus>("start_radt_ts_transcription", {
        request: buildTranscriptionRequest(transcription, selectedProjectId),
      });
      const output = await finishJob(initial);
      const segments = output?.kind === "transcription"
        ? output.artifacts.find((artifact) => artifact.label === "Timed segments")
        : null;
      if (segments) {
        clip.segmentsJsonPath = segments.path;
        if (!clip.audioPath) clip.audioPath = transcription.audioPath;
      }
    } catch (reason: unknown) {
      error = `Could not transcribe audio: ${toErrorMessage(reason)}`;
    } finally {
      processing = false;
      job = null;
    }
  }

  async function startClip() {
    if (!canClip) {
      error = "Choose audio and transcript segments, then provide both clip boundaries.";
      return;
    }
    processing = true;
    error = null;
    status = null;
    try {
      const initial = await invoke<RadtTsMediaJobStatus>("start_radt_ts_clip", {
        request: buildClipRequest(clip, selectedProjectId),
      });
      await finishJob(initial);
    } catch (reason: unknown) {
      error = `Could not create clip: ${toErrorMessage(reason)}`;
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
      job = await invoke<RadtTsMediaJobStatus>("cancel_radt_ts_media_job", { jobId: job.id });
      status = "Cancelling RADTTS processing...";
    } catch (reason: unknown) {
      error = `Could not cancel RADTTS processing: ${toErrorMessage(reason)}`;
    } finally {
      cancelling = false;
    }
  }
</script>

<section class="radtts-tools-workspace" aria-labelledby="radtts-tools-heading">
  <div class="workspace-header">
    <div>
      <p class="eyebrow">RADTTS</p>
      <h2 id="radtts-tools-heading">Transcribe &amp; clip</h2>
    </div>
    <button class="secondary-button compact-button" type="button" disabled={loading || processing} onclick={() => void refresh()}>
      Refresh
    </button>
  </div>

  {#if error}<div class="notice analysis-notice" role="alert">{error}</div>{/if}
  {#if status}<div class="notice radtts-status" aria-live="polite">{status}</div>{/if}
  {#if !capability.available}<div class="notice radtts-capability" role="status">{capability.detail}</div>{/if}
  {#if processing && job}
    <div class="radtts-progress" aria-live="polite">
      <div class="radtts-progress-heading"><strong>{phaseLabel(job.phase)}</strong><span>Running locally</span></div>
      <progress></progress>
      <small>Transcription and clip extraction run on this computer.</small>
      <button class="secondary-button compact-button" type="button" disabled={cancelling} onclick={() => void cancel()}>
        {cancelling ? "Cancelling..." : "Cancel processing"}
      </button>
    </div>
  {/if}

  <div class="radtts-tools-grid">
    <section class="radtts-panel" aria-labelledby="radtts-transcribe-heading">
      <div class="radtts-panel-heading"><div><p class="eyebrow">Speech to text</p><h3 id="radtts-transcribe-heading">Create a transcript</h3></div></div>
      <label class="stack"><span>Audio file</span><div class="radtts-reference-row"><input type="text" bind:value={transcription.audioPath} placeholder="Choose a lecture or recording" /><button class="secondary-button compact-button" type="button" disabled={processing} onclick={() => void chooseAudio("transcription")}>Choose audio</button></div></label>
      <label class="stack"><span>Transcript name</span><input type="text" bind:value={transcription.name} placeholder="lecture-1" /></label>
      <div class="radtts-tools-fields">
        <label class="stack"><span>Model</span><select bind:value={transcription.model}><option value="tiny">Tiny</option><option value="base">Base</option><option value="small">Small</option><option value="medium">Medium</option><option value="large-v3">Large v3</option></select></label>
        <label class="stack"><span>Language</span><input type="text" bind:value={transcription.language} placeholder="Auto-detect" /></label>
        <label class="stack"><span>Beam size</span><input type="number" min="1" max="10" step="1" bind:value={transcription.beamSize} /></label>
      </div>
      <small class="field-note">The transcript includes plain text, SRT captions, and timed segments for clip extraction.</small>
      <button class="primary-button radtts-process-button" type="button" disabled={!canTranscribe} onclick={() => void startTranscription()}>Create transcript</button>
    </section>

    <section class="radtts-panel" aria-labelledby="radtts-clip-heading">
      <div class="radtts-panel-heading"><div><p class="eyebrow">Transcript-verified editing</p><h3 id="radtts-clip-heading">Extract a clip</h3></div></div>
      <label class="stack"><span>Audio file</span><div class="radtts-reference-row"><input type="text" bind:value={clip.audioPath} placeholder="Choose the original recording" /><button class="secondary-button compact-button" type="button" disabled={processing} onclick={() => void chooseAudio("clip")}>Choose audio</button></div></label>
      <label class="stack"><span>Timed segments JSON</span><div class="radtts-reference-row"><select bind:value={clip.segmentsJsonPath}><option value="">Choose a transcript</option>{#each transcriptOutputs as output (output.id)}{#each output.artifacts.filter((artifact) => artifact.label === "Timed segments") as artifact (artifact.path)}<option value={artifact.path}>{output.name}</option>{/each}{/each}</select><button class="secondary-button compact-button" type="button" disabled={processing} onclick={() => void chooseSegments()}>Choose file</button></div></label>
      <label class="stack"><span>Clip name</span><input type="text" bind:value={clip.outputName} placeholder="opening-section" /></label>
      <div class="segmented-control" aria-label="Clip boundary mode"><button class:is-selected={clip.boundaryMode === "phrases"} type="button" onclick={() => (clip.boundaryMode = "phrases")}>Phrases</button><button class:is-selected={clip.boundaryMode === "times"} type="button" onclick={() => (clip.boundaryMode = "times")}>Exact times</button></div>
      {#if clip.boundaryMode === "phrases"}
        <div class="radtts-boundary-fields"><label class="stack"><span>Start phrase</span><input type="text" bind:value={clip.startPhrase} placeholder="In this section" /></label><label class="stack"><span>End phrase</span><input type="text" bind:value={clip.endPhrase} placeholder="Let us move on" /></label></div>
      {:else}
        <div class="radtts-boundary-fields"><label class="stack"><span>Start seconds</span><input type="number" min="0" step="0.1" bind:value={clip.startTime} /></label><label class="stack"><span>End seconds</span><input type="number" min="0.1" step="0.1" bind:value={clip.endTime} /></label></div>
      {/if}
      <div class="radtts-tools-fields"><label class="stack"><span>Verification</span><select bind:value={clip.verificationMode}><option value="strict">Strict</option><option value="lenient">Lenient</option></select></label><label class="stack"><span>Format</span><select bind:value={clip.outputFormat}><option value="mp3">MP3</option><option value="wav">WAV</option></select></label></div>
      <small class="field-note">Phrase boundaries snap to recognised transcript segments and include a small speech-safe margin.</small>
      <button class="primary-button radtts-process-button" type="button" disabled={!canClip} onclick={() => void startClip()}>Create verified clip</button>
    </section>
  </div>

  <section class="radtts-outputs" aria-labelledby="radtts-tools-output-heading">
    <div class="radtts-panel-heading"><div><p class="eyebrow">Project outputs</p><h3 id="radtts-tools-output-heading">Transcripts and clips</h3></div><span>{outputs.length} item{outputs.length === 1 ? "" : "s"}</span></div>
    {#if outputs.length}
      <div class="radtts-output-list">
        {#each outputs as output (output.id)}
          <article class="radtts-output-row radtts-tools-output-row">
            <div class="radtts-output-copy"><strong>{output.name}</strong><span>{outputLabel(output)}{output.output_format ? ` · ${output.output_format.toUpperCase()}` : ""}</span>{#each output.warnings as warning}<small>{warning}</small>{/each}</div>
            {#if output.kind === "clip"}<audio controls src={convertFileSrc(output.primary_path)}>Your browser does not support audio playback.</audio>{/if}
            <div class="radtts-output-actions"><a class="secondary-button compact-button" href={convertFileSrc(output.primary_path)} download>Download {output.kind === "transcription" ? "transcript" : "clip"}</a>{#each output.artifacts as artifact (artifact.path)}<a class="secondary-button compact-button" href={convertFileSrc(artifact.path)} download>{artifact.label}</a>{/each}</div>
          </article>
        {/each}
      </div>
    {:else}<div class="radtts-empty">Your transcripts and clips will appear here.</div>{/if}
  </section>
</section>
