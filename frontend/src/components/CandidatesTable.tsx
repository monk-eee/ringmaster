import { useState } from "react";
import { acceptCandidate, CANDIDATE_TYPES, correctCandidate, promoteCandidate, rejectCandidate, type Candidate } from "../api";
import { typeIcon } from "../icons";

type Props = { candidates: Candidate[]; onChanged: () => void };

export default function CandidatesTable({ candidates, onChanged }: Props) {
  const [pendingId, setPendingId] = useState<string | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [draftType, setDraftType] = useState("");
  const [draftStatement, setDraftStatement] = useState("");

  async function handle(action: (id: string) => Promise<unknown>, candidateId: string) {
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

  function startEditing(candidate: Candidate) {
    setEditingId(candidate.candidate_id);
    setDraftType(candidate.candidate_type);
    setDraftStatement(candidate.statement);
    setActionError(null);
  }

  async function saveCorrection(candidate: Candidate) {
    const correction: { candidate_type?: string; statement?: string } = {};
    if (draftType !== candidate.candidate_type) correction.candidate_type = draftType;
    if (draftStatement !== candidate.statement) correction.statement = draftStatement;
    setPendingId(candidate.candidate_id);
    setActionError(null);
    try {
      await correctCandidate(candidate.candidate_id, correction);
      setEditingId(null);
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
        <p className="empty-state">Nothing is waiting for review.</p>
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
              <td>
                <span className="type-icon" aria-hidden="true">
                  {typeIcon(c.candidate_type)}
                </span>
                {c.candidate_type}
              </td>
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
                {editingId === c.candidate_id ? (
                  <div className="correction-form">
                    <select value={draftType} onChange={(event) => setDraftType(event.target.value)}>
                      {CANDIDATE_TYPES.map((candidateType) => (
                        <option key={candidateType} value={candidateType}>
                          {candidateType}
                        </option>
                      ))}
                    </select>
                    <textarea value={draftStatement} onChange={(event) => setDraftStatement(event.target.value)} rows={2} />
                    <div className="correction-form-actions">
                      <button
                        className="save-correction-button"
                        disabled={pendingId === c.candidate_id}
                        onClick={() => saveCorrection(c)}
                      >
                        {pendingId === c.candidate_id ? "…" : "Save Correction"}
                      </button>
                      <button
                        className="cancel-correction-button"
                        disabled={pendingId === c.candidate_id}
                        onClick={() => setEditingId(null)}
                      >
                        Cancel
                      </button>
                    </div>
                  </div>
                ) : c.validation_state === "candidate" ? (
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
                    <button
                      className="correct-button"
                      disabled={pendingId === c.candidate_id}
                      onClick={() => startEditing(c)}
                    >
                      Correct
                    </button>
                  </>
                ) : c.validation_state === "accepted" || c.validation_state === "corrected" ? (
                  <button
                    className="promote-button"
                    disabled={pendingId === c.candidate_id}
                    onClick={() => handle(promoteCandidate, c.candidate_id)}
                  >
                    {pendingId === c.candidate_id ? "…" : "Promote to Obligation"}
                  </button>
                ) : c.validation_state === "promoted" && c.promoted_obligation_id ? (
                  <code title={c.promoted_obligation_id}>{c.promoted_obligation_id.slice(0, 8)}…</code>
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
