-- ADR-0005 (amended by ADR-0007): the Obligation aggregate's history is an
-- immutable, append-only event log. Current/queryable state must be derived
-- via projections built from this log, never stored here as the source of
-- truth. Commitment is a promise subtype of Obligation
-- (docs/PRODUCT-SPEC.md SS5.1), not a separate aggregate.
CREATE TABLE obligation_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    obligation_id UUID NOT NULL,
    event_type TEXT NOT NULL,
    payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    recorded_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX obligation_events_obligation_id_idx ON obligation_events (obligation_id);

-- Enforce immutability at the database level: no code path, correct or
-- buggy, may mutate or delete an existing event row (ADR-0005).
CREATE FUNCTION reject_obligation_event_mutation() RETURNS trigger AS $$
BEGIN
    RAISE EXCEPTION 'obligation_events is append-only: % is not permitted (ADR-0005/ADR-0007)', TG_OP;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER obligation_events_no_update
    BEFORE UPDATE ON obligation_events
    FOR EACH ROW EXECUTE FUNCTION reject_obligation_event_mutation();

CREATE TRIGGER obligation_events_no_delete
    BEFORE DELETE ON obligation_events
    FOR EACH ROW EXECUTE FUNCTION reject_obligation_event_mutation();
