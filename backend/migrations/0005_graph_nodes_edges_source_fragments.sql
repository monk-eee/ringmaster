-- ADR-0009: generic graph substrate for the 12 non-Obligation node types
-- (docs/PRODUCT-SPEC.md SS5.2, SS9.2). Ordinary mutable rows, deliberately
-- not event-sourced: Obligation (ADR-0005/ADR-0007) and audit_events
-- (ADR-0008) carry that guarantee; these entities do not need it.
CREATE TABLE nodes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    node_type TEXT NOT NULL,
    canonical_text TEXT NOT NULL,
    attributes JSONB NOT NULL DEFAULT '{}'::jsonb,
    lifecycle_state TEXT NOT NULL DEFAULT 'active',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX nodes_node_type_idx ON nodes (node_type);

-- from_id/to_id are polymorphic: either a nodes.id or an Obligation's
-- obligation_id. No foreign key enforces this; correctness is an
-- application-layer responsibility until real writers exist (ADR-0009).
CREATE TABLE edges (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    from_id UUID NOT NULL,
    to_id UUID NOT NULL,
    edge_type TEXT NOT NULL,
    confidence REAL,
    valid_from TIMESTAMPTZ,
    valid_to TIMESTAMPTZ,
    provenance JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX edges_from_id_idx ON edges (from_id);
CREATE INDEX edges_to_id_idx ON edges (to_id);
CREATE INDEX edges_edge_type_idx ON edges (edge_type);

-- Bounded source passages (transcript spans or document excerpts),
-- docs/PRODUCT-SPEC.md SS9.2 and the SS6.3 extraction object shape.
-- Immutable (ADR-0010): a quote must not be silently altered after capture,
-- the same way commitment/audit events cannot be (ADR-0005/ADR-0008).
-- Corrections belong to a future Obligation/candidate event, never an edit
-- to the fragment itself.
CREATE TABLE source_fragments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_id UUID NOT NULL,
    text TEXT NOT NULL,
    speaker TEXT,
    start_ms BIGINT,
    end_ms BIGINT,
    classification TEXT,
    hash TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX source_fragments_source_id_idx ON source_fragments (source_id);

CREATE FUNCTION reject_source_fragment_mutation() RETURNS trigger AS $$
BEGIN
    RAISE EXCEPTION 'source_fragments is append-only: % is not permitted (ADR-0010)', TG_OP;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER source_fragments_no_update
    BEFORE UPDATE ON source_fragments
    FOR EACH ROW EXECUTE FUNCTION reject_source_fragment_mutation();

CREATE TRIGGER source_fragments_no_delete
    BEFORE DELETE ON source_fragments
    FOR EACH ROW EXECUTE FUNCTION reject_source_fragment_mutation();

