-- ADR-0040: real-world event time, distinct from created_at (when
-- Ringmaster stored it). Nullable so existing rows are unaffected; every
-- node this ADR's ingestion path creates sets it (enforced at the
-- application layer, not by a NOT NULL constraint).
ALTER TABLE nodes ADD COLUMN occurred_at TIMESTAMPTZ;
