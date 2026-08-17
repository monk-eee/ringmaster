import type { Obligation } from "../api";
import StatusBadge from "./StatusBadge";
import { typeIcon } from "../icons";

type Props = {
  obligations: Obligation[];
  onSelect?: (obligationId: string) => void;
  hasMore?: boolean;
  onLoadMore?: () => void;
};

export default function ObligationsTable({ obligations, onSelect, hasMore, onLoadMore }: Props) {
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
            <tr
              key={o.obligation_id}
              className={onSelect ? "obligation-row-selectable" : undefined}
              onClick={onSelect ? () => onSelect(o.obligation_id) : undefined}
            >
              <td className="obligation-title-cell">
                <span className="type-icon" aria-hidden="true">
                  {typeIcon("obligation")}
                </span>
                {o.source_text ? (
                  <span className="obligation-title">"{o.source_text}"</span>
                ) : (
                  <span className="no-evidence">No evidence recorded</span>
                )}
                <code className="id-marker" title={o.obligation_id}>
                  {o.obligation_id.slice(0, 8)}…
                </code>
              </td>
              <td>
                <StatusBadge value={o.status} />
              </td>
              <td className="updated-cell">{new Date(o.updated_at).toLocaleString()}</td>
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
