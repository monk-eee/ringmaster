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

async function getJson<T>(path: string): Promise<T> {
  const response = await fetch(path);
  if (!response.ok) throw new Error(`${path} responded ${response.status}`);
  return (await response.json()) as T;
}

export function fetchObligations(): Promise<Obligation[]> {
  return getJson<Obligation[]>("/api/obligations");
}

export function fetchCandidates(): Promise<Candidate[]> {
  return getJson<Candidate[]>("/api/candidates");
}
