-- ADR-0056: dev-data report. Read-only. Makes NO writes of any kind.
--
-- Reports, per table, how many rows match a disclosed, conservative
-- test-fixture heuristic, alongside how many do NOT match -- so a human
-- reviewer sees both sides before anything is ever deleted, not just a
-- number to trust.
--
-- Heuristic (deliberately conservative -- false negatives are safe here,
-- false positives are not):
--   * nodes / candidate_projection: canonical_text / statement contains
--     the case-insensitive substring "test".
--   * obligation_projection: source_fragment_id IS NULL. Today, the only
--     path that creates a real Obligation is promoting an accepted
--     candidate (ADR-0027), which always carries a source_fragment_id
--     forward from the promoted candidate's own fragment -- a null one
--     has never been through that real flow, only through a test that
--     inserted an obligation_events row directly.
--
-- Usage: psql "$DATABASE_URL" -f scripts/dev-data-report.sql
-- Safe to run at any time, against any environment, including the shared
-- dev database -- it is pure SELECT, nothing else.

\echo '--- nodes: matching "test" heuristic, by node_type ---'
SELECT node_type,
       count(*) FILTER (WHERE canonical_text ILIKE '%test%') AS matching_heuristic,
       count(*) FILTER (WHERE canonical_text NOT ILIKE '%test%') AS not_matching,
       count(*) AS total
FROM nodes
GROUP BY node_type
ORDER BY node_type;

\echo '--- obligation_projection: source_fragment_id IS NULL heuristic ---'
SELECT count(*) FILTER (WHERE source_fragment_id IS NULL) AS matching_heuristic,
       count(*) FILTER (WHERE source_fragment_id IS NOT NULL) AS not_matching,
       count(*) AS total
FROM obligation_projection;

\echo '--- candidate_projection: matching "test" heuristic, by candidate_type ---'
SELECT candidate_type,
       count(*) FILTER (WHERE statement ILIKE '%test%') AS matching_heuristic,
       count(*) FILTER (WHERE statement NOT ILIKE '%test%') AS not_matching,
       count(*) AS total
FROM candidate_projection
GROUP BY candidate_type
ORDER BY candidate_type;

\echo '--- edges: total count (heuristic does not classify edges directly; see linked node/obligation counts above) ---'
SELECT count(*) AS total FROM edges;

\echo '--- source_fragments: total count (immutable/append-only, ADR-0010; not itself deleted by the paired cleanup script) ---'
SELECT count(*) AS total FROM source_fragments;

\echo '--- audit_events: total count (immutable/append-only, ADR-0008; not itself deleted by the paired cleanup script) ---'
SELECT count(*) AS total FROM audit_events;
