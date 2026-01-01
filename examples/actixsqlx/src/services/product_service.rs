use cache_kit::{backend::InMemoryBackend, strategy::CacheStrategy, CacheFeed, CacheService};
use std::sync::Arc;
use uuid::Uuid;

use crate::error::{ApiError, Result};
use crate::models::Product;
use crate::repository::ProductRepository;

/// Service layer handles business logic and cache coordination
pub struct ProductService {
    repo: Arc<ProductRepository>,
    cache: CacheService<InMemoryBackend>,
}

/// Feeder for Product caching
struct ProductFeeder {
    id: String,
    product: Option<Product>,
}

impl CacheFeed<Product> for ProductFeeder {
    fn entity_id(&mut self) -> String {
        self.id.clone()
    }

    fn feed(&mut self, entity: Option<Product>) {
        self.product = entity;
    }
}

impl ProductService {
    pub fn new(repo: Arc<ProductRepository>, cache: CacheService<InMemoryBackend>) -> Self {
        Self { repo, cache }
    }

    /// Get product by ID with caching (Refresh strategy)
    pub async fn get(&self, id: &str) -> Result<Option<Product>> {
        log::info!("[Service] Getting product: {}", id);

        // Validate UUID format
        Uuid::parse_str(id).map_err(|_| {
            ApiError::bad_request()
                .detail(format!("Invalid UUID format: {}", id))
                .error_code(2001)
        })?;

        let mut feeder = ProductFeeder {
            id: id.to_string(),
            product: None,
        };

        self.cache
            .execute(&mut feeder, &*self.repo, CacheStrategy::Refresh)
            .await?;

        Ok(feeder.product)
    }

    /// Create product and cache it
    pub async fn create(&self, product: &Product) -> Result<Product> {
        log::info!("[Service] Creating product: {}", product.id);

        let created = self.repo.create(product).await?;

        // Cache the newly created product (non-critical)
        let mut feeder = ProductFeeder {
            id: created.id.to_string(),
            product: Some(created.clone()),
        };

        if let Err(e) = self
            .cache
            .execute(&mut feeder, &*self.repo, CacheStrategy::Refresh)
            .await
        {
            log::warn!("[Service] Failed to cache created product: {}", e);
        }

        Ok(created)
    }

    /// Update product and invalidate cache
    pub async fn update(&self, product: &Product) -> Result<Product> {
        log::info!("[Service] Updating product: {}", product.id);

        let updated = self.repo.update(product).await?;

        // Invalidate cache to force fresh fetch on next read (non-critical)
        let mut feeder = ProductFeeder {
            id: updated.id.to_string(),
            product: None,
        };

        if let Err(e) = self
            .cache
            .execute(&mut feeder, &*self.repo, CacheStrategy::Invalidate)
            .await
        {
            log::warn!("[Service] Failed to invalidate product cache: {}", e);
        }

        Ok(updated)
    }

    /// Delete product and remove from cache
    pub async fn delete(&self, id: &str) -> Result<()> {
        log::info!("[Service] Deleting product: {}", id);

        // Validate and parse UUID
        let uuid = Uuid::parse_str(id).map_err(|_| {
            ApiError::bad_request()
                .detail(format!("Invalid UUID format: {}", id))
                .error_code(2001)
        })?;

        self.repo.delete(&uuid).await?;

        // Invalidate cache entry (non-critical)
        let mut feeder = ProductFeeder {
            id: id.to_string(),
            product: None,
        };

        if let Err(e) = self
            .cache
            .execute(&mut feeder, &*self.repo, CacheStrategy::Invalidate)
            .await
        {
            log::warn!("[Service] Failed to invalidate product cache: {}", e);
        }

        Ok(())
    }
}
