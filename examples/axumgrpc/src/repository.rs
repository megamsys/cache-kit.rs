use crate::db::Database;
use crate::models::Invoice;
use cache_kit::DataRepository;
use sqlx::PgPool;

/// Invoice repository for cache-kit integration
pub struct InvoiceRepository {
    pool: PgPool,
}

impl InvoiceRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl DataRepository<Invoice> for InvoiceRepository {
    async fn fetch_by_id(&self, id: &String) -> cache_kit::Result<Option<Invoice>> {
        let invoice_id = uuid::Uuid::parse_str(id)
            .map_err(|e| cache_kit::Error::BackendError(format!("Invalid UUID: {}", e)))?;

        Database::get_invoice(&self.pool, &invoice_id)
            .await
            .map_err(|e| cache_kit::Error::BackendError(e.to_string()))
    }
}
