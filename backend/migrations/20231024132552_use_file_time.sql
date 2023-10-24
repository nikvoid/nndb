-- Add migration script here

-- Add new table column for storing file creation/modification date
ALTER TABLE element ADD COLUMN file_time INTEGER;
-- RUN add_file_time