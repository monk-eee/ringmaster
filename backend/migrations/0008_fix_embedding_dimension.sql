-- ADR-0018: nomic-embed-text (now the chosen model) produces 768-dimension
-- vectors. embeddings is empty in every environment, so this is a plain
-- ALTER, not a backfill (closes ADR-0007's deliberately left open gap).
ALTER TABLE embeddings ALTER COLUMN embedding TYPE vector(768);
