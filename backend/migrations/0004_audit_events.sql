-- ADR-0008: system-level audit trail, distinct from the obligation domain
-- event log. Records who did what, through which policy outcome
-- (docs/PRODUCT-SPEC.md SS9.2, SS10). Immutable for the same reason
-- obligation_events is: an audit trail that can be edited is not evidence.
CREATE TABLE audit_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    actor TEXT NOT NULL,
    action TEXT NOT NULL,
    previous_state JSONB,
    new_state JSONB,
    source TEXT NOT NULL,
    policy_outcome TEXT NOT NULL,
    recorded_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX audit_events_actor_idx ON audit_events (actor);

CREATE FUNCTION reject_audit_event_mutation() RETURNS trigger AS $$
BEGIN
    RAISE EXCEPTION 'audit_events is append-only: % is not permitted (ADR-0008)', TG_OP;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER audit_events_no_update
    BEFORE UPDATE ON audit_events
    FOR EACH ROW EXECUTE FUNCTION reject_audit_event_mutation();

CREATE TRIGGER audit_events_no_delete
    BEFORE DELETE ON audit_events
    FOR EACH ROW EXECUTE FUNCTION reject_audit_event_mutation();
