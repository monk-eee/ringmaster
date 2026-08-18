-- ADR-0056: dev-data cleanup. DRAFTED FOR HUMAN REVIEW. NOT INVOKED BY ANY
-- AUTOMATED PROCESS, NOT RUN BY ACCEPTING ADR-0056.
--
-- =====================================================================
-- DO NOT RUN THIS AGAINST A SHARED / ACTIVELY-USED DATABASE WITHOUT:
--   1. Running scripts/dev-data-report.sql first and reading its output
--      against THIS specific database, at THIS specific moment.
--   2. Confirming no other session/person is actively using the app
--      against this same database right now.
--   3. Taking a backup you are prepared to restore from
--      (e.g. `pg_dump`), since this deletes rows.
-- This repo's own operational-safety posture treats deleting data on
-- shared infrastructure as needing explicit confirmation before it runs,
-- not just before a final irreversible step (ADR-0056).
-- =====================================================================
--
-- Heuristic (intentionally conservative; NOT identical to
-- dev-data-report.sql's generic preview -- see below):
--   * nodes, non-person: canonical_text ILIKE '%test%'.
--   * nodes, person: canonical_text ILIKE '%test%' OR a known ADR-0073
--     Playwright browser-fixture name prefix (some don't contain "test").
--   * candidate_projection / obligation_events: NOT bulk-deleted by
--     content heuristic. Verified live 2026-08-18 against this database
--     that "%test%" matches genuine extracted content (see below);
--     obligation_projection's source_fragment_id IS NULL heuristic is
--     kept (the only real Obligation-creation path, ADR-0027's candidate
--     promotion, always carries one forward), but currently matches zero
--     rows here.
--
-- obligation_events / candidate_events / source_fragments are append-only
-- by design (ADR-0005/ADR-0008/ADR-0010): a DB-level trigger rejects any
-- UPDATE/DELETE against them. This script narrowly and temporarily
-- disables just those three tables' rejection triggers, for the duration
-- of one transaction, to remove the *specific, disclosed, matching* rows
-- identified above -- never a blanket TRUNCATE of unrelated history, and
-- never leaving the triggers disabled after the transaction ends.
--
-- Usage (only after the confirmation steps above):
--   psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -f scripts/dev-data-cleanup.sql
-- Then restart the backend (rebuild_projection runs at boot, main.rs) and
-- exercise any candidate-mutating route once (rebuild_candidate_projection
-- runs on-demand, not at boot, extraction.rs) so both projections reflect
-- the trimmed event log rather than being patched in place.

BEGIN;

-- Identify matching obligations/candidates from the CURRENT projection
-- state before anything is deleted.
CREATE TEMP TABLE _cleanup_obligation_ids AS
    SELECT obligation_id FROM obligation_projection WHERE source_fragment_id IS NULL;

-- candidate_projection.statement ILIKE '%test%' is NOT used here: verified
-- live 2026-08-18 that it matches genuine extracted content that merely
-- discusses testing (e.g. "Please test the PubDev (pre-prod) environment.",
-- "...no production-code-for-tests..."), not fixture pollution. Playwright
-- has no known candidate-fixture naming convention the way it does for
-- Person/Meeting nodes (see the prefixes below), so there is currently no
-- safe automated heuristic for candidates -- any candidate cleanup must be
-- reviewed statement-by-statement by hand, not deleted in bulk here.
CREATE TEMP TABLE _cleanup_candidate_ids AS
    SELECT candidate_id FROM candidate_projection WHERE false;

CREATE TEMP TABLE _cleanup_node_ids AS
    SELECT id FROM nodes
    WHERE (node_type <> 'person' AND canonical_text ILIKE '%test%')
       OR (node_type = 'person' AND (
              canonical_text ILIKE '%test%'
           OR canonical_text LIKE 'Needs Attention Filter Bare%'
           OR canonical_text LIKE 'Recent Interaction Person%'
           OR canonical_text LIKE 'Capped Recent Interactions Person%'
       ));

-- Append-only tables: disable the rejection triggers for this transaction
-- only, to remove exactly the identified rows.
ALTER TABLE obligation_events DISABLE TRIGGER obligation_events_no_delete;
ALTER TABLE candidate_events DISABLE TRIGGER candidate_events_no_delete;
ALTER TABLE source_fragments DISABLE TRIGGER source_fragments_no_delete;

DELETE FROM obligation_events WHERE obligation_id IN (SELECT obligation_id FROM _cleanup_obligation_ids);
DELETE FROM candidate_events WHERE candidate_id IN (SELECT candidate_id FROM _cleanup_candidate_ids);

-- Orphaned source fragments: no longer referenced by any surviving
-- obligation or candidate projection row.
DELETE FROM source_fragments
WHERE id IN (
    SELECT source_fragment_id FROM obligation_projection WHERE source_fragment_id IS NOT NULL
        AND obligation_id IN (SELECT obligation_id FROM _cleanup_obligation_ids)
    UNION
    SELECT source_fragment_id FROM candidate_projection WHERE source_fragment_id IS NOT NULL
        AND candidate_id IN (SELECT candidate_id FROM _cleanup_candidate_ids)
)
AND id NOT IN (
    SELECT source_fragment_id FROM obligation_projection
    WHERE source_fragment_id IS NOT NULL
        AND obligation_id NOT IN (SELECT obligation_id FROM _cleanup_obligation_ids)
    UNION
    SELECT source_fragment_id FROM candidate_projection
    WHERE source_fragment_id IS NOT NULL
        AND candidate_id NOT IN (SELECT candidate_id FROM _cleanup_candidate_ids)
);

ALTER TABLE obligation_events ENABLE TRIGGER obligation_events_no_delete;
ALTER TABLE candidate_events ENABLE TRIGGER candidate_events_no_delete;
ALTER TABLE source_fragments ENABLE TRIGGER source_fragments_no_delete;

-- Derived projections: delete directly for immediate effect (a future
-- rebuild from the now-trimmed event log would reach the same state).
DELETE FROM obligation_projection WHERE obligation_id IN (SELECT obligation_id FROM _cleanup_obligation_ids);
DELETE FROM candidate_projection WHERE candidate_id IN (SELECT candidate_id FROM _cleanup_candidate_ids);

-- Nodes matching the heuristic, plus edges that reference them (edges are
-- polymorphic -- from_id/to_id may be a node id or an obligation id,
-- ADR-0009 -- so both the node-heuristic and the obligation-heuristic
-- sets are covered here).
DELETE FROM edges
WHERE from_id IN (SELECT id FROM _cleanup_node_ids)
   OR to_id IN (SELECT id FROM _cleanup_node_ids)
   OR from_id IN (SELECT obligation_id FROM _cleanup_obligation_ids)
   OR to_id IN (SELECT obligation_id FROM _cleanup_obligation_ids);

DELETE FROM nodes WHERE id IN (SELECT id FROM _cleanup_node_ids);

-- audit_events is intentionally NOT touched: it is the security-relevant
-- record of what actions were taken (ADR-0008), not itself test-fixture
-- content, and deleting from it is out of this ADR's scope entirely.

COMMIT;
