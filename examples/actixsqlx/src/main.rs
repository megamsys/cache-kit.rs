use actix_web::{web, App, HttpServer};
use cache_kit::{backend::InMemoryBackend, CacheService};
use cache_kit_actix_example::{
    repository::{ProductRepository, UserRepository},
    routes::{
        create_product, create_user, delete_product, delete_user, get_product, get_user,
        health_check, update_product, update_user, AppState,
    },
    services::{ProductService, UserService},
};
use sqlx::postgres::PgPoolOptions;
use std::env;
use std::sync::Arc;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Load environment variables
    dotenv::dotenv().ok();

    // Initialize logger
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    println!("\n╔════════════════════════════════════════════════════════╗");
    println!("║   Cache-Kit Actix + SQLX Example (Service Layer)      ║");
    println!("╚════════════════════════════════════════════════════════╝\n");

    // ========================================================================
    // Database Setup
    // ========================================================================

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set in .env file");

    println!("📦 Connecting to PostgreSQL...");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to create database pool");

    println!("✅ Database connection established");

    // Run migrations
    println!("🔄 Running database migrations...");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    println!("✅ Migrations completed");

    // ========================================================================
    // Dependency Injection - Build the application layers
    // ========================================================================

    println!("\n🏗️  Building application layers...");

    // Layer 1: Infrastructure (Cache Backend)
    let backend = InMemoryBackend::new();
    let cache_service = CacheService::new(backend);
    println!("  ✓ Cache backend initialized");

    // Layer 2: Data Access (Repositories with SQLX)
    // PgPool is already reference-counted internally, so cloning is cheap
    let user_repo = Arc::new(UserRepository::new(pool.clone()));
    let product_repo = Arc::new(ProductRepository::new(pool.clone()));
    println!("  ✓ Repositories initialized");

    // Layer 3: Business Logic (Services with Cache Integration)
    // CacheService is Clone-able and can be shared across services
    let user_service = Arc::new(UserService::new(user_repo.clone(), cache_service.clone()));
    let product_service = Arc::new(ProductService::new(
        product_repo.clone(),
        cache_service.clone(),
    ));
    println!("  ✓ Services initialized");

    // Layer 4: HTTP Layer (Application State)
    // Generic service container - just register services by type
    // To add a new service, just call: app_state.register(new_service);
    let mut app_state = AppState::new();
    app_state.register(user_service);
    app_state.register(product_service);
    let app_state = web::Data::new(app_state);
    println!("  ✓ Application state configured");

    // ========================================================================
    // Server Configuration
    // ========================================================================

    let host = env::var("SERVER_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = env::var("SERVER_PORT").unwrap_or_else(|_| "8080".to_string());
    let bind_address = format!("{}:{}", host, port);

    println!("\n🚀 Starting server at http://{}\n", bind_address);
    println!("📋 Architecture Layers:");
    println!("   ├─ Routes (HTTP)     → Clean REST handlers");
    println!("   ├─ Services (Logic)  → Business logic + Cache");
    println!("   ├─ Repository (Data) → SQLX + PostgreSQL");
    println!("   └─ Cache (Memory)    → In-memory backend\n");
    println!("🔗 Available endpoints:");
    println!("   ┌─ Health");
    println!("   │  └─ GET    /health");
    println!("   ├─ Users");
    println!("   │  ├─ GET    /users/:id");
    println!("   │  ├─ POST   /users");
    println!("   │  ├─ PUT    /users/:id");
    println!("   │  └─ DELETE /users/:id");
    println!("   └─ Products");
    println!("      ├─ GET    /products/:id");
    println!("      ├─ POST   /products");
    println!("      ├─ PUT    /products/:id");
    println!("      └─ DELETE /products/:id\n");

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            // Health check
            .route("/health", web::get().to(health_check))
            // User routes
            .route("/users/{id}", web::get().to(get_user))
            .route("/users", web::post().to(create_user))
            .route("/users/{id}", web::put().to(update_user))
            .route("/users/{id}", web::delete().to(delete_user))
            // Product routes
            .route("/products/{id}", web::get().to(get_product))
            .route("/products", web::post().to(create_product))
            .route("/products/{id}", web::put().to(update_product))
            .route("/products/{id}", web::delete().to(delete_product))
    })
    .bind(&bind_address)?
    .run()
    .await
}
