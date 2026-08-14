import type { TimeHorizon } from "../api";

type Props = { horizon: TimeHorizon; onOpenTimeline: () => void };

// ADR-0039: a compact preview of GET /api/time-horizon's own Next 7/30 Days
// buckets -- not a new engine, not the full Buckets/Timeline page, just a
// small honest strip naming counts and up to three items per window.
export default function ComingSoonStrip({ horizon, onOpenTimeline }: Props) {
  const windows: { label: string; items: TimeHorizon["next_7_days"] }[] = [
    { label: "Next 7 days", items: horizon.next_7_days ?? [] },
    { label: "Next 30 days", items: horizon.next_30_days ?? [] },
  ];

  if (windows.every((window) => window.items!.length === 0)) return null;

  return (
    <div className="card coming-soon-strip">
      <p className="coming-soon-title">Coming soon</p>
      <div className="coming-soon-windows">
        {windows.map((window) => (
          <div key={window.label} className="coming-soon-window">
            <p className="coming-soon-window-header">
              {window.label} <strong>{window.items!.length}</strong>
            </p>
            <ul>
              {window.items!.slice(0, 3).map((item) => (
                <li key={item.obligation_id}>{item.reason}</li>
              ))}
            </ul>
          </div>
        ))}
      </div>
      <button type="button" className="coming-soon-link" onClick={onOpenTimeline}>
        See full Timeline →
      </button>
    </div>
  );
}
