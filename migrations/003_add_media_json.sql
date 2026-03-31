-- Add media metadata for figures/tables/graphs per question
-- Stored as JSON array, e.g. [{"kind":"image","path":"/abs/path.png","caption":"Figure 1"}]
ALTER TABLE questions ADD COLUMN media_json TEXT NOT NULL DEFAULT '[]';
