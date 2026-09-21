# RADcast and RADTTS MP4 Export Design

## Problem

RADsuite currently exports audio from RADcast and RADTTS as MP3 or WAV. Echo360 can present a richer media tile and playback experience when the uploaded asset is an MP4 with a thumbnail/video stream, rather than an audio-only file. Users need an optional local export that combines a chosen course image with generated or cleaned audio without changing the existing audio-only workflow.

## Decision

- Add MP4 as an optional output format in both RADcast and RADTTS.
- Keep MP3 and WAV available with their current defaults, processing paths, and output behavior.
- Use a shared local video-export module for both workflows. It runs after the existing audio-producing step, so RADcast cleanup/trim/caption timing and RADTTS synthesis remain unchanged.
- Build the MP4 as a neutral split composition: the selected presenter image occupies the left 540 pixels (approximately 42% of the frame), while a dark charcoal panel occupies the remaining 740 pixels and contains a subtle audio-responsive white waveform. The final audio is encoded as AAC. Do not use RADsuite red or another course-specific accent.
- Use a standard H.264/AAC MP4 suitable for Echo360 and ordinary desktop media players. The video is fixed at 1280x720, 30 fps, H.264 `libx264` with `yuv420p`, CRF 23, and AAC at 192 kbps. The output ends when the final audio ends.
- Require an image only when MP4 is selected. Audio-only exports must not need an image.
- Support a project-level reusable image plus a per-export override. A user can choose the saved project image, choose a different image for one export, or save the newly selected image as the project image.
- Copy selected images into RADsuite-managed project storage. Outputs must not depend on a Desktop, OneDrive, or other external path remaining available.
- Keep source audio, existing audio outputs, captions, and existing output records intact. A new MP4 is one additional output record for that request, not a replacement for any already-created MP3 or WAV output.

## User workflow

### RADcast

1. The user imports or selects source audio as today.
2. The user applies the existing cleanup, enhancement, trim, pause, filler, and caption settings.
3. The user chooses MP3, WAV, or MP4.
4. When MP4 is selected, a required `Presenter image` panel appears. It asks the user to choose a photograph or avatar of the speaker. If a saved project image exists it is selected automatically; otherwise the panel shows `Choose presenter image` and the create action remains disabled until an image is selected. The panel also offers `Use saved project image`, `Choose a different image`, and `Save this image for future MP4s` actions as applicable.
5. The panel previews the fixed split composition with the selected presenter image on the left and a representative white waveform on the right. RADsuite recommends a clear square, portrait, or presenter-focused image but does not attempt face detection or reject other valid images.
6. The user creates the version. RADsuite renders the final RADcast audio first, then creates the image-backed MP4 from that exact rendered audio.
7. The output list exposes the MP4 as a playable/downloadable video while preserving caption artifacts and any separately-created audio outputs.

### RADTTS

1. The user generates a voice version or creates a verified clip using the existing RADTTS workflow.
2. The user chooses MP3, WAV, or MP4 where the current workflow offers an audio output format.
3. When MP4 is selected, the same required presenter-image prompt, saved-image reuse, per-export override, and split-composition preview are shown.
4. RADsuite lets RADTTS finish its normal audio output, then creates the MP4 from that output.
5. The output listing identifies the video format and keeps captions/timed-segment artifacts available separately.

MP4 applies to both the normal voice-generation path and the verified-clip path wherever that path currently produces an audio output. It does not add video output to metadata-only or non-media actions. Each path uses the same isolated WAV scratch workspace and shared video exporter; only the completed MP4 is added to its normal output listing.

## Architecture

### Shared video exporter

Add a shared Rust-side exporter in the engine/desktop boundary with a request containing:

- an existing audio path;
- an image path;
- an MP4 output path;
- the fixed frame and waveform presentation defaults.

The exporter validates regular files and supported image types, creates the output parent directory, builds an FFmpeg command without shell interpolation, runs it through the existing local FFmpeg resolver, and probes the resulting file. It must verify that the result contains exactly one video stream and exactly one audio stream, that the video is 1280x720 H.264/yuv420p at 30 fps, that the audio is AAC, and that the format duration is finite and greater than zero.

