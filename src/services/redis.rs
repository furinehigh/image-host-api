use redis::{Client, Connection, Commands};
use anyhow::Result;
use serde::{Serialize, Deserialize};
use std::time::Duration;

#[derive(Clone)]
pub struct RedisService {
    client: Client,
}

impl RedisService {
    pub async fn new(redis_url: &str) -> Result<Self> {
        let client = Client::open(redis_url)?;
        
        // Test connection
        let mut conn = client.get_connection()?;
        let _: String = conn.ping()?;
        
        Ok(RedisService { client })
    }

    pub fn get_connection(&self) -> Result<Connection> {
        Ok(self.client.get_connection()?)
    }

    // Enhanced rate limiting with sliding window
    pub async fn check_rate_limit(&self, key: &str, limit: u32, window: Duration) -> Result<bool> {
        let mut conn = self.get_connection()?;
        let now = chrono::Utc::now().timestamp();
        let window_start = now - window.as_secs() as i64;

        // Use Redis sorted set for sliding window rate limiting
        let script = r#"
            local key = KEYS[1]
            local now = tonumber(ARGV[1])
            local window_start = tonumber(ARGV[2])
            local limit = tonumber(ARGV[3])
            local window_seconds = tonumber(ARGV[4])
            
            -- Remove old entries outside the window
            redis.call('ZREMRANGEBYSCORE', key, '-inf', window_start)
            
            -- Count current entries in window
            local current = redis.call('ZCARD', key)
            
            if current < limit then
                -- Add current request with score as timestamp
                redis.call('ZADD', key, now, now .. ':' .. math.random())
                -- Set expiration for cleanup
                redis.call('EXPIRE', key, window_seconds)
                return {1, current + 1}
            else
                return {0, current}
            end
        "#;

        let result: Vec<i32> = redis::Script::new(script)
            .key(key)
            .arg(now)
            .arg(window_start)
            .arg(limit)
            .arg(window.as_secs())
            .invoke(&mut conn)?;

        Ok(result[0] == 1)
    }

    // Get current rate limit count
    pub async fn get_rate_limit_count(&self, key: &str, window: Duration) -> Result<u32> {
        let mut conn = self.get_connection()?;
        let now = chrono::Utc::now().timestamp();
        let window_start = now - window.as_secs() as i64;

        // Clean up old entries and count current
        let _: () = conn.zrembyscore(key, "-inf", window_start)?;
        let count: u32 = conn.zcard(key)?;
        
        Ok(count)
    }

    // Cache image metadata with compression
    pub async fn cache_image_metadata(&self, image_id: &str, metadata: &ImageCacheData, ttl: Duration) -> Result<()> {
        let mut conn = self.get_connection()?;
        let key = format!("image:meta:{}", image_id);
        
        // Serialize and optionally compress
        let data = bincode::serialize(metadata)?;
        let compressed = compress_data(&data)?;
        
        let _: () = conn.set_ex(&key, compressed, ttl.as_secs() as usize)?;
        
        Ok(())
    }

    pub async fn get_cached_image_metadata(&self, image_id: &str) -> Result<Option<ImageCacheData>> {
        let mut conn = self.get_connection()?;
        let key = format!("image:meta:{}", image_id);
        
        let data: Option<Vec<u8>> = conn.get(&key)?;
        
        match data {
            Some(compressed_data) => {
                let decompressed = decompress_data(&compressed_data)?;
                let metadata: ImageCacheData = bincode::deserialize(&decompressed)?;
                Ok(Some(metadata))
            }
            None => Ok(None),
        }
    }

    // Cache transformed images with LRU eviction
    pub async fn cache_transformed_image(&self, cache_key: &str, image_data: &[u8], ttl: Duration) -> Result<()> {
        let mut conn = self.get_connection()?;
        let key = format!("transform:{}", cache_key);
        
        // Compress image data for storage efficiency
        let compressed = compress_data(image_data)?;
        
        // Store with expiration
        let _: () = conn.set_ex(&key, compressed, ttl.as_secs() as usize)?;
        
        // Update LRU tracking
        let lru_key = "transform:lru";
        let _: () = conn.zadd(lru_key, chrono::Utc::now().timestamp(), &key)?;
        
        // Cleanup old entries if needed
        self.cleanup_transform_cache(&mut conn).await?;
        
        Ok(())
    }

