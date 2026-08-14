export type Obligation = {
  obligation_id: string;
  status: string;
  updated_at: string;
};

export type Candidate = {
  candidate_id: string;
  candidate_type: string;
  statement: string;
  validation_state: string;
  confidence: number;
  source_fragment_id: string | null;
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
};

export type GraphNode = {
  id: string;
  node_type: string;
  canonical_text: string;
  attributes: Record<string, unknown>;
  lifecycle_state: string;
};

export type NodeNeighbor = {
  edge_id: string;
  from_id: string;
  to_id: string;
  edge_type: string;
  confidence: number | null;
  neighbor: { id: string; node_type: string; canonical_text: string } | null;
};

export type NodeDetail = GraphNode & { neighbors: NodeNeighbor[] };

export type Edge = {
  id: string;
  from_id: string;
  to_id: string;
  edge_type: string;
  confidence: number | null;
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

export function fetchObligations(): Promise<Obligation[]> {
  return getJson<Obligation[]>("/api/obligations");
}

export function fetchCandidates(): Promise<Candidate[]> {
  return getJson<Candidate[]>("/api/candidates");
}

export function searchSourceFragments(query: string): Promise<SearchResult[]> {
  return getJson<SearchResult[]>(`/api/search?q=${encodeURIComponent(query)}`);
}

export function fetchDailyBrief(): Promise<DailyBriefItem[]> {
  return getJson<DailyBriefItem[]>("/api/daily-brief");
}

export function acceptCandidate(candidateId: string): Promise<Candidate> {
  return postJson<Candidate>(`/api/candidates/${encodeURIComponent(candidateId)}/accept`);
}

export function rejectCandidate(candidateId: string): Promise<Candidate> {
  return postJson<Candidate>(`/api/candidates/${encodeURIComponent(candidateId)}/reject`);
}

export function fetchNodes(nodeType?: string): Promise<GraphNode[]> {
  return getJson<GraphNode[]>(nodeType ? `/api/nodes?node_type=${encodeURIComponent(nodeType)}` : "/api/nodes");
}

export function fetchNodeDetail(id: string): Promise<NodeDetail> {
  return getJson<NodeDetail>(`/api/nodes/${encodeURIComponent(id)}`);
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

export function createEdge(fromId: string, toId: string, edgeType: string, confidence?: number): Promise<Edge> {
  return postJson<Edge>("/api/edges", { from_id: fromId, to_id: toId, edge_type: edgeType, confidence });
}
