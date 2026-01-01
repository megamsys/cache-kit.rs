use crate::{
    db::Database, feeders::InvoiceFeeder, models::Invoice, repository::InvoiceRepository, AppState,
};
use cache_kit::strategy::CacheStrategy;
use tonic::{transport::Server, Response, Status};
use uuid::Uuid;

pub mod invoices {
    #![allow(dead_code)]
    tonic::include_proto!("invoices");
}

use invoices::{
    invoices_service_server::{InvoicesService, InvoicesServiceServer},
    CreateInvoiceRequest, GetInvoiceRequest, InvoiceResponse, LineItem, ListInvoicesRequest,
    ListInvoicesResponse, UpdateInvoiceStatusRequest,
};

pub struct InvoicesHandler {
    state: AppState,
}

#[tonic::async_trait]
impl InvoicesService for InvoicesHandler {
    async fn get_invoice(
        &self,
        request: tonic::Request<GetInvoiceRequest>,
    ) -> Result<Response<InvoiceResponse>, Status> {
        let req = request.into_inner();
        let invoice_id = Uuid::parse_str(&req.invoice_id)
            .map_err(|_| Status::invalid_argument("Invalid invoice ID"))?;

        // Use cache with Refresh strategy (cache-first, fallback to DB)
        let repo = InvoiceRepository::new(self.state.db.clone());
        let mut feeder = InvoiceFeeder::new(invoice_id.to_string());

        self.state
            .cache_service
            .execute::<Invoice, _, _>(&mut feeder, &repo, CacheStrategy::Refresh)
            .await
            .map_err(|e| Status::internal(format!("Cache error: {}", e)))?;

        let invoice = feeder
            .invoice
            .ok_or_else(|| Status::not_found("Invoice not found"))?;

        let cache_source = if feeder.cache_hit {
            "cache"
        } else {
            "database"
        };
        tracing::info!("Retrieved invoice {} from {}", invoice_id, cache_source);

        Ok(Response::new(invoice_to_proto(&invoice)))
    }

    async fn list_invoices(
        &self,
        request: tonic::Request<ListInvoicesRequest>,
    ) -> Result<Response<ListInvoicesResponse>, Status> {
        let req = request.into_inner();
        let customer_id = Uuid::parse_str(&req.customer_id)
            .map_err(|_| Status::invalid_argument("Invalid customer ID"))?;

        let limit = (req.limit as i64).clamp(1, 100);
        let offset = (req.offset as i64).clamp(0, i64::MAX);

        let (invoices, total) =
            Database::list_invoices(&self.state.db, &customer_id, limit, offset)
                .await
                .map_err(|e| Status::internal(e.to_string()))?;

        let invoices = invoices
            .into_iter()
            .map(|inv| invoice_to_proto(&inv))
            .collect();

        Ok(Response::new(ListInvoicesResponse {
            invoices,
            total: total as i32,
        }))
    }

    async fn create_invoice(
        &self,
        request: tonic::Request<CreateInvoiceRequest>,
    ) -> Result<Response<InvoiceResponse>, Status> {
        let req = request.into_inner();
        let customer_id = Uuid::parse_str(&req.customer_id)
            .map_err(|_| Status::invalid_argument("Invalid customer ID"))?;

        let due_at = if req.due_at.is_empty() {
            None
        } else {
            Some(
                chrono::DateTime::parse_from_rfc3339(&req.due_at)
                    .map_err(|_| Status::invalid_argument("Invalid due_at format"))?
                    .with_timezone(&chrono::Utc),
            )
        };

        let invoice = Database::create_invoice(
            &self.state.db,
            &customer_id,
            &req.invoice_number,
            req.amount_cents,
            due_at,
        )
        .await
        .map_err(|e| Status::internal(e.to_string()))?;

        for item in req.line_items {
            Database::add_line_item(
                &self.state.db,
                &invoice.id,
                &item.description,
                item.quantity,
                item.unit_price_cents,
            )
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        }

        // Fetch complete invoice with line items and cache it
        let repo = InvoiceRepository::new(self.state.db.clone());
        let mut feeder = InvoiceFeeder::new(invoice.id.to_string());
        self.state
            .cache_service
            .execute::<Invoice, _, _>(&mut feeder, &repo, CacheStrategy::Refresh)
            .await
            .map_err(|e| Status::internal(format!("Cache error: {}", e)))?;

        let invoice = feeder
            .invoice
            .ok_or_else(|| Status::internal("Invoice not found after creation"))?;

        tracing::info!("Created and cached invoice {}", invoice.id);

        Ok(Response::new(invoice_to_proto(&invoice)))
    }

    async fn update_invoice_status(
        &self,
        request: tonic::Request<UpdateInvoiceStatusRequest>,
    ) -> Result<Response<InvoiceResponse>, Status> {
        let req = request.into_inner();
        let invoice_id = Uuid::parse_str(&req.invoice_id)
            .map_err(|_| Status::invalid_argument("Invalid invoice ID"))?;

        let invoice = Database::update_invoice_status(&self.state.db, &invoice_id, &req.status)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        // Invalidate cache and refresh with updated data
        let repo = InvoiceRepository::new(self.state.db.clone());
        let mut feeder = InvoiceFeeder::new(invoice_id.to_string());
        if let Err(e) = self
            .state
            .cache_service
            .execute::<Invoice, _, _>(&mut feeder, &repo, CacheStrategy::Invalidate)
            .await
        {
            tracing::warn!(
                "Failed to invalidate cache for invoice {}: {}",
                invoice_id,
                e
            );
        }

        tracing::info!("Updated and invalidated cache for invoice {}", invoice_id);

        Ok(Response::new(invoice_to_proto(&invoice)))
    }
}

fn invoice_to_proto(invoice: &Invoice) -> InvoiceResponse {
    InvoiceResponse {
        id: invoice.id.to_string(),
        customer_id: invoice.customer_id.to_string(),
        invoice_number: invoice.invoice_number.clone(),
        amount_cents: invoice.amount_cents,
        status: invoice.status.clone(),
        issued_at: invoice
            .issued_at
            .as_ref()
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_default(),
        due_at: invoice
            .due_at
            .as_ref()
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_default(),
        line_items: invoice
            .line_items
            .iter()
            .map(|li| LineItem {
                id: li.id.to_string(),
                description: li.description.clone(),
                quantity: li.quantity,
                unit_price_cents: li.unit_price_cents,
            })
            .collect(),
        created_at: invoice.created_at.to_rfc3339(),
        updated_at: invoice.updated_at.to_rfc3339(),
    }
}

pub async fn start_grpc_server(state: AppState) -> anyhow::Result<()> {
    let addr = "127.0.0.1:50051".parse()?;
    let handler = InvoicesHandler { state };

    tracing::info!("gRPC server listening on {}", addr);

    Server::builder()
        .add_service(InvoicesServiceServer::new(handler))
        .serve(addr)
        .await?;

    Ok(())
}
