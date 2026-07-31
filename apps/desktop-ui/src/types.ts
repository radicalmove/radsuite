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
  structureMode: "modules" | "weeks";
};

export type RadciteProjectSummary = {
  id: string;
  code: string | null;
  title: string;
};

export type ToolArea =
  | "documents"
  | "references"
  | "readings"
  | "exports"
  | "archive"
  | "radcast"
  | "radtts";

export type AudioOutputFormat = "mp3" | "wav";

export type CaptionFormat = "srt" | "vtt";

export type CaptionQualityMode = "fast" | "accurate" | "reviewed";

export type FillerRemovalMode = "normal" | "aggressive";

export type EnhancementModel = "none" | "studio_v18";

export type EnhancementQuality = "fast" | "standard" | "high";

export type RadcastAudioSource = {
  id: string;
  original_filename: string;
  path: string;
  duration_seconds: number;
  byte_size: number;
  created_at: string;
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
  enhancement_model: EnhancementModel;
  enhancement_quality: EnhancementQuality;
  caption_segment_count: number;
  remove_filler_words: boolean;
  filler_removal_mode: FillerRemovalMode;
  removed_filler_count: number;
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
};

export type RadcastCapabilityStatus = {
  caption_available: boolean;
  caption_detail: string;
  optimized_available: boolean;
  optimized_detail: string;
};

export type RadcastJobState = "running" | "completed" | "failed";

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
  summary: AnalyseDocxSummary;
  paragraphs: ReviewParagraph[];
};

export type SavedRadciteReviewSummary = {
  document_id: string;
  project_id: string;
  original_filename: string;
  paragraph_count: number;
  citation_count: number;
  missing_citation_count: number;
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

export type CourseReferencesExportRequest = {
  project_id?: string | null;
  for_ako_learn: boolean;
};

export type ModuleReadingsExportRequest = {
  module_id: string;
  for_ako_learn: boolean;
};

export type CourseReferencesExport = {
  filename: string;
  content_type: string;
  html: string;
  reference_count: number;
};

export type ModuleReadingsExport = {
  filename: string;
  content_type: string;
  html: string;
  module_id: string;
  reading_count: number;
};
