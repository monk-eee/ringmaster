import { useEffect, useState } from "react";
import { fetchPersonBrief, type PersonBrief } from "../api";
import { duePhrase } from "./DailyBrief";
import StatusBadge from "./StatusBadge";
import { renderBoldSegments } from "../markdown";

type Props = { personId: string | null };

// ADR-0086: the Workbench's right pane -- composes ADR-0083's person-brief
// read (open commitments, recent asks) into a compact panel. No linked
// person is an honest empty state, never a fabricated relationship.
export default function PersonBriefPanel({ personId }: Props) {
  const [brief, setBrief] = useState<PersonBrief | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setBrief(null);
    setError(null);
    if (!personId) return;
    fetchPersonBrief(personId)
      .then(setBrief)
      .catch((cause) => setError((cause as Error).message));
  }, [personId]);

  if (!personId) {
    return <p className="empty-state">No person linked to this item.</p>;
  }
  if (error) {
    return <p className="error">{error}</p>;
  }
  if (!brief) {
    return <p className="empty-state">Loading…</p>;
  }

  return (
    <div className="person-brief-panel">
      <h3 className="person-brief-name">{brief.person.canonical_text}</h3>

      <h4 className="obligation-detail-section-heading">Open commitments</h4>
      {brief.open_commitments.length === 0 ? (
        <p className="empty-state">Nothing owed right now.</p>
      ) : (
        <ul className="person-brief-list">
          {brief.open_commitments.map((commitment) => (
            <li key={commitment.obligation_id}>
              <div className="daily-brief-row">
                <StatusBadge value={commitment.status} />
                <span className="daily-brief-reason">{duePhrase(commitment.hard_due_at, commitment.soft_due_at)}</span>
              </div>
              <span className="daily-brief-reason">{renderBoldSegments(commitment.reason)}</span>
            </li>
          ))}
        </ul>
      )}

      <h4 className="obligation-detail-section-heading">Recent asks</h4>
      {brief.recent_asks.length === 0 ? (
        <p className="empty-state">Nothing recent recorded.</p>
      ) : (
        <ul className="person-brief-list">
          {brief.recent_asks.map((ask) => (
            <li key={ask.candidate_id}>
              <span className="daily-brief-reason">{ask.candidate_type}: {renderBoldSegments(ask.statement)}</span>
              {ask.source_text && (
                <span className="person-brief-source">
                  {ask.speaker ?? "unknown"}: "{ask.source_text}"
                </span>
              )}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
