export type Obligation = {
  obligation_id: string;
  status: string;
  updated_at: string;
  hard_due_at: string | null;
  soft_due_at: string | null;
  source_fragment_id: string | null;
  source_text: string | null;
};

export type Candidate = {
  candidate_id: string;
  candidate_type: string;
  statement: string;
  validation_state: string;
  confidence: number;
  source_fragment_id: string | null;
  promoted_obligation_id: string | null;
  source_text: string | null;
  speaker: string | null;
};

export type SearchResult = {
  source_fragment_id: string;
  text: string;
  speaker: string | null;
  similarity: number;
};

export type DailyBriefItem = {
  obligation_id: string;
  status: string;
  hard_due_at: string | null;
  soft_due_at: string | null;
  updated_at: string;
  reason: string;
  source_fragment_id: string | null;
  source_text: string | null;
  risk_signals: RiskSignal[];
};

export type RiskSignal = {
  signal: string;
  explanation: string;
};

export type TimeHorizonItem = {
  obligation_id: string;
  status: string;
  hard_due_at: string | null;
  soft_due_at: string | null;
  source_fragment_id: string | null;
  reason: string;
  risk_signals: RiskSignal[];
};

export type TimeHorizon = {
  overdue?: TimeHorizonItem[];
  next_7_days?: TimeHorizonItem[];
  next_30_days?: TimeHorizonItem[];
  next_90_days?: TimeHorizonItem[];
  beyond?: TimeHorizonItem[];
};

export type FocusBlockObligation = {
  obligation_id: string;
  status: string;
  hard_due_at: string | null;
  soft_due_at: string | null;
  reason: string;
};

export type FocusBlock = {
  node_id: string;
  node_type: string;
  canonical_text: string;
  time_horizon_bucket: string;
  obligations: FocusBlockObligation[];
};

export type GraphNode = {
  id: string;
  node_type: string;
  canonical_text: string;
  attributes: Record<string, unknown>;
  lifecycle_state: string;
};

// ADR-0051: present only when the list was fetched with ?node_type=person.
export type PersonListNode = GraphNode & {
  open_count: number;
  at_risk_count: number;
  last_interaction_at: string | null;
};

export type NodeNeighbor = {
  edge_id: string;
  from_id: string;
  to_id: string;
  edge_type: string;
  confidence: number | null;
  valid_from: string | null;
  valid_to: string | null;
  neighbor:
    | { id: string; node_type: string; canonical_text: string }
    | { id: string; type: "obligation"; status: string; hard_due_at: string | null; soft_due_at: string | null; reason: string }
    | null;
};

export type RelationshipObligation = {
  obligation_id: string;
  status: string;
  hard_due_at: string | null;
  soft_due_at: string | null;
  reason: string;
  risk_signals: RiskSignal[];
};

export type NodeDetail = GraphNode & {
  neighbors: NodeNeighbor[];
  relationship: { at_risk: RelationshipObligation[]; open: RelationshipObligation[] } | null;
  last_interaction_at: string | null;
};

export type Edge = {
  id: string;
  from_id: string;
  to_id: string;
  edge_type: string;
  confidence: number | null;
  valid_from: string | null;
  valid_to: string | null;
};

export type AuditEvent = {
  id: string;
  actor: string;
  action: string;
  previous_state: unknown;
  new_state: unknown;
  source: string;
  policy_outcome: string;
  recorded_at: string;
};

async function getJson<T>(path: string): Promise<T> {
  const response = await fetch(path);
  if (!response.ok) {
    const body = await response.text().catch(() => "");
    throw new Error(body || `${path} responded ${response.status}`);
  }
  return (await response.json()) as T;
}

async function postJson<T>(path: string, body?: unknown): Promise<T> {
  const response = await fetch(path, {
    method: "POST",
    ...(body !== undefined && { headers: { "content-type": "application/json" }, body: JSON.stringify(body) }),
  });
  if (!response.ok) {
    const text = await response.text().catch(() => "");
    throw new Error(text || `${path} responded ${response.status}`);
  }
  return (await response.json()) as T;
}

async function patchJson<T>(path: string, body: unknown): Promise<T> {
  const response = await fetch(path, {
    method: "PATCH",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(body),
  });
  if (!response.ok) {
    const text = await response.text().catch(() => "");
    throw new Error(text || `${path} responded ${response.status}`);
  }
  return (await response.json()) as T;
}

// ADR-0059: limit/offset are optional and additive -- omitting both fetches
// every row, unchanged from before this ADR.
export function fetchObligations(limit?: number, offset?: number): Promise<Obligation[]> {
  const params = new URLSearchParams();
  if (limit !== undefined) params.set("limit", String(limit));
  if (offset !== undefined) params.set("offset", String(offset));
  const query = params.toString();
  return getJson<Obligation[]>(query ? `/api/obligations?${query}` : "/api/obligations");
}

export function fetchCandidates(limit?: number, offset?: number): Promise<Candidate[]> {
  const params = new URLSearchParams();
  if (limit !== undefined) params.set("limit", String(limit));
  if (offset !== undefined) params.set("offset", String(offset));
  const query = params.toString();
  return getJson<Candidate[]>(query ? `/api/candidates?${query}` : "/api/candidates");
}

export function searchSourceFragments(query: string): Promise<SearchResult[]> {
  return getJson<SearchResult[]>(`/api/search?q=${encodeURIComponent(query)}`);
}

export function fetchDailyBrief(): Promise<DailyBriefItem[]> {
  return getJson<DailyBriefItem[]>("/api/daily-brief");
}

