-- Create extension for UUID generation
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Users table
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    is_admin BOOLEAN DEFAULT FALSE,
    profile_json JSONB DEFAULT '{}'::jsonb
);

-- API Keys table
CREATE TABLE api_keys (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    key_hash VARCHAR(255) UNIQUE NOT NULL,
    owner_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    revoked_at TIMESTAMP WITH TIME ZONE,
    config_json JSONB DEFAULT '{}'::jsonb,
    limits_json JSONB NOT NULL DEFAULT '{
        "daily_limit": 1000,
        "monthly_limit": 30000,
        "max_images": 10000,
        "max_image_size_bytes": 20971520,
        "allowed_origins": [],
        "rate_limits": {
            "requests_per_minute": 60,
            "requests_per_hour": 1000,
            "requests_per_day": 10000
        }
    }'::jsonb
);

-- Images table
CREATE TABLE images (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    owner_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    sha256 VARCHAR(64) UNIQUE NOT NULL,
    mime VARCHAR(100) NOT NULL,
    orig_size_bytes BIGINT NOT NULL,
    width INTEGER NOT NULL,
    height INTEGER NOT NULL,
    storage_path VARCHAR(500) NOT NULL,
    variants JSONB NOT NULL DEFAULT '{}'::jsonb,
    is_public BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    deleted_at TIMESTAMP WITH TIME ZONE,
    expires_at TIMESTAMP WITH TIME ZONE
);

-- Usage counters table for tracking API usage
CREATE TABLE usage_counters (
    date DATE NOT NULL,
    api_key_id UUID NOT NULL REFERENCES api_keys(id) ON DELETE CASCADE,
    requests BIGINT DEFAULT 0,
    bytes_served BIGINT DEFAULT 0,
    uploads BIGINT DEFAULT 0,
    PRIMARY KEY (date, api_key_id)
);

-- Rate limit rules table
CREATE TABLE rate_limit_rules (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    api_key_id UUID NOT NULL REFERENCES api_keys(id) ON DELETE CASCADE,
    rule_type VARCHAR(50) NOT NULL, -- 'minute', 'hour', 'day'
    capacity INTEGER NOT NULL,
    refill_rate INTEGER NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Events table for background worker logging and audit trail
CREATE TABLE events (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    event_type VARCHAR(100) NOT NULL,
    payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    processed_at TIMESTAMP WITH TIME ZONE,
    error_message TEXT
);

-- Create indexes for performance
CREATE INDEX idx_api_keys_key_hash ON api_keys(key_hash);
CREATE INDEX idx_api_keys_owner_id ON api_keys(owner_id);
CREATE INDEX idx_api_keys_revoked_at ON api_keys(revoked_at) WHERE revoked_at IS NULL;

CREATE INDEX idx_images_sha256 ON images(sha256);
CREATE INDEX idx_images_owner_id ON images(owner_id);
CREATE INDEX idx_images_created_at ON images(created_at);
CREATE INDEX idx_images_deleted_at ON images(deleted_at) WHERE deleted_at IS NULL;
CREATE INDEX idx_images_is_public ON images(is_public);
CREATE INDEX idx_images_expires_at ON images(expires_at) WHERE expires_at IS NOT NULL;

CREATE INDEX idx_usage_counters_date ON usage_counters(date);
CREATE INDEX idx_usage_counters_api_key_id ON usage_counters(api_key_id);

CREATE INDEX idx_rate_limit_rules_api_key_id ON rate_limit_rules(api_key_id);

CREATE INDEX idx_events_type ON events(event_type);
CREATE INDEX idx_events_created_at ON events(created_at);
CREATE INDEX idx_events_processed_at ON events(processed_at) WHERE processed_at IS NULL;

-- Create a function to automatically update usage counters
CREATE OR REPLACE FUNCTION update_usage_counter(
    p_api_key_id UUID,
    p_requests INTEGER DEFAULT 0,
    p_bytes_served BIGINT DEFAULT 0,
    p_uploads INTEGER DEFAULT 0
) RETURNS VOID AS $$
BEGIN
    INSERT INTO usage_counters (date, api_key_id, requests, bytes_served, uploads)
    VALUES (CURRENT_DATE, p_api_key_id, p_requests, p_bytes_served, p_uploads)
    ON CONFLICT (date, api_key_id)
    DO UPDATE SET
        requests = usage_counters.requests + p_requests,
        bytes_served = usage_counters.bytes_served + p_bytes_served,
        uploads = usage_counters.uploads + p_uploads;
END;
$$ LANGUAGE plpgsql;

-- Create a function to clean up expired images
CREATE OR REPLACE FUNCTION cleanup_expired_images() RETURNS INTEGER AS $$
DECLARE
    expired_count INTEGER;
BEGIN
    UPDATE images 
    SET deleted_at = NOW()
    WHERE expires_at IS NOT NULL 
      AND expires_at < NOW() 
      AND deleted_at IS NULL;
    
    GET DIAGNOSTICS expired_count = ROW_COUNT;
    
    -- Log the cleanup event
    INSERT INTO events (event_type, payload)
    VALUES ('image_cleanup', jsonb_build_object('expired_count', expired_count));
    
    RETURN expired_count;
END;
$$ LANGUAGE plpgsql;
