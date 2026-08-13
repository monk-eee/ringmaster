-- ADR-0007: pgvector is a mandatory dependency for semantic retrieval
-- (docs/PRODUCT-SPEC.md SS3.1, SS9.2, SS15). This migration fails if the
-- extension cannot be created, which is how "mandatory" is enforced today.
CREATE EXTENSION IF NOT EXISTS vector;

-- Minimal embeddings table (docs/PRODUCT-SPEC.md SS9.2). The vector column
-- is left dimension-unconstrained: no embedding model has been chosen yet
-- (SS15 "Open design decisions"). A follow-up ADR-governed migration can add
-- a fixed dimension or CHECK constraint once one is.
CREATE TABLE embeddings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    entity_id UUID NOT NULL,
    entity_type TEXT NOT NULL,
    model_id TEXT NOT NULL,
    embedding vector NOT NULL,
    source_hash TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX embeddings_entity_idx ON embeddings (entity_type, entity_id);
