import { useEffect, useState } from "react";
import { fetchObligationDetail, updateObligation, OBLIGATION_STATUSES, type ObligationDetail as ObligationDetailData } from "../api";
import { itemTitle, duePhrase } from "./DailyBrief";
import StatusBadge from "./StatusBadge";
import { typeIcon } from "../icons";
import { renderBoldSegments } from "../markdown";

type Props = { obligationId: string; onBack: () => void };

// Renders as an empty string ("no date") when absent -- <input type="date">
// only ever accepts a bare YYYY-MM-DD, never a full RFC3339 timestamp.
function toDateInputValue(iso: string | null): string {
  return iso ? iso.slice(0, 10) : "";
}

// ADR-0093: converts a bare YYYY-MM-DD from a date input into the RFC3339
// timestamp the backend's `chrono::DateTime::parse_from_rfc3339` requires.
// An empty string passes through unchanged -- the backend's own contract
// for "explicitly clear this due date."
function toRfc3339(dateInputValue: string): string {
  return dateInputValue ? `${dateInputValue}T00:00:00Z` : "";
}

// ADR-0047 gave this view read-only status; ADR-0093 closes that gap --
// status and due dates are now editable here, over the API, the CLI, and
// the MCP server, all calling the identical `obligation::update_status`.
export default function ObligationDetail({ obligationId, onBack }: Props) {
  const [detail, setDetail] = useState<ObligationDetailData | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [editing, setEditing] = useState(false);
  const [draftStatus, setDraftStatus] = useState("");
  const [draftHardDue, setDraftHardDue] = useState("");
  const [draftSoftDue, setDraftSoftDue] = useState("");
  const [saveError, setSaveError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  function load() {
    setError(null);
    return fetchObligationDetail(obligationId)
      .then(setDetail)
      .catch((cause) => setError((cause as Error).message));
  }

  useEffect(() => {
    setDetail(null);
    load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [obligationId]);

  function startEditing(current: ObligationDetailData) {
    setDraftStatus(current.status);
    setDraftHardDue(toDateInputValue(current.hard_due_at));
    setDraftSoftDue(toDateInputValue(current.soft_due_at));
    setSaveError(null);
    setEditing(true);
  }

  async function saveEdit(current: ObligationDetailData) {
    const patch: { status?: string; hard_due_at?: string; soft_due_at?: string } = {};
    if (draftStatus !== current.status) patch.status = draftStatus;
    const nextHardDue = toRfc3339(draftHardDue);
    if (nextHardDue !== (current.hard_due_at ?? "")) patch.hard_due_at = nextHardDue;
    const nextSoftDue = toRfc3339(draftSoftDue);
    if (nextSoftDue !== (current.soft_due_at ?? "")) patch.soft_due_at = nextSoftDue;
    if (Object.keys(patch).length === 0) {
      setEditing(false);
      return;
    }
    setSaving(true);
    setSaveError(null);
    try {
      await updateObligation(obligationId, patch);
      await load();
      setEditing(false);
    } catch (cause) {
      setSaveError((cause as Error).message);
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="obligation-detail">
      <button type="button" className="obligation-detail-back" onClick={onBack}>
        ← Back
      </button>
      {error && <p className="error">{error}</p>}
      {!detail && !error ? (
        <p className="empty-state">Loading…</p>
      ) : detail ? (
        <div className="card">
          <div className="daily-brief-row">
            <span className="type-icon" aria-hidden="true">
              {typeIcon("obligation")}
            </span>
            <StatusBadge value={detail.status} />
            <span className="daily-brief-reason">{duePhrase(detail.hard_due_at, detail.soft_due_at)}</span>
            {!editing && (
              <button type="button" className="obligation-edit-button" onClick={() => startEditing(detail)}>
                Edit
              </button>
            )}
          </div>
          <h2 className="today-item-title">{renderBoldSegments(itemTitle(detail))}</h2>
          <p className="daily-brief-reason">{detail.source_fragment_id ? "Evidence recorded" : "No evidence recorded"}</p>

          {detail.risk_signals.length > 0 && (
            <ul className="risk-signals">
              {detail.risk_signals.map((signal) => (
                <li key={signal.signal}>{signal.explanation}</li>
              ))}
            </ul>
          )}

          {editing && (
            <div className="obligation-edit-form">
              {saveError && <p className="error">{saveError}</p>}
              <label>
                Status
                <select value={draftStatus} onChange={(event) => setDraftStatus(event.target.value)}>
                  {OBLIGATION_STATUSES.map((value) => (
                    <option key={value} value={value}>
                      {value}
                    </option>
                  ))}
                </select>
              </label>
              <label>
                Hard due date
                <input type="date" value={draftHardDue} onChange={(event) => setDraftHardDue(event.target.value)} />
              </label>
              <label>
                Soft due date
                <input type="date" value={draftSoftDue} onChange={(event) => setDraftSoftDue(event.target.value)} />
              </label>
              <div className="obligation-edit-form-actions">
                <button type="button" className="save-correction-button" disabled={saving} onClick={() => saveEdit(detail)}>
                  {saving ? "…" : "Save"}
                </button>
                <button type="button" className="cancel-correction-button" disabled={saving} onClick={() => setEditing(false)}>
                  Cancel
                </button>
              </div>
            </div>
          )}

          <h3 className="obligation-detail-section-heading">Linked</h3>
          {detail.linked_nodes.length === 0 ? (
            <p className="empty-state">Nothing linked yet.</p>
          ) : (
            <ul className="obligation-detail-linked-list">
              {detail.linked_nodes.map((linked) => (
                <li key={linked.edge_id}>
                  <span aria-hidden="true">{typeIcon(linked.node_type ?? "")}</span>{" "}
                  {linked.canonical_text ?? "Unknown"} <span className="daily-brief-reason">({linked.edge_type})</span>
                </li>
              ))}
            </ul>
          )}
        </div>
      ) : null}
    </div>
  );
}