The desktop boundary introduces a media format with `mp3`, `wav`, and `mp4` variants. The existing engine `AudioOutputFormat` remains limited to `Mp3` and `Wav`; MP4 is never passed to the audio processor or the RADTTS CLI as an audio format. For MP4 requests, both workflows render an isolated temporary WAV in a per-job scratch directory, pass that WAV to the video exporter, and do not persist the scratch audio as an output record.

The exporter should use a two-stage workflow rather than embedding the existing cleanup graph inside the video graph:

1. Existing RADcast or RADTTS code produces the final audio artifact.
2. The shared exporter loops and centre-crops the presenter image into the left panel, creates the neutral waveform panel, renders the waveform from the final audio, joins the panels, maps the video and audio streams, and writes the MP4.

The fixed visual constants and filter graph are:

- frame: 1280x720, divided into a 540x720 presenter panel and a 740x720 waveform panel;
- presenter image: scaled with `force_original_aspect_ratio=increase` and centre-cropped to 540x720 so the panel is always filled without stretching;
- waveform panel: solid neutral charcoal (`0x101214`) at 740x720 with no course-specific accent colour;
- waveform: white `showwaves` line, 660x240, `cline` mode, 30 fps, centred in the waveform panel with 40-pixel horizontal margins;
- video: `libx264`, CRF 23, `yuv420p`, 30 fps;
- audio: AAC, 192 kbps;
- duration: `-shortest`, bounded by the final audio stream.

The command uses the image as input 0 with `-loop 1 -framerate 30`, the final audio as input 1, and the following filter graph semantics: scale/crop input 0 to 540x720 as `[presenter]`; create a 740x720 charcoal colour source as `[panel]`; render input 1 with `showwaves=s=660x240:mode=cline:rate=30:colors=white,format=rgba,colorkey=black:0.01:0.0` so the waveform background is transparent; overlay the waveform at `x=40:y=240` on `[panel]`; join `[presenter]` and the completed waveform panel with `hstack`; apply `fps=30,format=yuv420p`; map the composed video and `1:a:0`; encode with `-c:v libx264 -crf 23 -pix_fmt yuv420p -r 30 -c:a aac -b:a 192k -shortest -movflags +faststart`. The exporter uses fixed 1280x720, 540x720, and 660x240 constants; requests cannot override dimensions or placement in this slice.

The post-export probe invokes `ffprobe -v error -print_format json -show_streams -show_format` and requires exactly one stream with `codec_type=video`, one with `codec_type=audio`, video width 1280, video height 720, video codec `h264`, video pixel format `yuv420p`, video `r_frame_rate=30/1` and `avg_frame_rate=30/1`, audio codec `aac`, and `format.duration` finite and greater than zero. The output duration must be within 0.10 seconds of the intermediate audio duration.

This boundary avoids changing the researched RADcast filter chains or the RADTTS CLI contract. A temporary intermediate audio file may be used when the requested primary output is MP4; it must be cleaned up on success, failure, and cancellation.

When MP4 is selected, both workflows expose a distinct `Rendering video` progress phase between audio generation and manifest persistence. The video process is started with `-progress pipe:1 -nostats`; the exporter parses `out_time_us` against the known intermediate-audio duration. The UI reports 90-99% during this phase and 100% only after the output manifest is persisted. If duration is unavailable, the phase is indeterminate until the probe completes. Audio-only requests retain their current progress phases.

The cancellation-capable path uses a shared spawned-process runner for both audio and video FFmpeg/FFprobe work. The existing `AudioProcessor` public methods keep their current signatures and return types but delegate to this runner with a no-op cancellation token; new cancellation-aware methods are used by RADcast and RADTTS jobs. The runner polls the child, reads progress/output without blocking, and on cancellation terminates the child process tree using a platform-specific process-group helper (Unix process group; Windows new process group and numeric-PID tree termination), then waits for exit before cleanup. No shell command is built from user-provided paths.

### Image storage

Use one shared project-media root for both workflows: `<data_dir>/media/projects/<project_id>/`. Store the reusable project image at `cover/cover.<ext>` and its metadata at `cover/metadata.json`. RADcast and RADTTS must call the same storage helper; neither workflow owns a separate copy of the reusable image.

