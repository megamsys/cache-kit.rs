//! API-Level Integration Tests for cache-kit + Actix Example
//!
//! This test suite demonstrates:
//! - Cache-kit working with real Actix HTTP API
//! - CRUD operations (CREATE, READ, UPDATE, DELETE)
//! - Caching behavior across multiple requests
//! - Cache invalidation on mutations
//!
//! Run with: cargo test --test api_integration_tests -- --test-threads=1

use actix_web::{test, web, App};
use cache_kit::{backend::InMemoryBackend, CacheService};
use std::sync::Arc;

// Import from the example library
use cache_kit_actix_example::models::{Product, User};
use cache_kit_actix_example::repository::{ProductRepository, UserRepository};
use cache_kit_actix_example::routes::{
    create_product, create_user, delete_product, delete_user, get_product, get_user, health_check,
    update_product, update_user, AppState, CreateProductRequest, CreateUserRequest,
    UpdateProductRequest, UpdateUserRequest,
};
use cache_kit_actix_example::services::{ProductService, UserService};

// ============================================================================
// Test Setup
// ============================================================================

/// Setup test database and create app state
async fn setup_app_state() -> (Arc<UserService>, Arc<ProductService>) {
    // Get database URL from environment
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://cachekit:cachekit_dev@localhost:5432/cachekit_actix".to_string()
    });

    // Create connection pool
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to create database pool");

    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    // Clean up tables for test isolation
    sqlx::query("TRUNCATE TABLE users RESTART IDENTITY CASCADE")
        .execute(&pool)
        .await
        .expect("Failed to truncate users table");

    sqlx::query("TRUNCATE TABLE products RESTART IDENTITY CASCADE")
        .execute(&pool)
        .await
        .expect("Failed to truncate products table");

    // Create repositories
    let user_repo = Arc::new(UserRepository::new(pool.clone()));
    let product_repo = Arc::new(ProductRepository::new(pool));

    // Create cache service
    let backend = InMemoryBackend::new();
    let cache_service = CacheService::new(backend);

    // Create services with in-memory cache backend
    let user_service = Arc::new(UserService::new(user_repo, cache_service.clone()));
    let product_service = Arc::new(ProductService::new(product_repo, cache_service));

    (user_service, product_service)
}

// ============================================================================
// Health Check Tests
// ============================================================================

#[actix_web::test]
async fn test_health_check() {
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(AppState::new()))
            .route("/health", web::get().to(health_check)),
    )
    .await;

    let req = test::TestRequest::get().uri("/health").to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
}

// ============================================================================
// User Tests
// ============================================================================

#[actix_web::test]
async fn test_user_create() {
    let (user_service, product_service) = setup_app_state().await;

    let mut app_state = AppState::new();
    app_state.register(user_service);
    app_state.register(product_service);

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(app_state))
            .route("/users", web::post().to(create_user))
            .route("/users/{id}", web::get().to(get_user)),
    )
    .await;

    let req = CreateUserRequest {
        username: "alice".to_string(),
        email: "alice@example.com".to_string(),
    };

    let resp = test::TestRequest::post()
        .uri("/users")
        .set_json(&req)
        .to_request();

    let user: User = test::call_and_read_body_json(&app, resp).await;
    assert_eq!(user.username, "alice");
    assert_eq!(user.email, "alice@example.com");
}

#[actix_web::test]
async fn test_user_get_and_cache() {
    let (user_service, product_service) = setup_app_state().await;

    let mut app_state = AppState::new();
    app_state.register(user_service.clone());
    app_state.register(product_service);

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(app_state))
            .route("/users", web::post().to(create_user))
            .route("/users/{id}", web::get().to(get_user)),
    )
    .await;

    // Create a user
    let create_req = CreateUserRequest {
        username: "bob".to_string(),
        email: "bob@example.com".to_string(),
    };

    let resp = test::TestRequest::post()
        .uri("/users")
        .set_json(&create_req)
        .to_request();

    let created_user: User = test::call_and_read_body_json(&app, resp).await;
    let user_id = created_user.id.to_string();

    // First GET - should hit database
    let get_resp1 = test::TestRequest::get()
        .uri(&format!("/users/{}", user_id))
        .to_request();

    let user1: User = test::call_and_read_body_json(&app, get_resp1).await;
    assert_eq!(user1.username, "bob");

    // Second GET - should hit cache
    let get_resp2 = test::TestRequest::get()
        .uri(&format!("/users/{}", user_id))
        .to_request();

    let user2: User = test::call_and_read_body_json(&app, get_resp2).await;
    assert_eq!(user2.username, "bob");

    // Both should be identical (demonstrates caching worked)
    assert_eq!(user1.id, user2.id);
    assert_eq!(user1.username, user2.username);
}

