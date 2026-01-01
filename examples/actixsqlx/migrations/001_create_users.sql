-- ============================
-- USERS TABLE
-- ============================

CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY,  -- UUIDv7 generated in Rust
    username VARCHAR(100) NOT NULL UNIQUE,
    email VARCHAR(255) NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- ============================
-- UPDATED_AT TRIGGER FUNCTION
-- ============================

CREATE OR REPLACE FUNCTION update_users_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- ============================
-- TRIGGER (IDEMPOTENT)
-- ============================

DROP TRIGGER IF EXISTS update_users_updated_at ON users;

CREATE TRIGGER update_users_updated_at
BEFORE UPDATE ON users
FOR EACH ROW
EXECUTE FUNCTION update_users_updated_at_column();

-- ============================
-- OPTIONAL SEED DATA
-- ============================

INSERT INTO users (id, username, email, created_at)
VALUES
    ('018d8e5c-7b2a-7000-8000-000000000001'::uuid, 'alice', 'alice@example.com', '2024-01-15 10:00:00+00'),
    ('018d8e5c-7b2a-7000-8000-000000000002'::uuid, 'bob',   'bob@example.com',   '2024-01-20 14:30:00+00')
ON CONFLICT DO NOTHING;
