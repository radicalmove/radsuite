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

export type ParagraphFilter = "all" | "citation-total" | "has-citation" | "needs-citation";

export type ReviewCitation = {
  id: string;
  text: string;
  start: number | null;
  end: number | null;
  verified: boolean;
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
};

export type AnalyseDocxReviewResponse = {
  project_id: string;
  project_title: string;
  document_id: string;
  original_filename: string;
  summary: AnalyseDocxSummary;
  paragraphs: ReviewParagraph[];
};
