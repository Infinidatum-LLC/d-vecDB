use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use std::time::{SystemTime, UNIX_EPOCH, Duration};
use uuid::Uuid;

/// API Key with fine-grained permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: String,
    pub key: String,
    pub description: String,
    pub permissions: Vec<Permission>,
    pub created_at: u64,
    pub expires_at: Option<u64>,
    pub rate_limit: Option<RateLimit>,
}

/// Permission types for API keys
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Permission {
    /// Full cluster access (admin)
    ClusterAdmin,
    /// Read-only cluster access
    ClusterRead,
    /// Collection-level permissions
    Collection {
        name: String,
        access: AccessLevel,
    },
    /// Vector-level permissions (with filter)
    Vector {
        collection: String,
        filter: Option<serde_json::Value>,
        access: AccessLevel,
    },
}

/// Access levels
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AccessLevel {
    Read,
    Write,
    ReadWrite,
    Admin,
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimit {
    /// Requests per minute
    pub requests_per_minute: u32,
    /// Max concurrent requests
    pub max_concurrent: u32,
}

/// API Key manager
pub struct ApiKeyManager {
    keys: Arc<RwLock<HashMap<String, ApiKey>>>,
    rate_limiter: Arc<RwLock<HashMap<String, RateLimitState>>>,
}

#[derive(Debug, Clone)]
struct RateLimitState {
    requests: Vec<u64>, // Timestamps of recent requests
    concurrent: u32,
}

impl ApiKeyManager {
    /// Create a new API key manager
    pub fn new() -> Self {
        Self {
            keys: Arc::new(RwLock::new(HashMap::new())),
            rate_limiter: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new API key
    pub fn create_key(
        &self,
        description: String,
        permissions: Vec<Permission>,
        ttl_seconds: Option<u64>,
        rate_limit: Option<RateLimit>,
    ) -> ApiKey {
        let id = Uuid::new_v4().to_string();
        let key = format!("dvdb_{}", Uuid::new_v4().to_string().replace("-", ""));

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let expires_at = ttl_seconds.map(|ttl| now + ttl);

        let api_key = ApiKey {
            id: id.clone(),
            key: key.clone(),
            description,
            permissions,
            created_at: now,
            expires_at,
            rate_limit,
        };

        self.keys.write().insert(key.clone(), api_key.clone());

        api_key
    }

    /// Validate API key and check permissions
    pub fn validate_key(&self, key: &str) -> Result<ApiKey, AuthError> {
        let keys = self.keys.read();
        let api_key = keys.get(key).ok_or(AuthError::InvalidApiKey)?;

        // Check expiration
        if let Some(expires_at) = api_key.expires_at {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();

            if now > expires_at {
                return Err(AuthError::ExpiredApiKey);
            }
        }

        Ok(api_key.clone())
    }

    /// Check if API key has permission for an operation
    pub fn check_permission(
        &self,
        api_key: &ApiKey,
        operation: &Operation,
    ) -> Result<(), AuthError> {
        for permission in &api_key.permissions {
            match (permission, operation) {
                // Cluster admin has all permissions
                (Permission::ClusterAdmin, _) => return Ok(()),

                // Cluster read allows all read operations
                (Permission::ClusterRead, Operation::CollectionRead(_))
                | (Permission::ClusterRead, Operation::VectorRead(_, _)) => return Ok(()),

                // Collection-level permissions
                (
                    Permission::Collection { name, access },
                    Operation::CollectionRead(coll),
                ) if name == coll && access.allows_read() => return Ok(()),

                (
                    Permission::Collection { name, access },
                    Operation::CollectionWrite(coll),
                ) if name == coll && access.allows_write() => return Ok(()),

                (
                    Permission::Collection { name, access },
                    Operation::CollectionAdmin(coll),
                ) if name == coll && access.allows_admin() => return Ok(()),

                // Vector-level permissions
                (
                    Permission::Vector {
                        collection,
                        filter: _,
                        access,
                    },
                    Operation::VectorRead(coll, _),
                ) if collection == coll && access.allows_read() => return Ok(()),

                (
                    Permission::Vector {
                        collection,
                        filter: _,
                        access,
                    },
                    Operation::VectorWrite(coll, _),
                ) if collection == coll && access.allows_write() => return Ok(()),

                _ => continue,
            }
        }

        Err(AuthError::PermissionDenied)
    }

    /// Check rate limit
    pub fn check_rate_limit(&self, api_key: &ApiKey) -> Result<(), AuthError> {
        let rate_limit = match &api_key.rate_limit {
            Some(rl) => rl,
            None => return Ok(()), // No rate limit
        };

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut limiter = self.rate_limiter.write();
        let state = limiter
            .entry(api_key.key.clone())
            .or_insert_with(|| RateLimitState {
                requests: Vec::new(),
                concurrent: 0,
            });

        // Check concurrent requests
        if state.concurrent >= rate_limit.max_concurrent {
            return Err(AuthError::RateLimitExceeded);
        }

        // Clean up old requests (older than 1 minute)
        state.requests.retain(|&ts| now - ts < 60);

        // Check requests per minute
        if state.requests.len() >= rate_limit.requests_per_minute as usize {
            return Err(AuthError::RateLimitExceeded);
        }

        // Record this request
        state.requests.push(now);
        state.concurrent += 1;

        Ok(())
    }

    /// Mark request as completed (for concurrent tracking)
    pub fn complete_request(&self, key: &str) {
        let mut limiter = self.rate_limiter.write();
        if let Some(state) = limiter.get_mut(key) {
            state.concurrent = state.concurrent.saturating_sub(1);
        }
    }

    /// Revoke an API key
    pub fn revoke_key(&self, key: &str) -> Result<(), AuthError> {
        self.keys
            .write()
            .remove(key)
            .ok_or(AuthError::InvalidApiKey)?;
        self.rate_limiter.write().remove(key);
        Ok(())
    }

    /// List all API keys (without exposing the actual key)
    pub fn list_keys(&self) -> Vec<ApiKeySummary> {
        self.keys
            .read()
            .values()
            .map(|key| ApiKeySummary {
                id: key.id.clone(),
                description: key.description.clone(),
                permissions: key.permissions.clone(),
                created_at: key.created_at,
                expires_at: key.expires_at,
            })
            .collect()
    }
}

/// API Key summary (without the actual key value)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeySummary {
    pub id: String,
    pub description: String,
    pub permissions: Vec<Permission>,
    pub created_at: u64,
    pub expires_at: Option<u64>,
}

/// Operations that require permission checks
#[derive(Debug, Clone)]
pub enum Operation {
    CollectionRead(String),
    CollectionWrite(String),
    CollectionAdmin(String),
    VectorRead(String, Option<String>),
    VectorWrite(String, Option<String>),
}

/// Authentication errors
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Invalid API key")]
    InvalidApiKey,
    #[error("API key has expired")]
    ExpiredApiKey,
    #[error("Permission denied")]
    PermissionDenied,
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
}

impl AccessLevel {
    fn allows_read(&self) -> bool {
        matches!(
            self,
            AccessLevel::Read | AccessLevel::ReadWrite | AccessLevel::Admin
        )
    }

    fn allows_write(&self) -> bool {
        matches!(self, AccessLevel::Write | AccessLevel::ReadWrite | AccessLevel::Admin)
    }

    fn allows_admin(&self) -> bool {
        matches!(self, AccessLevel::Admin)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_key_creation() {
        let manager = ApiKeyManager::new();

        let key = manager.create_key(
            "Test key".to_string(),
            vec![Permission::ClusterRead],
            Some(3600),
            None,
        );

        assert!(key.key.starts_with("dvdb_"));
        assert_eq!(key.description, "Test key");
    }

    #[test]
    fn test_permission_check() {
        let manager = ApiKeyManager::new();

        let key = manager.create_key(
            "Collection key".to_string(),
            vec![Permission::Collection {
                name: "test".to_string(),
                access: AccessLevel::Read,
            }],
            None,
            None,
        );

        // Should allow read
        assert!(manager
            .check_permission(&key, &Operation::CollectionRead("test".to_string()))
            .is_ok());

        // Should deny write
        assert!(manager
            .check_permission(&key, &Operation::CollectionWrite("test".to_string()))
            .is_err());
    }

    #[test]
    fn test_rate_limiting() {
        let manager = ApiKeyManager::new();

        let key = manager.create_key(
            "Rate limited key".to_string(),
            vec![Permission::ClusterRead],
            None,
            Some(RateLimit {
                requests_per_minute: 5,
                max_concurrent: 2,
            }),
        );

        // First 5 requests should succeed
        for _ in 0..5 {
            assert!(manager.check_rate_limit(&key).is_ok());
        }

        // 6th request should fail
        assert!(manager.check_rate_limit(&key).is_err());
    }

    #[test]
    fn test_key_expiration() {
        let manager = ApiKeyManager::new();

        let key = manager.create_key(
            "Expired key".to_string(),
            vec![Permission::ClusterRead],
            Some(0), // Expire immediately
            None,
        );

        std::thread::sleep(std::time::Duration::from_secs(1));

        // Key should be expired
        assert!(matches!(
            manager.validate_key(&key.key),
            Err(AuthError::ExpiredApiKey)
        ));
    }
}
