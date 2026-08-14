import type { Obligation } from "../api";
import StatusBadge from "./StatusBadge";

type Props = { obligations: Obligation[] };

export default function ObligationsTable({ obligations }: Props) {
  if (obligations.length === 0) {
    return (
      <div className="card">
        <p className="empty-state">No obligations match the current filter.</p>
      </div>
    );
  }

  return (
    <div className="card">
      <table>
        <thead>
          <tr>
            <th>Obligation</th>
            <th>Status</th>
            <th>Updated</th>
          </tr>
        </thead>
        <tbody>
          {obligations.map((o) => (
            <tr key={o.obligation_id}>
              <td className="id-cell">
                <code title={o.obligation_id}>{o.obligation_id.slice(0, 8)}…</code>
              </td>
              <td>
                <StatusBadge value={o.status} />
              </td>
              <td className="updated-cell">{new Date(o.updated_at).toLocaleString()}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
