import { useState } from "react";
import type { FocusBlock, FocusBlockObligation } from "../api";
import StatusBadge from "./StatusBadge";
import { typeIcon } from "../icons";
import { BUCKETS } from "./TimeHorizon";

type Props = { blocks: FocusBlock[] };

// ADR-0052: names the bucket a block's Obligations actually share, reusing
// Time Horizon's own labels -- "these belong together" states why, not
// just that they do.
function bucketLabel(bucket: string): string {
  return BUCKETS.find((entry) => entry.key === bucket)?.label ?? bucket;
}

// ADR-0050: Today shows a few focus groups by default, ordered by urgency,
// with an honest "show all" -- not a 110-item dump that destroys the
// attention budget.
const FOCUS_BLOCK_CAP = 3;

// Lower sorts first (more urgent): at_risk before others, then soonest
// effective due date. Reuses the ranked list's own signal, no new scoring.
function obligationRank(obligation: FocusBlockObligation): [number, number] {
  const effective = obligation.hard_due_at ?? obligation.soft_due_at;
  const due = effective ? new Date(effective).getTime() : Number.MAX_SAFE_INTEGER;
  return [obligation.status === "at_risk" ? 0 : 1, due];
}

// A block is as urgent as its most urgent obligation.
function blockRank(block: FocusBlock): [number, number] {
  return block.obligations
    .map(obligationRank)
    .reduce((best, rank) => (rank[0] < best[0] || (rank[0] === best[0] && rank[1] < best[1]) ? rank : best), [1, Number.MAX_SAFE_INTEGER]);
}

export default function FocusBlocks({ blocks }: Props) {
  const [showAll, setShowAll] = useState(false);
  if (blocks.length === 0) return null;

  const ordered = [...blocks].sort((a, b) => {
    const rankA = blockRank(a);
    const rankB = blockRank(b);
    return rankA[0] - rankB[0] || rankA[1] - rankB[1];
  });
  const shown = showAll ? ordered : ordered.slice(0, FOCUS_BLOCK_CAP);

  return (
    <div className="focus-blocks">
      {shown.map((block) => (
        <div className="card focus-block" key={`${block.node_id}-${block.time_horizon_bucket}`}>
          <p className="daily-brief-summary">
            <span className="type-icon" aria-hidden="true">
              {typeIcon(block.node_type)}
            </span>{" "}
            {block.canonical_text} — {bucketLabel(block.time_horizon_bucket)} ({block.obligations.length} things belong together)
          </p>
          <ol className="daily-brief-list">
            {block.obligations.map((item) => (
              <li key={item.obligation_id}>
                <div className="daily-brief-row">
                  <span className="type-icon" aria-hidden="true">
                    {typeIcon("obligation")}
                  </span>
                  <StatusBadge value={item.status} />
                </div>
                <span className="daily-brief-reason">{item.reason}</span>
              </li>
            ))}
          </ol>
        </div>
      ))}
      {blocks.length > FOCUS_BLOCK_CAP && (
        <button type="button" className="daily-brief-view-more" onClick={() => setShowAll((value) => !value)}>
          {showAll ? "Show fewer" : `Show all ${blocks.length}`}
        </button>
      )}
    </div>
  );
}
