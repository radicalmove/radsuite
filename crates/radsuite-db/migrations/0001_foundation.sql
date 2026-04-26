CREATE TABLE users (
  id TEXT PRIMARY KEY,
  email TEXT NOT NULL UNIQUE,
  display_name TEXT NOT NULL,
  password_hash TEXT NOT NULL,
  is_active INTEGER NOT NULL DEFAULT 1,
  is_admin INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE projects (
  id TEXT PRIMARY KEY,
  owner_id TEXT NOT NULL REFERENCES users(id),
  code TEXT,
  title TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE project_members (
  project_id TEXT NOT NULL REFERENCES projects(id),
  user_id TEXT NOT NULL REFERENCES users(id),
  role TEXT NOT NULL,
  created_at TEXT NOT NULL,
  PRIMARY KEY (project_id, user_id)
);

CREATE TABLE assets (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects(id),
  sha256 TEXT NOT NULL,
  byte_size INTEGER NOT NULL,
  mime_type TEXT NOT NULL,
  original_name TEXT NOT NULL,
  sync_policy TEXT NOT NULL,
  created_at TEXT NOT NULL
);

CREATE TABLE documents (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects(id),
  asset_id TEXT REFERENCES assets(id),
  original_filename TEXT NOT NULL,
  file_type TEXT NOT NULL,
  doc_variant TEXT NOT NULL,
  doc_number INTEGER,
  notes TEXT,
  exclude_from_references INTEGER NOT NULL DEFAULT 0,
  archived_at TEXT,
  uploaded_at TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE INDEX idx_documents_project_id ON documents(project_id);

CREATE TABLE paragraphs (
  id TEXT PRIMARY KEY,
  document_id TEXT NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
  order_index INTEGER NOT NULL,
  page INTEGER,
  text TEXT NOT NULL,
  formatted_text TEXT,
  is_table INTEGER NOT NULL DEFAULT 0,
  needs_citation INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE INDEX idx_paragraphs_document_order ON paragraphs(document_id, order_index);

CREATE TABLE reference_entries (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects(id),
  document_id TEXT REFERENCES documents(id),
  paragraph_id TEXT REFERENCES paragraphs(id),
  reference_type TEXT NOT NULL,
  display_order INTEGER,
  citation_text TEXT,
  apa_citation TEXT,
  title TEXT,
  authors_json TEXT NOT NULL DEFAULT '[]',
  publication_year TEXT,
  source TEXT,
  doi TEXT,
  url TEXT,
  notes TEXT,
  apa_validation_status TEXT NOT NULL DEFAULT 'unknown',
  apa_validation_report TEXT,
  archived_at TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE INDEX idx_reference_entries_project_id ON reference_entries(project_id);

CREATE TABLE paragraph_citations (
  id TEXT PRIMARY KEY,
  paragraph_id TEXT NOT NULL REFERENCES paragraphs(id) ON DELETE CASCADE,
  reference_entry_id TEXT REFERENCES reference_entries(id),
  citation_text TEXT NOT NULL,
  position_start INTEGER,
  position_end INTEGER,
  verified INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE INDEX idx_paragraph_citations_paragraph_id ON paragraph_citations(paragraph_id);

CREATE TABLE sync_records (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects(id),
  entity_type TEXT NOT NULL,
  entity_id TEXT NOT NULL,
  operation TEXT NOT NULL,
  payload_json TEXT NOT NULL,
  local_created_at TEXT NOT NULL,
  server_applied_at TEXT
);
