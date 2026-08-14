-- ADR-0020: nullable due-date fields, the schema prerequisite for Epic E7's
-- attention/risk signals. obligation_events payloads may now optionally
-- carry hard_due_at/soft_due_at; rebuild_projection carries them forward.
ALTER TABLE obligation_projection ADD COLUMN hard_due_at TIMESTAMPTZ;
ALTER TABLE obligation_projection ADD COLUMN soft_due_at TIMESTAMPTZ;
