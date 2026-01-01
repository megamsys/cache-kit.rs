use cache_kit::DataRepository;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{Product, User};

// Type alias for cache-kit Result
type CacheKitResult<T> = cache_kit::error::Result<T>;

/// Pure repository layer - SQLX + PostgreSQL, no cache logic
#[derive(Clone)]
pub struct UserRepository {
    pool: PgPool,
}

impl UserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, user: &User) -> CacheKitResult<User> {
        sqlx::query_as::<_, User>(
            "INSERT INTO users (id, username, email, created_at, updated_at) VALUES ($1, $2, $3, $4, $5) RETURNING *"
        )
        .bind(user.id)
        .bind(&user.username)
        .bind(&user.email)
        .bind(user.created_at)
        .bind(user.updated_at)
        .fetch_one(&self.pool)
        .await
        .map_err(|e: sqlx::Error| cache_kit::error::Error::from(e.to_string()))
    }

    pub async fn update(&self, user: &User) -> CacheKitResult<User> {
        sqlx::query_as::<_, User>(
            "UPDATE users SET username = $2, email = $3 WHERE id = $1 RETURNING *",
        )
        .bind(user.id)
        .bind(&user.username)
        .bind(&user.email)
        .fetch_one(&self.pool)
        .await
        .map_err(|e: sqlx::Error| cache_kit::error::Error::from(e.to_string()))
    }

    pub async fn delete(&self, id: &Uuid) -> CacheKitResult<()> {
        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(|e: sqlx::Error| cache_kit::error::Error::from(e.to_string()))
    }
}

impl DataRepository<User> for UserRepository {
    async fn fetch_by_id(&self, id: &String) -> CacheKitResult<Option<User>> {
        let uuid = Uuid::parse_str(id)
            .map_err(|e: uuid::Error| cache_kit::error::Error::from(e.to_string()))?;

        sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
            .bind(uuid)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e: sqlx::Error| cache_kit::error::Error::from(e.to_string()))
    }
}

#[derive(Clone)]
pub struct ProductRepository {
    pool: PgPool,
}

impl ProductRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, product: &Product) -> CacheKitResult<Product> {
        sqlx::query_as::<_, Product>(
            r#"INSERT INTO products (id, name, price, stock, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6) RETURNING *"#
        )
        .bind(product.id)
        .bind(&product.name)
        .bind(product.price)
        .bind(product.stock)
        .bind(product.created_at)
        .bind(product.updated_at)
        .fetch_one(&self.pool)
        .await
        .map_err(|e: sqlx::Error| cache_kit::error::Error::from(e.to_string()))
    }

    pub async fn update(&self, product: &Product) -> CacheKitResult<Product> {
        sqlx::query_as::<_, Product>(
            r#"UPDATE products SET name = $2, price = $3, stock = $4 WHERE id = $1 RETURNING *"#,
        )
        .bind(product.id)
        .bind(&product.name)
        .bind(product.price)
        .bind(product.stock)
        .fetch_one(&self.pool)
        .await
        .map_err(|e: sqlx::Error| cache_kit::error::Error::from(e.to_string()))
    }

    pub async fn delete(&self, id: &Uuid) -> CacheKitResult<()> {
        sqlx::query("DELETE FROM products WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(|e: sqlx::Error| cache_kit::error::Error::from(e.to_string()))
    }
}

impl DataRepository<Product> for ProductRepository {
    async fn fetch_by_id(&self, id: &String) -> CacheKitResult<Option<Product>> {
        let uuid = Uuid::parse_str(id)
            .map_err(|e: uuid::Error| cache_kit::error::Error::from(e.to_string()))?;

        sqlx::query_as::<_, Product>(r#"SELECT * FROM products WHERE id = $1"#)
            .bind(uuid)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e: sqlx::Error| cache_kit::error::Error::from(e.to_string()))
    }
}
