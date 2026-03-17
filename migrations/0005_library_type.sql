-- Add library_type to libraries table
ALTER TABLE libraries
    ADD COLUMN IF NOT EXISTS library_type TEXT NOT NULL DEFAULT 'Mixed';

-- Create an index to quickly filter libraries by type
CREATE INDEX IF NOT EXISTS idx_libraries_library_type ON libraries(library_type);
