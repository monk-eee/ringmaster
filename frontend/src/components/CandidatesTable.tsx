import { useState } from "react";
import { acceptCandidate, rejectCandidate, type Candidate } from "../api";

type Props = { candidates: Candidate[]; onChanged: () => void };

export default function CandidatesTable({ candidates, onChanged }: Props) {
  const [pendingId, setPendingId] = useState<string | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);

  async function handle(action: (id: string) => Promise<Candidate>, candidateId: string) {
    setPendingId(candidateId);
    setActionError(null);
    try {
      await action(candidateId);
      onChanged();
    } catch (cause) {
      setActionError((cause as Error).message);
    } finally {
      setPendingId(null);
    }
  }

  if (candidates.length === 0) {
    return (
      <div className="card">
        <p className="empty-state">No candidates yet.</p>
      </div>
    );
  }

  return (
    <div className="card">
      {actionError && <p className="error">{actionError}</p>}
      <table>
        <thead>
          <tr>
            <th>Type</th>
            <th>Statement</th>
            <th>Validation state</th>
            <th>Confidence</th>
            <th>Evidence</th>
            <th>Actions</th>
          </tr>
        </thead>
        <tbody>
          {candidates.map((c) => (
            <tr key={c.candidate_id}>
              <td>{c.candidate_type}</td>
              <td>{c.statement}</td>
              <td>{c.validation_state}</td>
              <td>{Math.round(c.confidence * 100)}%</td>
              <td className="evidence-cell">
                {c.source_text ? (
                  <>
                    <span className="speaker">{c.speaker ?? "unknown"}:</span> "{c.source_text}"
                  </>
                ) : (
                  <span className="no-evidence">—</span>
                )}
              </td>
              <td className="actions-cell">
                {c.validation_state === "candidate" ? (
                  <>
                    <button
                      className="accept-button"
                      disabled={pendingId === c.candidate_id}
                      onClick={() => handle(acceptCandidate, c.candidate_id)}
                    >
                      {pendingId === c.candidate_id ? "…" : "Accept"}
                    </button>
                    <button
                      className="reject-button"
                      disabled={pendingId === c.candidate_id}
                      onClick={() => handle(rejectCandidate, c.candidate_id)}
                    >
                      {pendingId === c.candidate_id ? "…" : "Reject"}
                    </button>
                  </>
                ) : (
                  <span className="no-evidence">—</span>
                )}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
