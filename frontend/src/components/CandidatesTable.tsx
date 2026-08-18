import { useState } from "react";
import { acceptCandidate, batchPromoteCandidates, batchTransitionCandidates, CANDIDATE_TYPES, correctCandidate, promoteCandidate, rejectCandidate, type Candidate } from "../api";
import { typeIcon } from "../icons";

type Props = { candidates: Candidate[]; onChanged: () => void; hasMore?: boolean; onLoadMore?: () => void };

export default function CandidatesTable({ candidates, onChanged, hasMore, onLoadMore }: Props) {
  const [pendingId, setPendingId] = useState<string | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [draftType, setDraftType] = useState("");
  const [draftStatement, setDraftStatement] = useState("");
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  const [bulkPending, setBulkPending] = useState(false);

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

  // ADR-0076/ADR-0077: a row can be selected for bulk accept/reject while
  // still `candidate`, or for bulk promote once `accepted`/`corrected` --
  // anything else (rejected/promoted) has nothing left to bulk-act on.
  function isSelectable(c: Candidate) {
    return c.validation_state === "candidate" || c.validation_state === "accepted" || c.validation_state === "corrected";
  }
  const selectableIds = candidates.filter(isSelectable).map((c) => c.candidate_id);
  const allSelected = selectableIds.length > 0 && selectableIds.every((id) => selectedIds.has(id));
  const selectedCandidates = candidates.filter((c) => selectedIds.has(c.candidate_id));
  const allSelectedPending = selectedCandidates.length > 0 && selectedCandidates.every((c) => c.validation_state === "candidate");
  const allSelectedPromotable =
    selectedCandidates.length > 0 &&
    selectedCandidates.every((c) => c.validation_state === "accepted" || c.validation_state === "corrected");

  function toggleSelected(candidateId: string) {
    setSelectedIds((current) => {
      const next = new Set(current);
      if (next.has(candidateId)) next.delete(candidateId);
      else next.add(candidateId);
      return next;
    });
  }

  // Selects only the candidates currently loaded on the client -- matching
  // this table's existing Load-more pagination honestly, never reaching
  // into rows that haven't been fetched yet.
  function toggleSelectAll() {
    setSelectedIds(allSelected ? new Set() : new Set(selectableIds));
  }

  async function handleBulk(action: "accept" | "reject") {
    const ids = Array.from(selectedIds);
    if (ids.length === 0) return;
    setBulkPending(true);
    setActionError(null);
    try {
      const result = await batchTransitionCandidates(ids, action);
      if (result.errors.length > 0) {
        setActionError(
          `${result.updated.length} of ${ids.length} updated; ${result.errors.length} could not be ${action === "accept" ? "accepted" : "rejected"}: ${result.errors
            .map((e) => e.error)
            .join("; ")}`,
        );
      }
      setSelectedIds(new Set());
      onChanged();
    } catch (cause) {
      setActionError((cause as Error).message);
    } finally {
      setBulkPending(false);
    }
  }

  async function handleBulkPromote() {
    const ids = Array.from(selectedIds);
    if (ids.length === 0) return;
    setBulkPending(true);
    setActionError(null);
    try {
      const result = await batchPromoteCandidates(ids);
      if (result.errors.length > 0) {
        setActionError(
          `${result.promoted.length} of ${ids.length} promoted; ${result.errors.length} could not be promoted: ${result.errors
            .map((e) => e.error)
            .join("; ")}`,
        );
      }
      setSelectedIds(new Set());
      onChanged();
    } catch (cause) {
      setActionError((cause as Error).message);
    } finally {
      setBulkPending(false);
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
      {selectedIds.size > 0 && (
        <div className="bulk-action-bar">
          <span className="bulk-action-count">{selectedIds.size} selected</span>
          {allSelectedPending && (
            <>
              <button className="bulk-accept-button" disabled={bulkPending} onClick={() => handleBulk("accept")}>
                {bulkPending ? "…" : `Accept ${selectedIds.size} selected`}
              </button>
              <button className="bulk-reject-button" disabled={bulkPending} onClick={() => handleBulk("reject")}>
                {bulkPending ? "…" : `Reject ${selectedIds.size} selected`}
              </button>
            </>
          )}
          {allSelectedPromotable && (
            <button className="bulk-promote-button" disabled={bulkPending} onClick={handleBulkPromote}>
              {bulkPending ? "…" : `Promote ${selectedIds.size} selected`}
            </button>
          )}
          {!allSelectedPending && !allSelectedPromotable && (
            <span className="bulk-action-hint">Select only pending, or only accepted, candidates to act on them together.</span>
          )}
        </div>
      )}
      <table>
        <thead>
          <tr>
            <th>
              <input
                type="checkbox"
                aria-label={`Select all ${selectableIds.length} loaded`}
                checked={allSelected}
                disabled={selectableIds.length === 0}
                onChange={toggleSelectAll}
              />
            </th>
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
                {isSelectable(c) && (
                  <input
                    type="checkbox"
                    aria-label={`Select candidate: ${c.statement}`}
                    checked={selectedIds.has(c.candidate_id)}
                    onChange={() => toggleSelected(c.candidate_id)}
                  />
                )}
              </td>
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
      {hasMore && onLoadMore && (
        <button type="button" className="daily-brief-view-more" onClick={onLoadMore}>
          Load more
        </button>
      )}
    </div>
  );
}
