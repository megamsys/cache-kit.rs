use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};
use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::Arc,
};

use crate::error::{ApiError, Result};
use crate::models::{Product, User};
use crate::services::{ProductService, UserService};

/// Application state - generic service container
/// Similar to how exonum manages services, but simpler for REST APIs
pub struct AppState {
    services: HashMap<TypeId, Arc<dyn Any + Send + Sync>>,
}

impl AppState {
    /// Create a new AppState
    pub fn new() -> Self {
        Self {
            services: HashMap::new(),
        }
    }

    /// Register a service by type
    /// Usage: state.register(user_service);
    pub fn register<T: 'static + Send + Sync>(&mut self, service: Arc<T>) {
        self.services.insert(TypeId::of::<T>(), service);
    }

    /// Get a service by type (returns a reference to avoid unnecessary Arc clone)
    /// Usage: let user_service = state.get::<UserService>()?;
    pub fn get<T: 'static + Send + Sync>(&self) -> Result<Arc<T>> {
        let type_id = TypeId::of::<T>();
        // Clone the Arc (cheap - just increments ref count) before downcasting
        // This is necessary because downcast requires ownership
        self.services
            .get(&type_id)
            .and_then(|s| {
                // Try to downcast the Arc<dyn Any> to Arc<T>
                // We need to clone to get ownership for downcast
                Arc::clone(s).downcast::<T>().ok()
            })
            .ok_or_else(|| {
                ApiError::new(actix_web::http::StatusCode::INTERNAL_SERVER_ERROR)
                    .title("Service Not Registered")
                    .detail(format!(
                        "Service {} not registered",
                        std::any::type_name::<T>()
                    ))
                    .error_code(5000)
            })
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Health Check
// ============================================================================

pub async fn health_check() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "status": "healthy",
        "service": "cache-kit-actix-sqlx-example"
    })))
}

// ============================================================================
// User Endpoints - Clean HTTP layer, delegates to service
// ============================================================================

/// GET /users/:id - Fetch user with caching
pub async fn get_user(path: web::Path<String>, data: web::Data<AppState>) -> Result<HttpResponse> {
    let user_id = path.into_inner();
    let user_service = data.get::<UserService>()?;

    match user_service.get(&user_id).await? {
        Some(user) => Ok(HttpResponse::Ok().json(user)),
        None => Err(ApiError::not_found()
            .detail(format!("User with ID {} not found", user_id))
            .error_code(1002)),
    }
}

/// Request body for creating a user
#[derive(Deserialize, Serialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub email: String,
}

/// POST /users - Create user
pub async fn create_user(
    req: web::Json<CreateUserRequest>,
    data: web::Data<AppState>,
) -> Result<HttpResponse> {
    // Move values from request instead of cloning
    let user = User::new(req.username.clone(), req.email.clone());
    let user_service = data.get::<UserService>()?;
    let created_user = user_service.create(&user).await?;
    Ok(HttpResponse::Created().json(created_user))
}

/// Request body for updating a user
#[derive(Deserialize, Serialize)]
pub struct UpdateUserRequest {
    pub username: String,
    pub email: String,
}

/// PUT /users/:id - Update user
pub async fn update_user(
    path: web::Path<String>,
    req: web::Json<UpdateUserRequest>,
    data: web::Data<AppState>,
) -> Result<HttpResponse> {
    let user_id = path.into_inner();

    let user_service = data.get::<UserService>()?;

    // First, fetch the existing user to get the full record
    let existing_user = user_service.get(&user_id).await?.ok_or_else(|| {
        ApiError::not_found()
            .detail(format!("User with ID {} not found", user_id))
            .error_code(1002)
    })?;

    // Create updated user with new values but keep id and created_at
    // Move values from request instead of cloning
    let UpdateUserRequest { username, email } = req.into_inner();
    let mut updated_user = existing_user;
    updated_user.username = username;
    updated_user.email = email;
    updated_user.updated_at = chrono::Utc::now();

    let result = user_service.update(&updated_user).await?;
    Ok(HttpResponse::Ok().json(result))
}

/// DELETE /users/:id - Delete user
pub async fn delete_user(
    path: web::Path<String>,
    data: web::Data<AppState>,
) -> Result<HttpResponse> {
    let user_id = path.into_inner();
    let user_service = data.get::<UserService>()?;
    user_service.delete(&user_id).await?;
    Ok(HttpResponse::NoContent().finish())
}

// ============================================================================
// Product Endpoints - Clean HTTP layer, delegates to service
// ============================================================================

/// GET /products/:id - Fetch product with caching
pub async fn get_product(
    path: web::Path<String>,
    data: web::Data<AppState>,
) -> Result<HttpResponse> {
    let product_id = path.into_inner();
    let product_service = data.get::<ProductService>()?;

    match product_service.get(&product_id).await? {
        Some(product) => Ok(HttpResponse::Ok().json(product)),
        None => Err(ApiError::not_found()
            .detail(format!("Product with ID {} not found", product_id))
            .error_code(2002)),
    }
}

/// Request body for creating a product
#[derive(Deserialize, Serialize)]
pub struct CreateProductRequest {
    pub name: String,
    pub price: i64, // Price in cents (e.g., 9999 = $99.99)
    pub stock: i64,
}

/// POST /products - Create product
pub async fn create_product(
    req: web::Json<CreateProductRequest>,
    data: web::Data<AppState>,
) -> Result<HttpResponse> {
    let product = Product::new(req.name.clone(), req.price, req.stock);
    let product_service = data.get::<ProductService>()?;
    let created_product = product_service.create(&product).await?;
    Ok(HttpResponse::Created().json(created_product))
}

/// Request body for updating a product
#[derive(Deserialize, Serialize)]
pub struct UpdateProductRequest {
    pub name: String,
    pub price: i64, // Price in cents (e.g., 9999 = $99.99)
    pub stock: i64,
}

/// PUT /products/:id - Update product
pub async fn update_product(
    path: web::Path<String>,
    req: web::Json<UpdateProductRequest>,
    data: web::Data<AppState>,
) -> Result<HttpResponse> {
    let product_id = path.into_inner();

    let product_service = data.get::<ProductService>()?;

    // First, fetch the existing product to get the full record
    let existing_product = product_service.get(&product_id).await?.ok_or_else(|| {
        ApiError::not_found()
            .detail(format!("Product with ID {} not found", product_id))
            .error_code(2002)
    })?;

    // Create updated product with new values but keep id and created_at
    // Move values from request instead of cloning
    let UpdateProductRequest { name, price, stock } = req.into_inner();
    let mut updated_product = existing_product;
    updated_product.name = name;
    updated_product.price = price;
    updated_product.stock = stock;
    updated_product.updated_at = chrono::Utc::now();

    let result = product_service.update(&updated_product).await?;
    Ok(HttpResponse::Ok().json(result))
}

/// DELETE /products/:id - Delete product
pub async fn delete_product(
    path: web::Path<String>,
    data: web::Data<AppState>,
) -> Result<HttpResponse> {
    let product_id = path.into_inner();
    let product_service = data.get::<ProductService>()?;
    product_service.delete(&product_id).await?;
    Ok(HttpResponse::NoContent().finish())
}
