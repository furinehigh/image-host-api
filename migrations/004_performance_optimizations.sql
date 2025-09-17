-- Add partial indexes for better performance
CREATE INDEX CONCURRENTLY idx_images_active 
ON images(owner_id, created_at DESC) 
WHERE deleted_at IS NULL;

CREATE INDEX CONCURRENTLY idx_images_public_active 
ON images(created_at DESC) 
WHERE is_public = true AND deleted_at IS NULL;

CREATE INDEX CONCURRENTLY idx_usage_counters_recent 
ON usage_counters(api_key_id, date DESC) 
WHERE date >= CURRENT_DATE - INTERVAL '30 days';

-- Add GIN index for JSONB columns for better query performance
CREATE INDEX CONCURRENTLY idx_images_variants_gin ON images USING GIN(variants);
CREATE INDEX CONCURRENTLY idx_api_keys_limits_gin ON api_keys USING GIN(limits_json);
CREATE INDEX CONCURRENTLY idx_events_payload_gin ON events USING GIN(payload);

-- Create materialized view for usage statistics (refresh periodically)
CREATE MATERIALIZED VIEW usage_statistics AS
SELECT 
    DATE_TRUNC('day', uc.date) as day,
    COUNT(DISTINCT uc.api_key_id) as active_keys,
    SUM(uc.requests) as total_requests,
    SUM(uc.bytes_served) as total_bytes_served,
    SUM(uc.uploads) as total_uploads,
    AVG(uc.requests) as avg_requests_per_key,
    MAX(uc.requests) as max_requests_per_key
FROM usage_counters uc
WHERE uc.date >= CURRENT_DATE - INTERVAL '90 days'
GROUP BY DATE_TRUNC('day', uc.date)
ORDER BY day DESC;

-- Create unique index on the materialized view
CREATE UNIQUE INDEX idx_usage_statistics_day ON usage_statistics(day);

-- Function to refresh usage statistics
CREATE OR REPLACE FUNCTION refresh_usage_statistics()
RETURNS VOID AS $$
BEGIN
    REFRESH MATERIALIZED VIEW CONCURRENTLY usage_statistics;
END;
$$ LANGUAGE plpgsql;

-- Add table for storing image processing jobs queue
CREATE TABLE processing_jobs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    image_id UUID NOT NULL REFERENCES images(id) ON DELETE CASCADE,
    job_type VARCHAR(50) NOT NULL, -- 'resize', 'convert', 'optimize'
    parameters JSONB NOT NULL DEFAULT '{}'::jsonb,
    status VARCHAR(20) DEFAULT 'pending', -- 'pending', 'processing', 'completed', 'failed'
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    started_at TIMESTAMP WITH TIME ZONE,
    completed_at TIMESTAMP WITH TIME ZONE,
    error_message TEXT,
    retry_count INTEGER DEFAULT 0,
    max_retries INTEGER DEFAULT 3
);

CREATE INDEX idx_processing_jobs_status ON processing_jobs(status, created_at);
CREATE INDEX idx_processing_jobs_image_id ON processing_jobs(image_id);
CREATE INDEX idx_processing_jobs_retry ON processing_jobs(retry_count, max_retries) WHERE status = 'failed';
