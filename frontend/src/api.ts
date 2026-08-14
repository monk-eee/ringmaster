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

async function getJson<T>(path: string): Promise<T> {
  const response = await fetch(path);
  if (!response.ok) {
    const body = await response.text().catch(() => "");
    throw new Error(body || `${path} responded ${response.status}`);
  }
  return (await response.json()) as T;
}

async function postJson<T>(path: string): Promise<T> {
  const response = await fetch(path, { method: "POST" });
  if (!response.ok) {
    const body = await response.text().catch(() => "");
    throw new Error(body || `${path} responded ${response.status}`);
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
