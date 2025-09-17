-- Function to automatically create default rate limit rules when an API key is created
CREATE OR REPLACE FUNCTION create_default_rate_limit_rules()
RETURNS TRIGGER AS $$
DECLARE
    limits JSONB;
BEGIN
    limits := NEW.limits_json->'rate_limits';
    
    -- Create rate limit rules based on the limits in the API key
    INSERT INTO rate_limit_rules (api_key_id, rule_type, capacity, refill_rate) VALUES
    (NEW.id, 'minute', (limits->>'requests_per_minute')::INTEGER, (limits->>'requests_per_minute')::INTEGER),
    (NEW.id, 'hour', (limits->>'requests_per_hour')::INTEGER, (limits->>'requests_per_hour')::INTEGER),
    (NEW.id, 'day', (limits->>'requests_per_day')::INTEGER, (limits->>'requests_per_day')::INTEGER);
    
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Trigger to create default rate limit rules
CREATE TRIGGER trigger_create_rate_limit_rules
    AFTER INSERT ON api_keys
    FOR EACH ROW
    EXECUTE FUNCTION create_default_rate_limit_rules();

-- Function to log image operations
CREATE OR REPLACE FUNCTION log_image_operation()
RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        INSERT INTO events (event_type, payload) VALUES 
        ('image_uploaded', jsonb_build_object(
            'image_id', NEW.id,
            'owner_id', NEW.owner_id,
            'size_bytes', NEW.orig_size_bytes,
            'mime', NEW.mime
        ));
        RETURN NEW;
    ELSIF TG_OP = 'UPDATE' AND OLD.deleted_at IS NULL AND NEW.deleted_at IS NOT NULL THEN
        INSERT INTO events (event_type, payload) VALUES 
        ('image_deleted', jsonb_build_object(
            'image_id', NEW.id,
            'owner_id', NEW.owner_id,
            'size_bytes', NEW.orig_size_bytes
        ));
        RETURN NEW;
    END IF;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

-- Trigger to log image operations
CREATE TRIGGER trigger_log_image_operations
    AFTER INSERT OR UPDATE ON images
    FOR EACH ROW
    EXECUTE FUNCTION log_image_operation();

-- Function to get current usage for an API key
CREATE OR REPLACE FUNCTION get_current_usage(p_api_key_id UUID, p_period TEXT)
RETURNS BIGINT AS $$
DECLARE
    usage_count BIGINT;
    start_date DATE;
BEGIN
    CASE p_period
        WHEN 'day' THEN
            start_date := CURRENT_DATE;
        WHEN 'month' THEN
            start_date := DATE_TRUNC('month', CURRENT_DATE)::DATE;
        ELSE
            RAISE EXCEPTION 'Invalid period: %', p_period;
    END CASE;
    
    SELECT COALESCE(SUM(requests), 0)
    INTO usage_count
    FROM usage_counters
    WHERE api_key_id = p_api_key_id
      AND date >= start_date;
    
    RETURN usage_count;
END;
$$ LANGUAGE plpgsql;

-- Function to check if API key has exceeded limits
CREATE OR REPLACE FUNCTION check_api_key_limits(p_api_key_id UUID)
RETURNS TABLE(
    exceeded BOOLEAN,
    limit_type TEXT,
    current_usage BIGINT,
    limit_value BIGINT
) AS $$
DECLARE
    key_limits JSONB;
    daily_usage BIGINT;
    monthly_usage BIGINT;
    daily_limit BIGINT;
    monthly_limit BIGINT;
BEGIN
    -- Get the API key limits
    SELECT limits_json INTO key_limits
    FROM api_keys
    WHERE id = p_api_key_id AND revoked_at IS NULL;
    
    IF NOT FOUND THEN
        RETURN QUERY SELECT TRUE, 'invalid_key'::TEXT, 0::BIGINT, 0::BIGINT;
        RETURN;
    END IF;
    
    daily_limit := (key_limits->>'daily_limit')::BIGINT;
    monthly_limit := (key_limits->>'monthly_limit')::BIGINT;
    
    -- Get current usage
    daily_usage := get_current_usage(p_api_key_id, 'day');
    monthly_usage := get_current_usage(p_api_key_id, 'month');
    
    -- Check daily limit
    IF daily_usage >= daily_limit THEN
        RETURN QUERY SELECT TRUE, 'daily'::TEXT, daily_usage, daily_limit;
        RETURN;
    END IF;
    
    -- Check monthly limit
    IF monthly_usage >= monthly_limit THEN
        RETURN QUERY SELECT TRUE, 'monthly'::TEXT, monthly_usage, monthly_limit;
        RETURN;
    END IF;
    
    -- No limits exceeded
    RETURN QUERY SELECT FALSE, 'none'::TEXT, daily_usage, daily_limit;
END;
$$ LANGUAGE plpgsql;

-- Create a view for active API keys with their current usage
CREATE VIEW active_api_keys_with_usage AS
SELECT 
    ak.id,
    ak.name,
    ak.owner_id,
    ak.created_at,
    ak.limits_json,
    u.email as owner_email,
    COALESCE(daily_usage.requests, 0) as daily_requests,
    COALESCE(monthly_usage.requests, 0) as monthly_requests,
    (ak.limits_json->>'daily_limit')::BIGINT as daily_limit,
    (ak.limits_json->>'monthly_limit')::BIGINT as monthly_limit
FROM api_keys ak
JOIN users u ON ak.owner_id = u.id
LEFT JOIN (
    SELECT api_key_id, SUM(requests) as requests
    FROM usage_counters
    WHERE date = CURRENT_DATE
    GROUP BY api_key_id
) daily_usage ON ak.id = daily_usage.api_key_id
LEFT JOIN (
    SELECT api_key_id, SUM(requests) as requests
    FROM usage_counters
    WHERE date >= DATE_TRUNC('month', CURRENT_DATE)::DATE
    GROUP BY api_key_id
) monthly_usage ON ak.id = monthly_usage.api_key_id
WHERE ak.revoked_at IS NULL;