The storage helper must:

- accept PNG, JPEG, and WebP inputs;
- copy the selected file into managed storage with the stable `cover/cover.<ext>` filename for a saved project image;
- use a temporary UUID-named copy under the same project-media root while rendering a per-export override;
- retain the selected override as an output-scoped copy at `exports/<output_id>/image.<ext>` when it is not saved as the project image, so completed MP4 metadata never points at a deleted file; remove that copy only when the associated MP4 output is deleted;
- reject missing, non-regular, unsupported, or larger-than-50-MB files with an actionable error;
- preserve the existing image when a per-export override is used without saving it as the project image;
- be reusable by RADcast and RADTTS without duplicating copy/validation logic.

The initial implementation does not need image editing, multiple image libraries, animated source images, or remote image URLs.

### Request and output contracts

Extend the existing Rust and Svelte request/output types so MP4 is explicit rather than inferred from a filename. The desktop request uses `media_format`; audio-engine and RADTTS CLI requests continue to use `AudioOutputFormat`/`RadtTsOutputFormat` with only MP3/WAV. For MP4, the request also carries the selected managed image and whether it becomes the project default. The existing serialized settings and output manifests must continue to deserialize with their current defaults.

The durable primary-output record uses `media_format: mp3 | wav | mp4`, `path`, `duration_seconds`, optional `audio_source_path`, optional `image_path`, and workflow-specific caption/timed-segment artifact paths. Legacy RADcast records infer `media_format` from their existing `output_format` field. Legacy RADTTS voice records continue to load from `manifests/outputs.json`, and legacy verified clips continue to be discovered from their existing MP3/WAV files and boundary reports. New RADTTS MP4 voice and clip records are written to `manifests/media-outputs.json` using the same primary-output fields plus `kind` (`voice` or `clip`) and are merged into both RADTTS listings. This avoids rewriting legacy manifests while making MP4 records durable and explicit.

For a requested MP4, the final MP4 is the only durable primary output created by that request. The intermediate cleaned or synthesized WAV is created under an isolated per-job scratch directory and is never sent through the normal output-manifest persistence path. For RADTTS, this scratch directory includes any CLI-generated temporary manifest and is deleted with the scratch job directory; the normal RADTTS `outputs.json` receives no temporary entry, and `media-outputs.json` receives only the completed MP4 record. For verified clips, the MP4 record retains the boundary report and timed-segment artifacts as separate paths. The source audio, previously-created MP3/WAV outputs, captions, and other artifacts are not changed or removed.

Output metadata must identify:

- the MP4 path and format;
- the source audio/output relationship;
- the managed image path used for the export, either the reusable project cover or the durable output-scoped image copy;
- the final duration.

Existing records written before this feature must remain listable and playable. Existing MP3/WAV output records keep their current output formats and playback controls.

Each workflow exposes an output-delete operation for MP4 records. It verifies that the primary path, artifacts, and any output-scoped image are contained in the workflow/project media roots, deletes the MP4 and only artifacts owned by that record, removes the corresponding `media-outputs.json`/RADcast manifest record atomically, and then removes the output-scoped image. A reusable project cover is never deleted by output deletion. If an output-scoped image is referenced by more than one record, it is retained until the last referencing record is deleted. Legacy MP3/WAV deletion behavior is unchanged.

### Atomic output lifecycle

The final MP4 is written to a unique `.partial.mp4` path in the project output directory. The output is probed for the exact stream, codec, dimension, pixel-format, and duration contract above before it is promoted. The temporary intermediate audio uses a unique `.partial.wav` filename in the isolated job scratch directory. A per-export image override is copied into its final output-scoped location only after successful probing.

On encoding failure, invalid probe results, or cancellation, RADsuite removes the partial MP4, temporary audio, temporary image, and any not-yet-committed output-scoped image and does not add a completed output record. After a successful probe, the MP4 is promoted to its final filename and the manifest is written through a temporary manifest followed by an atomic rename. If manifest persistence fails, RADsuite attempts to remove the promoted MP4 and output-scoped image; no completed record is considered present. If any cleanup or rollback fails, the operation remains failed, the error lists every path requiring cleanup, and startup cleanup scans `.partial-*` and recorded orphan paths without ever listing them as completed outputs. Existing source and prior outputs are never removed. Cancellation after FFmpeg exits but before manifest commit still wins and follows the same rollback path.