#[actix_web::test]
async fn test_user_update_invalidates_cache() {
    let (user_service, product_service) = setup_app_state().await;

    let mut app_state = AppState::new();
    app_state.register(user_service.clone());
    app_state.register(product_service);

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(app_state))
            .route("/users", web::post().to(create_user))
            .route("/users/{id}", web::get().to(get_user))
            .route("/users/{id}", web::put().to(update_user)),
    )
    .await;

    // Create a user
    let create_req = CreateUserRequest {
        username: "charlie".to_string(),
        email: "charlie@example.com".to_string(),
    };

    let resp = test::TestRequest::post()
        .uri("/users")
        .set_json(&create_req)
        .to_request();

    let created_user: User = test::call_and_read_body_json(&app, resp).await;
    let user_id = created_user.id.to_string();

    // GET original
    let get_resp1 = test::TestRequest::get()
        .uri(&format!("/users/{}", user_id))
        .to_request();

    let user1: User = test::call_and_read_body_json(&app, get_resp1).await;
    assert_eq!(user1.username, "charlie");

    // UPDATE user
    let update_req = UpdateUserRequest {
        username: "charlie_updated".to_string(),
        email: "charlie_updated@example.com".to_string(),
    };

    let update_resp = test::TestRequest::put()
        .uri(&format!("/users/{}", user_id))
        .set_json(&update_req)
        .to_request();

    let updated_user: User = test::call_and_read_body_json(&app, update_resp).await;
    assert_eq!(updated_user.username, "charlie_updated");

    // GET after update should return updated data (cache invalidated)
    let get_resp2 = test::TestRequest::get()
        .uri(&format!("/users/{}", user_id))
        .to_request();

    let user2: User = test::call_and_read_body_json(&app, get_resp2).await;
    assert_eq!(user2.username, "charlie_updated");
}

#[actix_web::test]
async fn test_user_delete() {
    let (user_service, product_service) = setup_app_state().await;

    let mut app_state = AppState::new();
    app_state.register(user_service.clone());
    app_state.register(product_service);

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(app_state))
            .route("/users", web::post().to(create_user))
            .route("/users/{id}", web::get().to(get_user))
            .route("/users/{id}", web::delete().to(delete_user)),
    )
    .await;

    // Create a user
    let create_req = CreateUserRequest {
        username: "david".to_string(),
        email: "david@example.com".to_string(),
    };

    let resp = test::TestRequest::post()
        .uri("/users")
        .set_json(&create_req)
        .to_request();

    let created_user: User = test::call_and_read_body_json(&app, resp).await;
    let user_id = created_user.id.to_string();

    // DELETE user
    let delete_resp = test::TestRequest::delete()
        .uri(&format!("/users/{}", user_id))
        .to_request();

    let resp = test::call_service(&app, delete_resp).await;
    assert_eq!(resp.status(), actix_web::http::StatusCode::NO_CONTENT);

    // GET should return 404
    let get_resp = test::TestRequest::get()
        .uri(&format!("/users/{}", user_id))
        .to_request();

    let resp = test::call_service(&app, get_resp).await;
    assert_eq!(resp.status(), actix_web::http::StatusCode::NOT_FOUND);
}

// ============================================================================
// Product Tests
// ============================================================================

#[actix_web::test]
async fn test_product_create() {
    let (user_service, product_service) = setup_app_state().await;

    let mut app_state = AppState::new();
    app_state.register(user_service);
    app_state.register(product_service);

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(app_state))
            .route("/products", web::post().to(create_product)),
    )
    .await;

    let req = CreateProductRequest {
        name: "Widget".to_string(),
        price: 9999, // $99.99 in cents
        stock: 100,
    };

    let resp = test::TestRequest::post()
        .uri("/products")
        .set_json(&req)
        .to_request();

    let resp_obj = test::call_service(&app, resp).await;
    let status = resp_obj.status();
    let body = test::read_body(resp_obj).await;
    let body_str = String::from_utf8(body.to_vec()).unwrap();
    eprintln!("Status: {}", status);
    eprintln!("Body: {}", body_str);

    let product: Product = serde_json::from_str(&body_str).unwrap();
    assert_eq!(product.name, "Widget");
    assert_eq!(product.price, 9999); // $99.99 in cents
    assert_eq!(product.stock, 100);
}

