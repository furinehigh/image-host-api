use redis::{AsyncCommands, RedisResult};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use uuid::Uuid;

use crate::{
    errors::{AppError, Result},
    services::redis::RedisService,
};

pub struct RateLimiter {
    redis: RedisService,
}

impl RateLimiter {
    pub fn new(redis: RedisService) -> Self {
        Self { redis }
    }

    pub async fn check_rate_limit(
        &self,
        api_key_id: Uuid,
        limit_type: &str,
        capacity: u32,
        refill_rate: u32,
    ) -> Result<RateLimitResult> {
        let key = format!("rate_limit:{}:{}", api_key_id, limit_type);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Use Lua script for atomic rate limiting
        let lua_script = r#"
            local key = KEYS[1]
            local capacity = tonumber(ARGV[1])
            local refill_rate = tonumber(ARGV[2])
            local now = tonumber(ARGV[3])
            local requested = tonumber(ARGV[4])
            
            local bucket = redis.call('HMGET', key, 'tokens', 'last_refill')
            local tokens = tonumber(bucket[1]) or capacity
            local last_refill = tonumber(bucket[2]) or now
            
            -- Calculate tokens to add based on time elapsed
            local time_elapsed = now - last_refill
            local tokens_to_add = math.floor(time_elapsed * refill_rate / 60) -- refill_rate per minute
            tokens = math.min(capacity, tokens + tokens_to_add)
            
            local allowed = 0
            if tokens >= requested then
                tokens = tokens - requested
                allowed = 1
            end
            
            -- Update bucket state
            redis.call('HMSET', key, 'tokens', tokens, 'last_refill', now)
            redis.call('EXPIRE', key, 3600) -- Expire after 1 hour of inactivity
            
            return {allowed, tokens, capacity}
        "#;

        let mut conn = self.redis.connection_manager().clone();
        let result: Vec<i32> = redis::Script::new(lua_script)
            .key(&key)
            .arg(capacity)
            .arg(refill_rate)
            .arg(now)
            .arg(1) // requesting 1 token
            .invoke_async(&mut conn)
            .await
            .map_err(|e| AppError::Redis(e))?;

        Ok(RateLimitResult {
            allowed: result[0] == 1,
            remaining_tokens: result[1] as u32,
            capacity,
            reset_time: now + 60, // Next potential refill
        })
    }

    pub async fn check_quota_limits(
        &self,
        api_key_id: Uuid,
        current_usage: u64,
        limit: u64,
    ) -> Result<bool> {
        Ok(current_usage < limit)
    }

    pub async fn increment_usage_counter(
        &self,
        api_key_id: Uuid,
        counter_type: &str,
        amount: u64,
    ) -> Result<u64> {
        let key = format!("usage:{}:{}", api_key_id, counter_type);
        let mut conn = self.redis.connection_manager().clone();
        
        let new_value: u64 = conn.incr(&key, amount).await
            .map_err(|e| AppError::Redis(e))?;
        
        // Set expiration based on counter type
        let expiration = match counter_type {
            "daily" => 86400,   // 24 hours
            "monthly" => 2592000, // 30 days
            _ => 3600,          // 1 hour default
        };
        
        conn.expire(&key, expiration).await
            .map_err(|e| AppError::Redis(e))?;

        Ok(new_value)
    }

    pub async fn get_usage_counter(
        &self,
        api_key_id: Uuid,
        counter_type: &str,
    ) -> Result<u64> {
        let key = format!("usage:{}:{}", api_key_id, counter_type);
        let mut conn = self.redis.connection_manager().clone();
        
        let value: Option<u64> = conn.get(&key).await
            .map_err(|e| AppError::Redis(e))?;
        
        Ok(value.unwrap_or(0))
    }

    pub async fn reset_rate_limit(&self, api_key_id: Uuid, limit_type: &str) -> Result<()> {
        let key = format!("rate_limit:{}:{}", api_key_id, limit_type);
        let mut conn = self.redis.connection_manager().clone();
        
        conn.del(&key).await
            .map_err(|e| AppError::Redis(e))?;
        
        Ok(())
    }
}

#[derive(Debug)]
pub struct RateLimitResult {
    pub allowed: bool,
    pub remaining_tokens: u32,
    pub capacity: u32,
    pub reset_time: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests would require a Redis instance for integration testing
    #[tokio::test]
    async fn test_rate_limit_result() {
        let result = RateLimitResult {
            allowed: true,
            remaining_tokens: 59,
            capacity: 60,
            reset_time: 1234567890,
        };
        
        assert!(result.allowed);
        assert_eq!(result.remaining_tokens, 59);
    }
}
