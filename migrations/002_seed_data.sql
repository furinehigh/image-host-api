-- Create a default admin user (password: 'admin123')
INSERT INTO users (id, email, password_hash, is_admin) VALUES 
(
    '00000000-0000-0000-0000-000000000001',
    'admin@example.com',
    '$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewdBPj/RK.PmvlJO', -- bcrypt hash of 'admin123'
    true
);

-- Create a default API key for the admin user (key: 'admin_key_12345')
INSERT INTO api_keys (id, key_hash, owner_id, name, limits_json) VALUES 
(
    '00000000-0000-0000-0000-000000000002',
    'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855', -- sha256 hash of 'admin_key_12345'
    '00000000-0000-0000-0000-000000000001',
    'Default Admin Key',
    '{
        "daily_limit": 100000,
        "monthly_limit": 3000000,
        "max_images": 1000000,
        "max_image_size_bytes": 104857600,
        "allowed_origins": ["*"],
        "rate_limits": {
            "requests_per_minute": 1000,
            "requests_per_hour": 10000,
            "requests_per_day": 100000
        }
    }'::jsonb
);

-- Create default rate limit rules for the admin key
INSERT INTO rate_limit_rules (api_key_id, rule_type, capacity, refill_rate) VALUES
('00000000-0000-0000-0000-000000000002', 'minute', 1000, 1000),
('00000000-0000-0000-0000-000000000002', 'hour', 10000, 10000),
('00000000-0000-0000-0000-000000000002', 'day', 100000, 100000);

-- Create a test user (password: 'testuser123')
INSERT INTO users (id, email, password_hash, is_admin) VALUES 
(
    '00000000-0000-0000-0000-000000000003',
    'test@example.com',
    '$2b$12$92IXUNpkjO0rOQ5byMi.Ye4oKoEa3Ro9llC/.og/at2.uheWG/igi', -- bcrypt hash of 'testuser123'
    false
);

-- Create a test API key (key: 'test_key_67890')
INSERT INTO api_keys (id, key_hash, owner_id, name, limits_json) VALUES 
(
    '00000000-0000-0000-0000-000000000004',
    'a665a45920422f9d417e4867efdc4fb8a04a1f3fff1fa07e998e86f7f7a27ae3', -- sha256 hash of 'test_key_67890'
    '00000000-0000-0000-0000-000000000003',
    'Test User Key',
    '{
        "daily_limit": 1000,
        "monthly_limit": 30000,
        "max_images": 10000,
        "max_image_size_bytes": 20971520,
        "allowed_origins": ["http://localhost:3000", "https://example.com"],
        "rate_limits": {
            "requests_per_minute": 60,
            "requests_per_hour": 1000,
            "requests_per_day": 10000
        }
    }'::jsonb
);

-- Create default rate limit rules for the test key
INSERT INTO rate_limit_rules (api_key_id, rule_type, capacity, refill_rate) VALUES
('00000000-0000-0000-0000-000000000004', 'minute', 60, 60),
('00000000-0000-0000-0000-000000000004', 'hour', 1000, 1000),
('00000000-0000-0000-0000-000000000004', 'day', 10000, 10000);
