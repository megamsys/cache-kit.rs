use crate::models::Invoice;
use cache_kit::error::Result;
use cache_kit::feed::CacheFeed;

/// Invoice feeder for single entity caching
pub struct InvoiceFeeder {
    pub id: String,
    pub invoice: Option<Invoice>,
    pub cache_hit: bool,
}

impl InvoiceFeeder {
    pub fn new(id: String) -> Self {
        Self {
            id,
            invoice: None,
            cache_hit: false,
        }
    }
}

impl CacheFeed<Invoice> for InvoiceFeeder {
    fn entity_id(&mut self) -> String {
        self.id.clone()
    }

    fn feed(&mut self, entity: Option<Invoice>) {
        self.invoice = entity;
    }

    fn on_hit(&mut self, key: &str) -> Result<()> {
        self.cache_hit = true;
        tracing::debug!("Cache hit for {}", key);
        Ok(())
    }

    fn on_miss(&mut self, key: &str) -> Result<()> {
        self.cache_hit = false;
        tracing::debug!("Cache miss for {}", key);
        Ok(())
    }

    fn on_loaded(&mut self, entity: &Invoice) -> Result<()> {
        tracing::debug!("Invoice loaded: {}", entity.id);
        Ok(())
    }
}
