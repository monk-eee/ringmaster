-- ADR-0036: source_fragments had no explicit ordering column; created_at
-- alone is unreliable because ingest_transcript writes every fragment in
-- one transaction, and Postgres now() returns the transaction start time,
-- not a per-statement time -- ties are the common case, not an edge case.
ALTER TABLE source_fragments ADD COLUMN sequence INTEGER;
