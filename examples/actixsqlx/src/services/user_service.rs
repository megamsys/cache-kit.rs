use cache_kit::{backend::InMemoryBackend, strategy::CacheStrategy, CacheFeed, CacheService};
use std::sync::Arc;
use uuid::Uuid;

use crate::error::{ApiError, Result};
use crate::models::User;
use crate::repository::UserRepository;

/// Service layer handles business logic and cache coordination
pub struct UserService {
    repo: Arc<UserRepository>,
    cache: CacheService<InMemoryBackend>,
}

/// Feeder for User caching
struct UserFeeder {
    id: String,
    user: Option<User>,
}

impl CacheFeed<User> for UserFeeder {
    fn entity_id(&mut self) -> String {
        self.id.clone()
    }

    fn feed(&mut self, entity: Option<User>) {
        self.user = entity;
    }
}

impl UserService {
    pub fn new(repo: Arc<UserRepository>, cache: CacheService<InMemoryBackend>) -> Self {
        Self { repo, cache }
    }

    /// Get user by ID with caching (Refresh strategy)
    pub async fn get(&self, id: &str) -> Result<Option<User>> {
        log::info!("[Service] Getting user: {}", id);

        // Validate UUID format
        Uuid::parse_str(id).map_err(|_| {
            ApiError::bad_request()
                .detail(format!("Invalid UUID format: {}", id))
                .error_code(1001)
        })?;

        let mut feeder = UserFeeder {
            id: id.to_string(),
            user: None,
        };

        self.cache
            .execute(&mut feeder, &*self.repo, CacheStrategy::Refresh)
            .await?;

        Ok(feeder.user)
    }

    /// Create user and cache it
    pub async fn create(&self, user: &User) -> Result<User> {
        log::info!("[Service] Creating user: {}", user.id);

        let created = self.repo.create(user).await?;

        // Cache the newly created user (non-critical)
        let mut feeder = UserFeeder {
            id: created.id.to_string(),
            user: Some(created.clone()),
        };

        if let Err(e) = self
            .cache
            .execute(&mut feeder, &*self.repo, CacheStrategy::Refresh)
            .await
        {
            log::warn!("[Service] Failed to cache created user: {}", e);
        }

        Ok(created)
    }

    /// Update user and invalidate cache
    pub async fn update(&self, user: &User) -> Result<User> {
        log::info!("[Service] Updating user: {}", user.id);

        let updated = self.repo.update(user).await?;

        // Invalidate cache to force fresh fetch on next read (non-critical)
        let mut feeder = UserFeeder {
            id: updated.id.to_string(),
            user: None,
        };

        if let Err(e) = self
            .cache
            .execute(&mut feeder, &*self.repo, CacheStrategy::Invalidate)
            .await
        {
            log::warn!("[Service] Failed to invalidate user cache: {}", e);
        }

        Ok(updated)
    }

    /// Delete user and remove from cache
    pub async fn delete(&self, id: &str) -> Result<()> {
        log::info!("[Service] Deleting user: {}", id);

        // Validate and parse UUID
        let uuid = Uuid::parse_str(id).map_err(|_| {
            ApiError::bad_request()
                .detail(format!("Invalid UUID format: {}", id))
                .error_code(1001)
        })?;

        self.repo.delete(&uuid).await?;

        // Invalidate cache entry (non-critical)
        let mut feeder = UserFeeder {
            id: id.to_string(),
            user: None,
        };

        if let Err(e) = self
            .cache
            .execute(&mut feeder, &*self.repo, CacheStrategy::Invalidate)
            .await
        {
            log::warn!("[Service] Failed to invalidate user cache: {}", e);
        }

        Ok(())
    }
}