#[actix_web::test]
async fn test_product_get_and_cache() {
    let (user_service, product_service) = setup_app_state().await;

    let mut app_state = AppState::new();
    app_state.register(user_service);
    app_state.register(product_service.clone());

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(app_state))
            .route("/products", web::post().to(create_product))
            .route("/products/{id}", web::get().to(get_product)),
    )
    .await;

    // Create a product
    let create_req = CreateProductRequest {
        name: "Gadget".to_string(),
        price: 4999, // $49.99 in cents
        stock: 50,
    };

    let resp = test::TestRequest::post()
        .uri("/products")
        .set_json(&create_req)
        .to_request();

    let created_product: Product = test::call_and_read_body_json(&app, resp).await;
    let product_id = created_product.id.to_string();

    // First GET - should hit database
    let get_resp1 = test::TestRequest::get()
        .uri(&format!("/products/{}", product_id))
        .to_request();

    let resp_obj = test::call_service(&app, get_resp1).await;
    let status = resp_obj.status();
    let body = test::read_body(resp_obj).await;
    let body_str = String::from_utf8(body.to_vec()).unwrap();
    eprintln!("GET Status: {}", status);
    eprintln!("GET Body: {}", body_str);

    let product1: Product = serde_json::from_str(&body_str).unwrap();
    assert_eq!(product1.name, "Gadget");

    // Second GET - should hit cache
    let get_resp2 = test::TestRequest::get()
        .uri(&format!("/products/{}", product_id))
        .to_request();

    let product2: Product = test::call_and_read_body_json(&app, get_resp2).await;
    assert_eq!(product2.name, "Gadget");

    // Both should be identical
    assert_eq!(product1.id, product2.id);
}

#[actix_web::test]
async fn test_product_update_invalidates_cache() {
    let (user_service, product_service) = setup_app_state().await;

    let mut app_state = AppState::new();
    app_state.register(user_service);
    app_state.register(product_service.clone());

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(app_state))
            .route("/products", web::post().to(create_product))
            .route("/products/{id}", web::get().to(get_product))
            .route("/products/{id}", web::put().to(update_product)),
    )
    .await;

    // Create a product
    let create_req = CreateProductRequest {
        name: "Tool".to_string(),
        price: 2999, // $29.99 in cents
        stock: 200,
    };

    let resp = test::TestRequest::post()
        .uri("/products")
        .set_json(&create_req)
        .to_request();

    let created_product: Product = test::call_and_read_body_json(&app, resp).await;
    let product_id = created_product.id.to_string();

    // GET original
    let get_resp1 = test::TestRequest::get()
        .uri(&format!("/products/{}", product_id))
        .to_request();

    let product1: Product = test::call_and_read_body_json(&app, get_resp1).await;
    assert_eq!(product1.stock, 200);

    // UPDATE product
    let update_req = UpdateProductRequest {
        name: "Tool Pro".to_string(),
        price: 3999, // $39.99 in cents
        stock: 150,
    };

    let update_resp = test::TestRequest::put()
        .uri(&format!("/products/{}", product_id))
        .set_json(&update_req)
        .to_request();

    let updated_product: Product = test::call_and_read_body_json(&app, update_resp).await;
    assert_eq!(updated_product.stock, 150);

    // GET after update should return updated data
    let get_resp2 = test::TestRequest::get()
        .uri(&format!("/products/{}", product_id))
        .to_request();

    let product2: Product = test::call_and_read_body_json(&app, get_resp2).await;
    assert_eq!(product2.stock, 150);
    assert_eq!(product2.name, "Tool Pro");
}

#[actix_web::test]
async fn test_product_delete() {
    let (user_service, product_service) = setup_app_state().await;

    let mut app_state = AppState::new();
    app_state.register(user_service);
    app_state.register(product_service);

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(app_state))
            .route("/products", web::post().to(create_product))
            .route("/products/{id}", web::get().to(get_product))
            .route("/products/{id}", web::delete().to(delete_product)),
    )
    .await;

    // Create a product
    let create_req = CreateProductRequest {
        name: "Doomed Product".to_string(),
        price: 1999, // $19.99 in cents
        stock: 10,
    };

    let resp = test::TestRequest::post()
        .uri("/products")
        .set_json(&create_req)
        .to_request();

    let created_product: Product = test::call_and_read_body_json(&app, resp).await;
    let product_id = created_product.id.to_string();

    // DELETE product
    let delete_resp = test::TestRequest::delete()
        .uri(&format!("/products/{}", product_id))
        .to_request();

    let resp = test::call_service(&app, delete_resp).await;
    assert_eq!(resp.status(), actix_web::http::StatusCode::NO_CONTENT);

    // GET should return 404
    let get_resp = test::TestRequest::get()
        .uri(&format!("/products/{}", product_id))
        .to_request();

    let resp = test::call_service(&app, get_resp).await;
    assert_eq!(resp.status(), actix_web::http::StatusCode::NOT_FOUND);
}
