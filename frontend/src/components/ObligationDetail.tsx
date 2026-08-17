import { useEffect, useState } from "react";
import { fetchObligationDetail, type ObligationDetail as ObligationDetailData } from "../api";
import { itemTitle, duePhrase } from "./DailyBrief";
import StatusBadge from "./StatusBadge";
import { typeIcon } from "../icons";

type Props = { obligationId: string; onBack: () => void };

// ADR-0047: a first-class read view for one Obligation, unblocking the
// "review" action Today/Obligations rows previously had nowhere to go.
// Read-only -- snooze/dismiss/correct-owner are separate, future decisions.
export default function ObligationDetail({ obligationId, onBack }: Props) {
  const [detail, setDetail] = useState<ObligationDetailData | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setDetail(null);
    setError(null);
    fetchObligationDetail(obligationId)
      .then(setDetail)
      .catch((cause) => setError((cause as Error).message));
  }, [obligationId]);

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
          </div>
          <h2 className="today-item-title">{itemTitle(detail)}</h2>
          <p className="daily-brief-reason">{detail.source_fragment_id ? "Evidence recorded" : "No evidence recorded"}</p>

          {detail.risk_signals.length > 0 && (
            <ul className="risk-signals">
              {detail.risk_signals.map((signal) => (
                <li key={signal.signal}>{signal.explanation}</li>
              ))}
            </ul>
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
