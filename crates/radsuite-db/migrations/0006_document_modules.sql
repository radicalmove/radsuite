ALTER TABLE documents ADD COLUMN module_id TEXT REFERENCES course_modules(id);

CREATE INDEX idx_documents_module_id
ON documents(module_id);
