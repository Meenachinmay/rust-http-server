-- Add status and value fields to videos table
ALTER TABLE videos
    ADD COLUMN IF NOT EXISTS status VARCHAR(255) NOT NULL DEFAULT 'pending',
    ADD COLUMN IF NOT EXISTS course_value VARCHAR(255) NOT NULL DEFAULT 'free';