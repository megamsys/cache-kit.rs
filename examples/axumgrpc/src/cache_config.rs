use crate::models::Invoice;
use cache_kit::CacheEntity;
use uuid::Uuid;

/// Cache configuration for Invoice
impl CacheEntity for Invoice {
    type Key = String;

    fn cache_key(&self) -> Self::Key {
        format!("{}:{}", Self::cache_prefix(), self.id)
    }

    fn cache_prefix() -> &'static str {
        "invoice"
    }
}

/// Helper to create a cache key for listing invoices by customer
#[allow(dead_code)]
pub fn invoice_list_cache_key(customer_id: &Uuid, limit: i64, offset: i64) -> String {
    format!("invoice:list:{}:{}:{}", customer_id, limit, offset)
}
