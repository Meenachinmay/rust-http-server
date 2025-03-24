ALTER TABLE videos
ADD COLUMN upload_id VARCHAR(255);

CREATE INDEX idx_videos_upload_id ON videos(upload_id);