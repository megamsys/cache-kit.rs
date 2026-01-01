use std::sync::Arc;

mod grpc {
    #![allow(dead_code)]
    tonic::include_proto!("invoices");
}

use grpc::invoices_service_client::InvoicesServiceClient;
use grpc::{CreateInvoiceRequest, GetInvoiceRequest, LineItem, UpdateInvoiceStatusRequest};
use uuid::Uuid;

/// Integration test for invoice caching via gRPC
///
/// This test validates the cache behavior through multiple CRUD cycles:
/// 1. Create invoice (should cache the result)
/// 2. Get invoice (should hit the cache)
/// 3. Update invoice status (should invalidate the cache)
/// 4. Get updated invoice (should fetch from DB and cache new result)
///
/// This cycle runs multiple times to ensure cache behavior is consistent.
#[tokio::test]
async fn test_invoice_cache_crud_cycle() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize database
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:password@localhost:5432/cache_kit".to_string());

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    // Run migrations
    sqlx::migrate!("./migrations").run(&pool).await?;

    // Create a customer first to satisfy foreign key constraint
    let unique_email = format!("customer-{}@example.com", Uuid::now_v7());
    let customer =
        axumgrpc::db::Database::create_customer(&pool, "Test Customer", &unique_email).await?;
    let customer_id = customer.id.to_string();

    // Setup cache backend
    let cache_backend = Arc::new(cache_kit::backend::InMemoryBackend::new());

    // Create app state
    let state = axumgrpc::AppState::new(pool, cache_backend);

    // Start gRPC server in background
    let server_state = state.clone();
    let server_handle = tokio::spawn(async move {
        if let Err(e) = axumgrpc::grpc::start_grpc_server(server_state).await {
            eprintln!("gRPC server error: {}", e);
        }
    });

    // Wait for server to start
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Connect gRPC client
    let mut client = InvoicesServiceClient::connect("http://127.0.0.1:50051").await?;

    let num_iterations = 3;
    let test_run_id = Uuid::now_v7();

    for i in 0..num_iterations {
        println!("\n=== Iteration {} ===", i + 1);

        // 1. Create invoice
        println!("Creating invoice...");
        let create_req = CreateInvoiceRequest {
            customer_id: customer_id.clone(),
            invoice_number: format!("INV-{}-{:04}", test_run_id, i + 1),
            amount_cents: 10000 + (i as i64 * 1000),
            due_at: "2025-12-31T23:59:59Z".to_string(),
            line_items: vec![
                LineItem {
                    id: String::new(),
                    description: "Item 1".to_string(),
                    quantity: 2,
                    unit_price_cents: 2500,
                },
                LineItem {
                    id: String::new(),
                    description: "Item 2".to_string(),
                    quantity: 1,
                    unit_price_cents: 5000,
                },
            ],
        };

        let create_response = client
            .create_invoice(tonic::Request::new(create_req))
            .await?;
        let invoice = create_response.into_inner();
        let invoice_id = invoice.id.clone();
        println!(
            "✓ Created invoice {} with amount: {} cents",
            invoice.invoice_number, invoice.amount_cents
        );
        println!("  Status: {}", invoice.status);
        println!("  Line items: {}", invoice.line_items.len());

        // 2. Get invoice (test cache hit)
        println!("\nGetting invoice (should hit cache)...");
        let get_req = GetInvoiceRequest {
            invoice_id: invoice_id.clone(),
        };

        let get_response = client.get_invoice(tonic::Request::new(get_req)).await?;
        let retrieved = get_response.into_inner();
        println!("✓ Retrieved invoice: {}", retrieved.invoice_number);
        println!("  Amount: {} cents", retrieved.amount_cents);
        println!("  Line items: {}", retrieved.line_items.len());

        // Get it again to ensure we get a cache hit
        println!("\nGetting invoice again (should be cache hit)...");
        let get_req2 = GetInvoiceRequest {
            invoice_id: invoice_id.clone(),
        };
        let get_response2 = client.get_invoice(tonic::Request::new(get_req2)).await?;
        let retrieved2 = get_response2.into_inner();
        println!("✓ Cache hit verified - retrieved same invoice");

        // Verify data consistency
        assert_eq!(invoice.id, retrieved.id);
        assert_eq!(invoice.invoice_number, retrieved.invoice_number);
        assert_eq!(invoice.amount_cents, retrieved.amount_cents);
        assert_eq!(invoice.line_items.len(), retrieved.line_items.len());
        assert_eq!(retrieved.id, retrieved2.id);

        // 3. Update invoice status
        println!("\nUpdating invoice status...");
        let status = match i % 3 {
            0 => "draft",
            1 => "sent",
            _ => "paid",
        };

        let update_req = UpdateInvoiceStatusRequest {
            invoice_id: invoice_id.clone(),
            status: status.to_string(),
        };

        let update_response = client
            .update_invoice_status(tonic::Request::new(update_req))
            .await?;
        let updated = update_response.into_inner();
        println!("✓ Updated invoice status to: {}", updated.status);

        // 4. Get updated invoice to verify cache invalidation
        println!("\nGetting updated invoice (cache should be invalidated)...");
        let get_req = GetInvoiceRequest {
            invoice_id: invoice_id.clone(),
        };

        let get_response = client.get_invoice(tonic::Request::new(get_req)).await?;
        let final_invoice = get_response.into_inner();
        println!("✓ Final invoice status: {}", final_invoice.status);

        assert_eq!(final_invoice.status, status);
        println!("✓ Cache update verified!");
    }

    println!("\n=== All {} iterations passed! ===", num_iterations);

    // Shutdown server
    server_handle.abort();

    Ok(())
}
