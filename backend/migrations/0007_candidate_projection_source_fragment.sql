-- ADR-0015: candidate_projection gains a nullable source_fragment_id,
-- populated by rebuild_candidate_projection from the candidate's own
-- extracted-event payload. Nullable so a candidate predating this column,
-- or genuinely without a source fragment, does not fail projection rebuild.
ALTER TABLE candidate_projection ADD COLUMN source_fragment_id UUID;