## Error handling

Return user-readable errors for:

- MP4 selected without an image;
- missing, unsupported, oversized, or unreadable image;
- missing FFmpeg/FFprobe;
- failure to create the managed image/output directory;
- FFmpeg encoding failure or missing audio/video stream;
- invalid duration or output outside the project root;
- failure to remove a temporary intermediate file.

If manifest rollback cannot remove a promoted MP4, RADsuite records its path in a local orphan-cleanup file under the project media root and shows that cleanup is required; it never adds the orphan to the normal output list. A later startup cleanup retries those paths. Cleanup errors must name the affected path and whether it is a temporary, promoted, or output-scoped image file.

A failed video export must not create a completed MP4 output record. The temporary intermediate audio is not a user-facing output and is removed on both success and failure. Cancellation must terminate the active FFmpeg process, remove partial files, and leave the source audio and prior outputs untouched.

## Testing and acceptance criteria

### Automated tests

- Shared exporter tests build deterministic FFmpeg arguments for image looping, input ordering, the exact 540x720 presenter crop, 740x720 charcoal panel, centred 660x240 white waveform, split-panel composition, H.264/AAC mapping, `fps=30`, `-progress pipe:1`, `-shortest`, and output path handling. Probe tests parse fixed ffprobe JSON fixtures and reject missing/duplicate streams, wrong codecs/dimensions/pixel format/frame rate, non-finite or zero duration, and duration mismatch above 0.10 seconds.
- Image-storage tests cover supported extensions, rejected files, the 50-MB limit, managed copies in the shared project-media root, project reuse, and per-export overrides.
- RADcast tests verify MP3/WAV requests are unchanged and MP4 requests render from the final audio stage without changing the selected cleanup/enhancement filters.
- RADTTS tests verify MP3/WAV CLI arguments remain unchanged; both voice-generation and verified-clip flows use an isolated WAV/scratch manifest for MP4 and add only the post-generation mux step to the normal workflow.
- Output-listing tests verify MP4 records deserialize/list alongside legacy MP3/WAV records and captions.
- Output-listing tests verify MP4 records deserialize/list alongside legacy MP3/WAV records and captions for RADcast, RADTTS voice generation, and RADTTS verified clips, including legacy file-scan compatibility.
- Failure tests verify no completed output is persisted after a failed mux, partial/promoted-orphan MP4 files are not listed, temporary audio and images are cleaned or reported with paths as cleanup failures, cancellation terminates the active process runner, and manifest rollback is retried at startup.
- Deletion tests verify output-scoped images are removed with their last owning MP4, reusable project covers are retained, and media manifests are updated atomically.
- UI tests verify MP4 reveals the required `Presenter image` controls, prevents creation without an image, shows the split-composition preview, automatically reuses a saved project image, and preserves that image when a per-export override is used unless the user explicitly saves the replacement. Audio-only formats do not show or require the image controls.

### Real acceptance smoke

Using a short local lecture clip and a presenter-focused PNG or JPEG, including at least one non-16:9 image:

1. Create a RADcast MP4 using the RADcast Optimized profile and the existing trim range.
2. Confirm the result contains one video stream, one audio stream, and the expected duration.
3. Confirm the waveform changes with the audio, the presenter image remains visible in the left panel throughout, and no stretching or overlap occurs.
4. Create a RADTTS MP4 from a generated voice clip using the same saved project image.
5. Confirm both outputs play locally and can be imported into Echo360 as MP4 files.
6. Repeat with a per-export image override and confirm the project default remains unchanged.
7. Cancel an in-progress video export and confirm no partial output is listed.

## Scope exclusions

This slice does not add animated image editing, manual crop positioning, face detection, course-specific theme packs, cloud rendering, online image storage, automatic Echo360 upload, custom waveform colours, arbitrary video resolutions, or changes to the RADcast enhancement algorithms and RADTTS voice models.