    pub async fn get_cached_transformed_image(&self, cache_key: &str) -> Result<Option<Vec<u8>>> {
        let mut conn = self.get_connection()?;
        let key = format!("transform:{}", cache_key);
        
        let data: Option<Vec<u8>> = conn.get(&key)?;
        
        match data {
            Some(compressed_data) => {
                // Update LRU score
                let lru_key = "transform:lru";
                let _: () = conn.zadd(lru_key, chrono::Utc::now().timestamp(), &key)?;
                
                let decompressed = decompress_data(&compressed_data)?;
                Ok(Some(decompressed))
            }
            None => Ok(None),
        }
    }

    // Cleanup old transform cache entries
    async fn cleanup_transform_cache(&self, conn: &mut Connection) -> Result<()> {
        let lru_key = "transform:lru";
        let max_entries = 10000; // Keep max 10k transformed images
        
        let count: usize = conn.zcard(lru_key)?;
        
        if count > max_entries {
            let to_remove = count - max_entries;
            
            // Get oldest entries
            let old_keys: Vec<String> = conn.zrange(lru_key, 0, to_remove as isize - 1)?;
            
            // Remove from cache and LRU set
            for key in old_keys {
                let _: () = conn.del(&key)?;
                let _: () = conn.zrem(lru_key, &key)?;
            }
        }
        
        Ok(())
    }

    // Session management with automatic cleanup
    pub async fn store_session(&self, session_id: &str, user_id: &str, ttl: Duration) -> Result<()> {
        let mut conn = self.get_connection()?;
        let key = format!("session:{}", session_id);
        
        let session_data = SessionData {
            user_id: user_id.to_string(),
            created_at: chrono::Utc::now(),
            last_accessed: chrono::Utc::now(),
        };
        
        let serialized = bincode::serialize(&session_data)?;
        let _: () = conn.set_ex(&key, serialized, ttl.as_secs() as usize)?;
        
        Ok(())
    }

    pub async fn get_session(&self, session_id: &str) -> Result<Option<String>> {
        let mut conn = self.get_connection()?;
        let key = format!("session:{}", session_id);
        
        let data: Option<Vec<u8>> = conn.get(&key)?;
        
        match data {
            Some(serialized) => {
                let mut session_data: SessionData = bincode::deserialize(&serialized)?;
                session_data.last_accessed = chrono::Utc::now();
                
                // Update last accessed time
                let updated = bincode::serialize(&session_data)?;
                let _: () = conn.set(&key, updated)?;
                
                Ok(Some(session_data.user_id))
            }
            None => Ok(None),
        }
    }

    pub async fn delete_session(&self, session_id: &str) -> Result<()> {
        let mut conn = self.get_connection()?;
        let key = format!("session:{}", session_id);
        
        let _: () = conn.del(&key)?;
        
        Ok(())
    }

    // Batch operations for efficiency
    pub async fn batch_cache_images(&self, images: Vec<(String, ImageCacheData)>, ttl: Duration) -> Result<()> {
        let mut conn = self.get_connection()?;
        let mut pipe = redis::pipe();
        
        for (image_id, metadata) in images {
            let key = format!("image:meta:{}", image_id);
            let data = bincode::serialize(&metadata)?;
            let compressed = compress_data(&data)?;
            
            pipe.set_ex(&key, compressed, ttl.as_secs() as usize);
        }
        
        pipe.query(&mut conn)?;
        Ok(())
    }

    // Health check
    pub async fn health_check(&self) -> Result<bool> {
        let mut conn = self.get_connection()?;
        let _: String = conn.ping()?;
        Ok(true)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImageCacheData {
    pub filename: String,
    pub mime_type: String,
    pub file_size: i64,
    pub width: i32,
    pub height: i32,
    pub storage_path: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct SessionData {
    user_id: String,
    created_at: chrono::DateTime<chrono::Utc>,
    last_accessed: chrono::DateTime<chrono::Utc>,
}

// Compression utilities
fn compress_data(data: &[u8]) -> Result<Vec<u8>> {
    use flate2::Compression;
    use flate2::write::GzEncoder;
    use std::io::Write;
    
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data)?;
    Ok(encoder.finish()?)
}

fn decompress_data(compressed: &[u8]) -> Result<Vec<u8>> {
    use flate2::read::GzDecoder;
    use std::io::Read;
    
    let mut decoder = GzDecoder::new(compressed);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed)?;
    Ok(decompressed)
}
