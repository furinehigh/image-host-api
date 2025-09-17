-- Add additional metadata columns to images table
ALTER TABLE images ADD COLUMN metadata JSONB DEFAULT '{}';
ALTER TABLE images ADD COLUMN tags TEXT[] DEFAULT '{}';
ALTER TABLE images ADD COLUMN is_public BOOLEAN DEFAULT FALSE;
ALTER TABLE images ADD COLUMN download_count INTEGER DEFAULT 0;

-- Add indexes for new columns
CREATE INDEX idx_images_metadata ON images USING GIN (metadata);
CREATE INDEX idx_images_tags ON images USING GIN (tags);
CREATE INDEX idx_images_public ON images(is_public) WHERE is_public = TRUE;

-- Add function to update download count
CREATE OR REPLACE FUNCTION increment_download_count(image_uuid UUID)
RETURNS VOID AS $$
BEGIN
    UPDATE images SET download_count = download_count + 1 WHERE id = image_uuid;
END;
$$ LANGUAGE plpgsql;
