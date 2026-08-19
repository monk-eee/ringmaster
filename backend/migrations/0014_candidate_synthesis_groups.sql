-- ADR-0094: a synthesis group re-assembles same-source candidates that
-- describe the same underlying goal/commitment/topic into one clearer,
-- still-evidence-backed statement. Append-only, like source_fragments
-- (ADR-0010) and candidate_events (ADR-0011): a synthesis result is a dated
-- interpretation over existing evidence, not evidence itself -- revising it
-- means running synthesis again, never rewriting a prior result in place.
CREATE TABLE candidate_synthesis_groups (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_id UUID NOT NULL,
    synthesized_statement TEXT NOT NULL,
    candidate_type TEXT NOT NULL,
    member_candidate_ids UUID[] NOT NULL,
    synthesis_model TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX candidate_synthesis_groups_source_id_idx ON candidate_synthesis_groups (source_id);

CREATE FUNCTION reject_candidate_synthesis_group_mutation() RETURNS trigger AS $$
BEGIN
    RAISE EXCEPTION 'candidate_synthesis_groups is append-only: % is not permitted (ADR-0094)', TG_OP;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER candidate_synthesis_groups_no_update
    BEFORE UPDATE ON candidate_synthesis_groups
    FOR EACH ROW EXECUTE FUNCTION reject_candidate_synthesis_group_mutation();

CREATE TRIGGER candidate_synthesis_groups_no_delete
    BEFORE DELETE ON candidate_synthesis_groups
    FOR EACH ROW EXECUTE FUNCTION reject_candidate_synthesis_group_mutation();
