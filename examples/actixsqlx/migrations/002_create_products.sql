-- ============================================================
-- Products Table
-- UUIDv7 is generated in Rust application code (NOT in database)
-- ============================================================

CREATE TABLE IF NOT EXISTS products (
    id UUID PRIMARY KEY,                      -- UUIDv7 from Rust
    name VARCHAR(200) NOT NULL,
    price BIGINT NOT NULL CHECK (price >= 0), -- Price in cents (e.g., 9999 = $99.99)
    stock BIGINT NOT NULL CHECK (stock >= 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- ============================================================
-- Indexes
-- ============================================================

-- For exact or prefix-based product name lookups
CREATE INDEX IF NOT EXISTS idx_products_name
    ON products (name);

-- For range queries or sorting by price (keep only if used)
CREATE INDEX IF NOT EXISTS idx_products_price
    ON products (price);

-- ============================================================
-- updated_at Trigger (Reusable & Migration-safe)
-- ============================================================

CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Drop trigger if exists, then create it
DROP TRIGGER IF EXISTS update_products_updated_at ON products;

CREATE TRIGGER update_products_updated_at
BEFORE UPDATE ON products
FOR EACH ROW
EXECUTE FUNCTION update_updated_at_column();

-- ============================================================
-- Sample Seed Data (Local / Dev only)
-- ============================================================

INSERT INTO products (id, name, price, stock, created_at) VALUES
    ('018d8e5c-7b2a-7000-8000-0000000000aa'::uuid, 'Laptop',   99999,  25, '2024-01-15 10:00:00+00'),  -- $999.99
    ('018d8e5c-7b2a-7000-8000-0000000000bb'::uuid, 'Keyboard',  7950, 120, '2024-01-20 14:30:00+00')  -- $79.50
ON CONFLICT DO NOTHING;
