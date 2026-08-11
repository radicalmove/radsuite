export type EngineStatus = {
  id: string;
  label: string;
  available: boolean;
  detail: string;
};

export type AppStatus = {
  app_name: string;
  database_ready: boolean;
  sync_configured: boolean;
  engines: EngineStatus[];
};

export type ProjectNavItem = {
  id: string;
  code: string;
  title: string;
  description: string | null;
  structureMode: "modules" | "weeks";
  archived_at: string | null;
};

export type RadciteProjectSummary = {
  id: string;
  code: string | null;
  title: string;
  description: string | null;
  structure_mode: "modules" | "weeks";
  archived_at: string | null;
};

export type ToolArea =
  | "documents"
  | "references"
  | "readings"
  | "exports"
  | "archive"
  | "radcast"
  | "radtts"
  | "radt-tools";

export type DocumentSource = "docx" | "pdf";

export type RadciteDocumentVariant = "content" | "rise" | "other";

export type AudioOutputFormat = "mp3" | "wav";

export type CaptionFormat = "srt" | "vtt";

export type CaptionQualityMode = "fast" | "accurate" | "reviewed";

export type FillerRemovalMode = "normal" | "aggressive";

export type EnhancementModel = "none" | "resemble" | "deepfilternet" | "studio" | "studio_v18" | "studio_v18_natural" | "studio_v18_natural_plus" | "studio_v18_natural_double_plus";

export type EnhancementQuality = "fast" | "standard" | "high";

export type RadcastAudioSource = {
  id: string;
  original_filename: string;
  path: string;
  duration_seconds: number;
  byte_size: number;
  created_at: string;
};

export type RadcastTrimRange = {
  clip_start_seconds: number;
  clip_end_seconds: number;
};

export type RadcastAudioOutput = {
  id: string;
  source_id: string;
  filename: string;
  path: string;
  duration_seconds: number;
  output_format: AudioOutputFormat;
  cleanup_enabled: boolean;
  clip_start_seconds: number | null;
  clip_end_seconds: number | null;
  max_silence_seconds: number | null;
  caption_path: string | null;
  caption_format: CaptionFormat | null;
  caption_quality_mode: CaptionQualityMode;
  caption_glossary: string | null;
  caption_review_path: string | null;
  caption_review_required: boolean;
  caption_average_probability: number | null;
  caption_low_confidence_segments: number;
  caption_total_segments: number;
  enhancement_model: EnhancementModel;
  enhancement_quality: EnhancementQuality;
  caption_segment_count: number;
  remove_filler_words: boolean;
  filler_removal_mode: FillerRemovalMode;
  removed_filler_count: number;
  removed_pause_count: number;
  created_at: string;
};

export type RadcastAudioListing = {
  sources: RadcastAudioSource[];
  outputs: RadcastAudioOutput[];
  settings: RadcastProjectSettings;
};

export type RadcastProjectSettings = {
  output_format: AudioOutputFormat;
  caption_format: CaptionFormat | null;
  caption_language: string;
  caption_quality_mode: CaptionQualityMode;
  caption_glossary: string | null;
  enhancement_model: EnhancementModel;
  enhancement_quality: EnhancementQuality;
  cleanup_enabled: boolean;
  max_silence_seconds: number | null;
  remove_filler_words: boolean;
  filler_removal_mode: FillerRemovalMode;
  trim_ranges_by_source_id: Record<string, RadcastTrimRange>;
};

export type RadcastCapabilityStatus = {
  caption_available: boolean;
  caption_detail: string;
  optimized_available: boolean;
  optimized_detail: string;
  enhancement_models: RadcastEnhancementCapability[];
};

export type RadcastEnhancementCapability = {
  id: EnhancementModel;
  label: string;
  description: string;
  available: boolean;
  detail: string;
};

export type RadcastJobState = "running" | "completed" | "failed" | "cancelled";

export type RadcastProcessingPhase =
  | "preparing"
  | "removing_filler_words"
  | "preparing_enhancement"
  | "enhancing_audio"
  | "rendering_audio"
  | "generating_captions"
  | "saving_output";

export type RadcastJobStatus = {
  id: string;
  state: RadcastJobState;
  phase: RadcastProcessingPhase;
  percent: number;
  elapsed_seconds: number;
  output: RadcastAudioOutput | null;
  error: string | null;
};

export type RadtTsQuality = "fast" | "high";

export type RadtTsVoiceSource = "reference" | "builtin";

export type RadtTsChunkMode = "single" | "sentence";

export type RadtTsOutputFormat = "mp3" | "wav";

export type RadtTsCapabilityStatus = {
  available: boolean;
  executable: string | null;
  detail: string;
};

export type RadtTsJobState = "starting" | "running" | "completed" | "failed" | "cancelled";

export type RadtTsProcessingPhase = "preparing" | "generating" | "saving_output";

export type RadtTsAudioOutput = {
  id: string;
  filename: string;
  path: string;
  output_format: RadtTsOutputFormat;
  caption_paths: string[];
  duration_seconds: number | null;
  created_at: string | null;
};

export type RadtTsJobStatus = {
  id: string;
  project_id: string;
  state: RadtTsJobState;
  phase: RadtTsProcessingPhase;
  percent: number | null;
  output: RadtTsAudioOutput | null;
  error: string | null;
};

export type RadtTsMediaJobKind = "transcription" | "clip";

export type RadtTsMediaProcessingPhase =
  | "preparing"
  | "transcribing"
  | "extracting_clip"
  | "saving_output";

