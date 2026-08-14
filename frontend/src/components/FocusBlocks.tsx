import type { FocusBlock } from "../api";
import StatusBadge from "./StatusBadge";
import { typeIcon } from "../icons";

type Props = { blocks: FocusBlock[] };

export default function FocusBlocks({ blocks }: Props) {
  if (blocks.length === 0) return null;

  return (
    <div className="focus-blocks">
      {blocks.map((block) => (
        <div className="card focus-block" key={block.node_id}>
          <p className="daily-brief-summary">
            <span className="type-icon" aria-hidden="true">
              {typeIcon(block.node_type)}
            </span>{" "}
            {block.canonical_text} — {block.obligations.length} things belong together
          </p>
          <ol className="daily-brief-list">
            {block.obligations.map((item) => (
              <li key={item.obligation_id}>
                <div className="daily-brief-row">
                  <span className="type-icon" aria-hidden="true">
                    {typeIcon("obligation")}
                  </span>
                  <StatusBadge value={item.status} />
                  <code className="id-marker" title={item.obligation_id}>
                    {item.obligation_id.slice(0, 8)}…
                  </code>
                </div>
                <span className="daily-brief-reason">{item.reason}</span>
              </li>
            ))}
          </ol>
        </div>
      ))}
    </div>
  );
}
