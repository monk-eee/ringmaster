-- ADR-0027: candidate_projection gains a nullable promoted_obligation_id,
-- populated by rebuild_candidate_projection from a "promoted" event's
-- payload -- the same carry-forward treatment source_fragment_id already
-- got in ADR-0015.
ALTER TABLE candidate_projection ADD COLUMN promoted_obligation_id UUID;
