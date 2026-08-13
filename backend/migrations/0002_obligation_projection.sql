-- ADR-0005 (amended by ADR-0007): derived read model. Always truncated and
-- rebuilt entirely from obligation_events; never written to directly and
-- never treated as authoritative over the event log.
CREATE TABLE obligation_projection (
    obligation_id UUID PRIMARY KEY,
    status TEXT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
