ALTER TABLE projects ADD COLUMN description TEXT;
ALTER TABLE projects ADD COLUMN structure_mode TEXT NOT NULL DEFAULT 'modules';
