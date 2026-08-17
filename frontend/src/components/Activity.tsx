import { useEffect, useState } from "react";
import { fetchAuditEvents, type AuditEvent } from "../api";

function relativeTime(recordedAt: string): string {
  const seconds = Math.max(0, Math.floor((Date.now() - new Date(recordedAt).getTime()) / 1000));
  if (seconds < 60) return "just now";
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes} minute${minutes === 1 ? "" : "s"} ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours} hour${hours === 1 ? "" : "s"} ago`;
  const days = Math.floor(hours / 24);
  return `${days} day${days === 1 ? "" : "s"} ago`;
}

// ADR-0049: a flat, global audit feed -- not correlated to any specific
// Obligation or candidate, and deliberately not a fabricated narrative
// across action types whose payloads carry genuinely different shapes.
export default function Activity() {
  const [events, setEvents] = useState<AuditEvent[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setLoading(true);
    setError(null);
    fetchAuditEvents()
      .then(setEvents)
      .catch((cause) => setError((cause as Error).message))
      .finally(() => setLoading(false));
  }, []);

  if (loading) {
    return (
      <div className="card">
        <p className="empty-state">Loading…</p>
      </div>
    );
  }

  if (error) {
    return <p className="error">{error}</p>;
  }

  if (events.length === 0) {
    return (
      <div className="card">
        <p className="empty-state">No activity recorded yet.</p>
      </div>
    );
  }

  return (
    <div className="card">
      <ol className="daily-brief-list">
        {events.map((event) => (
          <li key={event.id}>
            <div className="daily-brief-row">
              <span className="activity-action">{event.action}</span>
              <span className="activity-actor">{event.actor}</span>
              <span className="activity-time">{relativeTime(event.recorded_at)}</span>
            </div>
            <span className="daily-brief-reason activity-states">
              {Boolean(event.previous_state) && <code>{JSON.stringify(event.previous_state)}</code>}
              {Boolean(event.previous_state) && Boolean(event.new_state) && " → "}
              {Boolean(event.new_state) && <code>{JSON.stringify(event.new_state)}</code>}
            </span>
          </li>
        ))}
      </ol>
    </div>
  );
}
