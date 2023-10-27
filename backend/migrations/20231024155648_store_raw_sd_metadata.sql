-- Add migration script here

-- Drop entire ai_metadata table
DROP TABLE ai_metadata;

-- Add column for storing raw metadata in existing table
ALTER TABLE metadata ADD raw_meta TEXT;

-- Add raw metadata for each element
-- RUN add_raw_sd_meta
