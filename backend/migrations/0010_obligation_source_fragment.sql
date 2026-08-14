-- ADR-0023: nullable source_fragment_id on obligation_projection, mirroring
-- ADR-0015's identical treatment of candidate_projection. obligation_events
-- payloads may now optionally carry source_fragment_id.
ALTER TABLE obligation_projection ADD COLUMN source_fragment_id UUID;
