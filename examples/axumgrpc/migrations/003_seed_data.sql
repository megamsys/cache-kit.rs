-- ============================================================
-- Sample Seed Data (Local / Dev only)
-- ============================================================

INSERT INTO customers (id, name, email, created_at) VALUES
    ('550e8400-e29b-41d4-a716-446655440000'::uuid, 'Acme Corp', 'contact@acme.example.com', '2024-01-15 10:00:00+00'),
    ('550e8400-e29b-41d4-a716-446655440001'::uuid, 'TechStart Inc', 'billing@techstart.example.com', '2024-01-20 14:30:00+00')
ON CONFLICT DO NOTHING;

INSERT INTO invoices (id, customer_id, invoice_number, amount_cents, status, issued_at, due_at, created_at, updated_at) VALUES
    ('019b747b-a331-73b3-acfe-867a5d0c3ded'::uuid, '550e8400-e29b-41d4-a716-446655440000'::uuid, 'INV-001', 10000, 'draft', '2025-12-31 12:56:58.417477+00', '2025-12-31 23:59:59+00', '2025-12-31 12:56:58.417477+00', '2025-12-31 12:56:58.417477+00'),
    ('019b747b-a331-73b3-acfe-867a5d0c3dee'::uuid, '550e8400-e29b-41d4-a716-446655440000'::uuid, 'INV-002', 25000, 'sent', '2025-12-25 09:00:00+00', '2026-01-25 09:00:00+00', '2025-12-25 09:00:00+00', '2025-12-25 09:00:00+00'),
    ('019b747b-a331-73b3-acfe-867a5d0c3def'::uuid, '550e8400-e29b-41d4-a716-446655440001'::uuid, 'INV-003', 15000, 'paid', '2025-12-20 11:30:00+00', '2025-12-31 00:00:00+00', '2025-12-20 11:30:00+00', '2025-12-30 15:45:00+00')
ON CONFLICT DO NOTHING;

INSERT INTO invoice_line_items (id, invoice_id, description, quantity, unit_price_cents, created_at) VALUES
    ('019b747b-a335-7983-b1ff-4df31c9c0318'::uuid, '019b747b-a331-73b3-acfe-867a5d0c3ded'::uuid, 'Test item', 1, 10000, '2025-12-31 12:56:58.417477+00'),
    ('019b747b-a335-7983-b1ff-4df31c9c0319'::uuid, '019b747b-a331-73b3-acfe-867a5d0c3dee'::uuid, 'Consulting Services', 5, 5000, '2025-12-25 09:00:00+00'),
    ('019b747b-a335-7983-b1ff-4df31c9c0320'::uuid, '019b747b-a331-73b3-acfe-867a5d0c3def'::uuid, 'Software License', 3, 5000, '2025-12-20 11:30:00+00')
ON CONFLICT DO NOTHING;
