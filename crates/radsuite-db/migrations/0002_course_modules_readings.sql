CREATE TABLE course_modules (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects(id),
  code TEXT,
  title TEXT NOT NULL,
  order_index INTEGER,
  description TEXT,
  archived_at TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE INDEX idx_course_modules_project_order
ON course_modules(project_id, order_index, title);

ALTER TABLE reference_entries ADD COLUMN module_id TEXT REFERENCES course_modules(id);
ALTER TABLE reference_entries ADD COLUMN lesson_code TEXT;
ALTER TABLE reference_entries ADD COLUMN reading_category TEXT;
ALTER TABLE reference_entries ADD COLUMN reading_notes TEXT;
ALTER TABLE reference_entries ADD COLUMN estimated_reading_time TEXT;

CREATE INDEX idx_reference_entries_module_type
ON reference_entries(module_id, reference_type);
