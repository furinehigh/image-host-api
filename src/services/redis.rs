use redis::{Client, Connection, aio::ConnectionManager};
use crate::errors::Result;

#[derive(Clone)]
pub struct RedisService {
    client: Client,
    connection_manager: ConnectionManager,
}

impl RedisService {
    pub async fn new(redis_url: &str) -> Result<Self> {
        let client = Client::open(redis_url)?;
        let connection_manager = ConnectionManager::new(client.clone()).await?;
        
        Ok(Self {
            client,
            connection_manager,
        })
    }

    pub fn client(&self) -> &Client {
        &self.client
    }

    pub fn connection_manager(&self) -> &ConnectionManager {
        &self.connection_manager
    }
}