export function fetchTimeHorizon(): Promise<TimeHorizon> {
  return getJson<TimeHorizon>("/api/time-horizon");
}

export function fetchFocusBlocks(): Promise<FocusBlock[]> {
  return getJson<FocusBlock[]>("/api/focus-blocks");
}

export function fetchAuditEvents(): Promise<AuditEvent[]> {
  return getJson<AuditEvent[]>("/api/audit-events");
}

export function acceptCandidate(candidateId: string): Promise<Candidate> {
  return postJson<Candidate>(`/api/candidates/${encodeURIComponent(candidateId)}/accept`);
}

export function rejectCandidate(candidateId: string): Promise<Candidate> {
  return postJson<Candidate>(`/api/candidates/${encodeURIComponent(candidateId)}/reject`);
}

// ADR-0045: the six candidate_type values the backend's correct/extract
// routes accept -- kept here so the correction form can't drift from them.
export const CANDIDATE_TYPES = ["commitment", "request", "risk", "follow_up", "decision", "expectation"] as const;

export function correctCandidate(candidateId: string, correction: { candidate_type?: string; statement?: string }): Promise<Candidate> {
  return postJson<Candidate>(`/api/candidates/${encodeURIComponent(candidateId)}/correct`, correction);
}

export function promoteCandidate(candidateId: string): Promise<Obligation> {
  return postJson<Obligation>(`/api/candidates/${encodeURIComponent(candidateId)}/promote`);
}

export function fetchNodes(nodeType?: string, needsAttention?: boolean, limit?: number, offset?: number): Promise<GraphNode[]> {
  const params = new URLSearchParams();
  if (nodeType) params.set("node_type", nodeType);
  if (needsAttention) params.set("needs_attention", "true");
  if (limit !== undefined) params.set("limit", String(limit));
  if (offset !== undefined) params.set("offset", String(offset));
  const query = params.toString();
  return getJson<GraphNode[]>(query ? `/api/nodes?${query}` : "/api/nodes");
}

export function fetchNodeDetail(id: string): Promise<NodeDetail> {
  return getJson<NodeDetail>(`/api/nodes/${encodeURIComponent(id)}`);
}

export type MeetingFragment = {
  id: string;
  text: string;
  speaker: string | null;
  sequence: number | null;
  created_at: string;
};

export type MeetingDetail = {
  id: string;
  canonical_text: string;
  attributes: Record<string, unknown>;
  fragments: MeetingFragment[];
};

export type MeetingCandidate = {
  candidate_id: string;
  candidate_type: string;
  statement: string;
  validation_state: string;
  confidence: number;
};

export type MeetingCandidateFragment = {
  fragment_id: string;
  sequence: number | null;
  speaker: string | null;
  text: string;
  candidates: MeetingCandidate[];
};

export type MeetingCandidates = {
  meeting_id: string;
  fragments: MeetingCandidateFragment[];
  progress: {
    fragment_count: number;
    extracted_fragment_count: number;
    pending_fragment_count: number;
    by_validation_state: Record<string, number>;
  };
};

export function fetchMeetingDetail(id: string): Promise<MeetingDetail> {
  return getJson<MeetingDetail>(`/api/meetings/${encodeURIComponent(id)}`);
}

export function fetchMeetingCandidates(id: string): Promise<MeetingCandidates> {
  return getJson<MeetingCandidates>(`/api/meetings/${encodeURIComponent(id)}/candidates`);
}

export type LinkedNode = {
  edge_id: string;
  edge_type: string;
  node_id: string | null;
  node_type: string | null;
  canonical_text: string | null;
};

export type ObligationDetail = {
  obligation_id: string;
  status: string;
  updated_at: string;
  hard_due_at: string | null;
  soft_due_at: string | null;
  source_fragment_id: string | null;
  source_text: string | null;
  risk_signals: RiskSignal[];
  linked_nodes: LinkedNode[];
};

export function fetchObligationDetail(id: string): Promise<ObligationDetail> {
  return getJson<ObligationDetail>(`/api/obligations/${encodeURIComponent(id)}`);
}

export type ExtractResult = { status: "created" } | { status: "empty" } | { status: "unavailable"; message: string };

// ADR-0013's trigger route distinguishes created/empty/unavailable by status
// code alone (201/204/503), so this reads the status directly rather than
// reusing postJson, which assumes every ok response has a JSON body.
export async function extractSourceFragment(fragmentId: string): Promise<ExtractResult> {
  const response = await fetch(`/api/source-fragments/${encodeURIComponent(fragmentId)}/extract`, { method: "POST" });
  if (response.status === 201) {
    return { status: "created" };
  }
  if (response.status === 204) {
    return { status: "empty" };
  }
  const text = await response.text().catch(() => "");
  if (response.status === 503) {
    return { status: "unavailable", message: text || "No model configured" };
  }
  throw new Error(text || `extract responded ${response.status}`);
}

export function createNode(nodeType: string, canonicalText: string, attributes?: Record<string, unknown>): Promise<GraphNode> {
  return postJson<GraphNode>("/api/nodes", { node_type: nodeType, canonical_text: canonicalText, attributes });
}

export function updateNode(
  id: string,
  patch: { canonical_text?: string; lifecycle_state?: string; attributes?: Record<string, unknown> },
): Promise<GraphNode> {
  return patchJson<GraphNode>(`/api/nodes/${encodeURIComponent(id)}`, patch);
}

export function createEdge(fromId: string, toId: string, edgeType: string, confidence?: number, supersede?: boolean): Promise<Edge> {
  return postJson<Edge>("/api/edges", { from_id: fromId, to_id: toId, edge_type: edgeType, confidence, supersede });
}
