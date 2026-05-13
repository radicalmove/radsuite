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

export type ToolArea = "documents" | "references" | "readings" | "exports" | "radcast" | "radtts";

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

export type CourseReferencesExportRequest = {
  for_ako_learn: boolean;
};

export type CourseReferencesExport = {
  filename: string;
  content_type: string;
  html: string;
  reference_count: number;
};