export type RadtTsVerificationMode = "strict" | "lenient";

export type RadtTsMediaArtifact = {
  label: string;
  path: string;
};

export type RadtTsMediaOutput = {
  id: string;
  kind: RadtTsMediaJobKind;
  name: string;
  primary_path: string;
  artifacts: RadtTsMediaArtifact[];
  output_format: RadtTsOutputFormat | null;
  warnings: string[];
};

export type RadtTsMediaJobStatus = {
  id: string;
  project_id: string;
  kind: RadtTsMediaJobKind;
  state: RadtTsJobState;
  phase: RadtTsMediaProcessingPhase;
  percent: number | null;
  output: RadtTsMediaOutput | null;
  error: string | null;
};

export type ParagraphFilter =
  | "all"
  | "citation-total"
  | "has-citation"
  | "needs-citation"
  | "linked-citation"
  | "suggested-citation"
  | "unlinked-citation";

export type ReviewCitationReferenceSuggestion = {
  reference_entry_id: string;
  label: string;
  confidence: "strong" | "possible";
  reason: string;
};

export type ReviewCitation = {
  id: string;
  text: string;
  start: number | null;
  end: number | null;
  verified: boolean;
  reference_entry_id: string | null;
  reference_suggestions: ReviewCitationReferenceSuggestion[];
};

export type ReviewParagraph = {
  id: string;
  order_index: number;
  page: number | null;
  text: string;
  formatted_text: string | null;
  is_table: boolean;
  needs_citation: boolean;
  citations: ReviewCitation[];
};

export type AnalyseDocxSummary = {
  paragraph_count: number;
  citation_count: number;
  cited_paragraph_count: number;
  missing_citation_count: number;
  linked_citation_count: number;
  suggested_citation_count: number;
  unlinked_citation_count: number;
};

export type AnalyseDocxReviewResponse = {
  project_id: string;
  project_title: string;
  document_id: string;
  original_filename: string;
  display_name: string;
  source_path: string | null;
  source_file_type: DocumentSource;
  doc_variant: RadciteDocumentVariant;
  doc_number: number | null;
  exclude_from_references: boolean;
  summary: AnalyseDocxSummary;
  paragraphs: ReviewParagraph[];
};

export type SavedRadciteReviewSummary = {
  document_id: string;
  project_id: string;
  original_filename: string;
  display_name: string;
  source_path: string | null;
  source_file_type: DocumentSource;
  doc_variant: RadciteDocumentVariant;
  doc_number: number | null;
  exclude_from_references: boolean;
  paragraph_count: number;
  citation_count: number;
  missing_citation_count: number;
};

export type RadciteReviewReportExport = {
  filename: string;
  content_type: string;
  json: string;
};

export type RadciteArchiveItemKind =
  | "document"
  | "module"
  | "course_reference"
  | "module_reading";

export type RadciteArchiveItem = {
  id: string;
  kind: RadciteArchiveItemKind;
  label: string;
  detail: string | null;
  archived_at: string;
};

export type CourseReferenceSummary = {
  id: string;
  project_id: string;
  module_id: string | null;
  reference_type: "reference" | "reading";
  apa_citation: string | null;
  citation_text: string | null;
  title: string | null;
  authors: string[];
  publication_year: string | null;
  source: string | null;
  doi: string | null;
  url: string | null;
  notes: string | null;
  validation_status: "unknown" | "valid" | "needs_fix";
  validation_report: string | null;
};

export type CourseModuleSummary = {
  id: string;
  project_id: string;
  code: string | null;
  title: string;
  order_index: number | null;
  description: string | null;
};

export type ModuleReadingSummary = {
  id: string;
  project_id: string;
  module_id: string;
  reading_category: "compulsory" | "optional";
  lesson_code: string | null;
  apa_citation: string | null;
  citation_text: string | null;
  title: string | null;
  doi: string | null;
  url: string | null;
  notes: string | null;
  reading_notes: string | null;
  estimated_reading_time: string | null;
  validation_status: "unknown" | "valid" | "needs_fix";
  validation_report: string | null;
};

export type ModuleReadingImportCandidate = {
  source_path: string | null;
  source_filename: string | null;
  module_order: number | null;
  module_title: string | null;
  reading_category: "compulsory" | "optional";
  lesson_code: string | null;
  apa_citation: string;
  citation_text: string | null;
  doi: string | null;
  url: string | null;
};

export type ModuleReadingsPdfImportFailure = {
  path: string;
  message: string;
};

export type ModuleReadingsPdfImportPreview = {
  candidates: ModuleReadingImportCandidate[];
  failures: ModuleReadingsPdfImportFailure[];
};

export type ImportDocumentReadingsResponse = {
  candidate_count: number;
  saved_count: number;
  created_module_count: number;
  unassigned_count: number;
  failed_file_count: number;
};

export type CourseReferencesExportRequest = {
  project_id?: string | null;
  for_ako_learn: boolean;
  allow_incomplete: boolean;
  use_library_links: boolean;
};

export type ModuleReadingsExportRequest = {
  module_id: string;
  for_ako_learn: boolean;
  use_library_links: boolean;
};

export type CourseReferencesExport = {
  filename: string;
  content_type: string;
  html: string;
  reference_count: number;
  apa_error_count: number;
  apa_warning_count: number;
};

export type ModuleReadingsExport = {
  filename: string;
  content_type: string;
  html: string;
  module_id: string;
  reading_count: number;
};
