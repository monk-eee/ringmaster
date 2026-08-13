-- ADR-0011: candidate_events is the immutable, append-only extraction log,
-- mirroring obligation_events (ADR-0005/ADR-0007). Event payloads carry the
-- docs/PRODUCT-SPEC.md SS6.3 extraction-object shape and the SS6.4
-- validation-state transition being recorded.
CREATE TABLE candidate_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    candidate_id UUID NOT NULL,
    event_type TEXT NOT NULL,
    payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    recorded_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX candidate_events_candidate_id_idx ON candidate_events (candidate_id);

CREATE FUNCTION reject_candidate_event_mutation() RETURNS trigger AS $$
BEGIN
    RAISE EXCEPTION 'candidate_events is append-only: % is not permitted (ADR-0011)', TG_OP;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER candidate_events_no_update
    BEFORE UPDATE ON candidate_events
    FOR EACH ROW EXECUTE FUNCTION reject_candidate_event_mutation();

CREATE TRIGGER candidate_events_no_delete
    BEFORE DELETE ON candidate_events
    FOR EACH ROW EXECUTE FUNCTION reject_candidate_event_mutation();

-- Derived read model (ADR-0011): always truncated and rebuilt entirely from
-- candidate_events; never written to directly and never authoritative over
-- the event log.
CREATE TABLE candidate_projection (
    candidate_id UUID PRIMARY KEY,
    candidate_type TEXT NOT NULL,
    statement TEXT NOT NULL,
    validation_state TEXT NOT NULL,
    confidence REAL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
