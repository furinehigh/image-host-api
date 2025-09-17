-- Add additional indexes for performance
CREATE INDEX idx_images_created_at ON images(created_at DESC);
CREATE INDEX idx_images_user_created ON images(user_id, created_at DESC);
CREATE INDEX idx_api_keys_expires_at ON api_keys(expires_at) WHERE expires_at IS NOT NULL;

-- Add constraints for data integrity
ALTER TABLE images ADD CONSTRAINT chk_file_size_positive CHECK (file_size > 0);
ALTER TABLE images ADD CONSTRAINT chk_dimensions_positive CHECK (width > 0 AND height > 0);
ALTER TABLE users ADD CONSTRAINT chk_quota_positive CHECK (quota_bytes >= 0);
ALTER TABLE users ADD CONSTRAINT chk_used_bytes_positive CHECK (used_bytes >= 0);

-- Add unique constraint on sha256_hash for deduplication
CREATE UNIQUE INDEX idx_images_sha256_unique ON images(sha256_hash);
