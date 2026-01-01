use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Customer entity
#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, sqlx::FromRow)]
pub struct Customer {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub created_at: DateTime<Utc>,
}

/// Invoice entity with line items
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Invoice {
    pub id: Uuid,
    pub customer_id: Uuid,
    pub invoice_number: String,
    pub amount_cents: i64,
    pub status: String,
    pub issued_at: Option<DateTime<Utc>>,
    pub due_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub line_items: Vec<LineItem>,
}

/// Line item for invoice
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct LineItem {
    pub id: Uuid,
    pub description: String,
    pub quantity: i32,
    pub unit_price_cents: i64,
}
