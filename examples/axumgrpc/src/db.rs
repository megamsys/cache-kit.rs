use crate::models::{Customer, Invoice, LineItem};
use sqlx::PgPool;
use uuid::Uuid;

pub struct Database;

impl Database {
    pub async fn get_invoice(pool: &PgPool, invoice_id: &Uuid) -> sqlx::Result<Option<Invoice>> {
        let row = sqlx::query_as::<
            _,
            (
                Uuid,
                Uuid,
                String,
                i64,
                String,
                Option<chrono::DateTime<chrono::Utc>>,
                Option<chrono::DateTime<chrono::Utc>>,
                chrono::DateTime<chrono::Utc>,
                chrono::DateTime<chrono::Utc>,
            ),
        >(
            r#"
            SELECT 
                id, customer_id, invoice_number, amount_cents, status,
                issued_at, due_at, created_at, updated_at
            FROM invoices
            WHERE id = $1
            "#,
        )
        .bind(invoice_id)
        .fetch_optional(pool)
        .await?;

        match row {
            None => Ok(None),
            Some((
                id,
                customer_id,
                invoice_number,
                amount_cents,
                status,
                issued_at,
                due_at,
                created_at,
                updated_at,
            )) => {
                let line_items = Self::get_line_items(pool, invoice_id).await?;
                Ok(Some(Invoice {
                    id,
                    customer_id,
                    invoice_number,
                    amount_cents,
                    status,
                    issued_at,
                    due_at,
                    created_at,
                    updated_at,
                    line_items,
                }))
            }
        }
    }

    pub async fn list_invoices(
        pool: &PgPool,
        customer_id: &Uuid,
        limit: i64,
        offset: i64,
    ) -> sqlx::Result<(Vec<Invoice>, i64)> {
        let total =
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM invoices WHERE customer_id = $1")
                .bind(customer_id)
                .fetch_one(pool)
                .await?;

        let rows = sqlx::query_as::<
            _,
            (
                Uuid,
                Uuid,
                String,
                i64,
                String,
                Option<chrono::DateTime<chrono::Utc>>,
                Option<chrono::DateTime<chrono::Utc>>,
                chrono::DateTime<chrono::Utc>,
                chrono::DateTime<chrono::Utc>,
            ),
        >(
            r#"
            SELECT 
                id, customer_id, invoice_number, amount_cents, status,
                issued_at, due_at, created_at, updated_at
            FROM invoices
            WHERE customer_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(customer_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;

        let mut invoices = Vec::new();
        for (
            id,
            customer_id,
            invoice_number,
            amount_cents,
            status,
            issued_at,
            due_at,
            created_at,
            updated_at,
        ) in rows
        {
            let line_items = Self::get_line_items(pool, &id).await?;
            invoices.push(Invoice {
                id,
                customer_id,
                invoice_number,
                amount_cents,
                status,
                issued_at,
                due_at,
                created_at,
                updated_at,
                line_items,
            });
        }

        Ok((invoices, total))
    }

    pub async fn create_invoice(
        pool: &PgPool,
        customer_id: &Uuid,
        invoice_number: &str,
        amount_cents: i64,
        due_at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> sqlx::Result<Invoice> {
        let id = Uuid::now_v7();
        let now = chrono::Utc::now();

        sqlx::query(
            r#"
            INSERT INTO invoices 
            (id, customer_id, invoice_number, amount_cents, status, issued_at, due_at, created_at, updated_at)
            VALUES ($1, $2, $3, $4, 'draft', $5, $6, $7, $8)
            "#
        )
        .bind(id)
        .bind(customer_id)
        .bind(invoice_number)
        .bind(amount_cents)
        .bind(now)
        .bind(due_at)
        .bind(now)
        .bind(now)
        .execute(pool)
        .await?;

        Ok(Invoice {
            id,
            customer_id: *customer_id,
            invoice_number: invoice_number.to_string(),
            amount_cents,
            status: "draft".to_string(),
            issued_at: Some(now),
            due_at,
            created_at: now,
            updated_at: now,
            line_items: Vec::new(),
        })
    }

    pub async fn add_line_item(
        pool: &PgPool,
        invoice_id: &Uuid,
        description: &str,
        quantity: i32,
        unit_price_cents: i64,
    ) -> sqlx::Result<LineItem> {
        let id = Uuid::now_v7();
        let now = chrono::Utc::now();

        sqlx::query(
            r#"
            INSERT INTO invoice_line_items 
            (id, invoice_id, description, quantity, unit_price_cents, created_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(id)
        .bind(invoice_id)
        .bind(description)
        .bind(quantity)
        .bind(unit_price_cents)
        .bind(now)
        .execute(pool)
        .await?;

        Ok(LineItem {
            id,
            description: description.to_string(),
            quantity,
            unit_price_cents,
        })
    }

    async fn get_line_items(pool: &PgPool, invoice_id: &Uuid) -> sqlx::Result<Vec<LineItem>> {
        let rows = sqlx::query_as::<_, (Uuid, String, i32, i64)>(
            "SELECT id, description, quantity, unit_price_cents FROM invoice_line_items WHERE invoice_id = $1"
        )
        .bind(invoice_id)
        .fetch_all(pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|(id, description, quantity, unit_price_cents)| LineItem {
                id,
                description,
                quantity,
                unit_price_cents,
            })
            .collect())
    }

    #[allow(dead_code)]
    pub async fn get_customer(pool: &PgPool, customer_id: &Uuid) -> sqlx::Result<Option<Customer>> {
        sqlx::query_as::<_, Customer>(
            "SELECT id, name, email, created_at FROM customers WHERE id = $1",
        )
        .bind(customer_id)
        .fetch_optional(pool)
        .await
    }

    #[allow(dead_code)]
    pub async fn create_customer(pool: &PgPool, name: &str, email: &str) -> sqlx::Result<Customer> {
        let id = Uuid::now_v7();
        let now = chrono::Utc::now();

        sqlx::query_as::<_, Customer>(
            r#"
            INSERT INTO customers (id, name, email, created_at)
            VALUES ($1, $2, $3, $4)
            RETURNING id, name, email, created_at
            "#,
        )
        .bind(id)
        .bind(name)
        .bind(email)
        .bind(now)
        .fetch_one(pool)
        .await
    }

    pub async fn update_invoice_status(
        pool: &PgPool,
        invoice_id: &Uuid,
        status: &str,
    ) -> sqlx::Result<Invoice> {
        let now = chrono::Utc::now();

        let (
            id,
            customer_id,
            invoice_number,
            amount_cents,
            invoice_status,
            issued_at,
            due_at,
            created_at,
            updated_at,
        ) = sqlx::query_as::<
            _,
            (
                Uuid,
                Uuid,
                String,
                i64,
                String,
                Option<chrono::DateTime<chrono::Utc>>,
                Option<chrono::DateTime<chrono::Utc>>,
                chrono::DateTime<chrono::Utc>,
                chrono::DateTime<chrono::Utc>,
            ),
        >(
            r#"
            UPDATE invoices
            SET status = $2, updated_at = $3
            WHERE id = $1
            RETURNING 
                id, customer_id, invoice_number, amount_cents, status,
                issued_at, due_at, created_at, updated_at
            "#,
        )
        .bind(invoice_id)
        .bind(status)
        .bind(now)
        .fetch_one(pool)
        .await?;

        let line_items = Self::get_line_items(pool, invoice_id).await?;

        Ok(Invoice {
            id,
            customer_id,
            invoice_number,
            amount_cents,
            status: invoice_status,
            issued_at,
            due_at,
            created_at,
            updated_at,
            line_items,
        })
    }
}
