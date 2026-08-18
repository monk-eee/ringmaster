import { useEffect, useState } from "react";
import { fetchObligationDetail, type DailyBriefItem } from "../api";
import DailyBrief from "./DailyBrief";
import ObligationDetail from "./ObligationDetail";
import PersonBriefPanel from "./PersonBriefPanel";

type Props = { dailyBrief: DailyBriefItem[] };

// ADR-0086: three panes composing already-proven reads, no page navigation.
// DailyBrief/ObligationDetail are reused verbatim -- neither is modified.
export default function Workbench({ dailyBrief }: Props) {
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [linkedPersonId, setLinkedPersonId] = useState<string | null>(null);

  useEffect(() => {
    setLinkedPersonId(null);
    if (!selectedId) return;
    fetchObligationDetail(selectedId)
      .then((detail) => {
        const person = detail.linked_nodes.find((node) => node.edge_type === "owns" && node.node_type === "person");
        setLinkedPersonId(person?.node_id ?? null);
      })
      .catch(() => setLinkedPersonId(null));
  }, [selectedId]);

  return (
    <div className="workbench">
      <div className="workbench-pane workbench-pane-attention">
        <h2 className="today-section-heading">Attention</h2>
        <DailyBrief items={dailyBrief} onSelect={setSelectedId} />
      </div>
      <div className="workbench-pane workbench-pane-focus">
        <h2 className="today-section-heading">Current focus</h2>
        {selectedId ? (
          <ObligationDetail obligationId={selectedId} onBack={() => setSelectedId(null)} />
        ) : (
          <p className="empty-state">Select an item on the left to see its context.</p>
        )}
      </div>
      <div className="workbench-pane workbench-pane-relationship">
        <h2 className="today-section-heading">Relationship context</h2>
        {selectedId ? (
          <PersonBriefPanel personId={linkedPersonId} />
        ) : (
          <p className="empty-state">Select an item to see who it's connected to.</p>
        )}
      </div>
    </div>
  );
}
