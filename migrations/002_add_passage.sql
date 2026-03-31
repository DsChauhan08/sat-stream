-- Add passage column for reading comprehension questions
ALTER TABLE questions ADD COLUMN passage TEXT NOT NULL DEFAULT '';
