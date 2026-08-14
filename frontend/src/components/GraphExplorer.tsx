import { useEffect, useState } from "react";
import { createEdge, createNode, fetchNodeDetail, fetchNodes, updateNode, type GraphNode, type NodeDetail, type NodeNeighbor, type RelationshipObligation } from "../api";
import StatusBadge from "./StatusBadge";
import { typeIcon } from "../icons";

const RADIUS = 120;
const CENTER = 160;

const NODE_TYPE_COLORS: Record<string, { bg: string; fg: string }> = {
  person: { bg: "#e0e7ff", fg: "#2547d0" },
  meeting: { bg: "#fef3c7", fg: "#92400e" },
  risk: { bg: "#fde3d8", fg: "#c2410c" },
  decision: { bg: "#dcfce7", fg: "#15803d" },
  expectation: { bg: "#f3e8ff", fg: "#7e22ce" },
  customer_problem: { bg: "#fee2e2", fg: "#b91c1c" },
  outcome: { bg: "#cffafe", fg: "#0e7490" },
  service: { bg: "#e9eaef", fg: "#454b58" },
};

const FALLBACK_NODE_TYPE_PALETTE: { bg: string; fg: string }[] = [
  { bg: "#ccfbf1", fg: "#0f766e" },
  { bg: "#fce7f3", fg: "#be185d" },
  { bg: "#ffedd5", fg: "#9a3412" },
  { bg: "#ecfccb", fg: "#4d7c0f" },
  { bg: "#e0f2fe", fg: "#0369a1" },
  { bg: "#ede9fe", fg: "#6d28d9" },
];

const NEUTRAL_NODE_COLOR = { bg: "#e9eaef", fg: "#9aa1ae" };

// Deterministic per-type color so the same node type always renders the same way, even for freeform types.
function nodeTypeColors(nodeType: string): { bg: string; fg: string } {
  const known = NODE_TYPE_COLORS[nodeType];
  if (known) return known;
  let hash = 0;
  for (let index = 0; index < nodeType.length; index += 1) {
    hash = (hash * 31 + nodeType.charCodeAt(index)) >>> 0;
  }
  return FALLBACK_NODE_TYPE_PALETTE[hash % FALLBACK_NODE_TYPE_PALETTE.length];
}

function estimateTextWidth(text: string, fontSize: number): number {
  return text.length * fontSize * 0.6;
}

function parseAttributes(text: string): Record<string, unknown> | undefined {
  const trimmed = text.trim();
  if (!trimmed) return undefined;
  return JSON.parse(trimmed) as Record<string, unknown>;
}

/// Reuses the Daily Brief's own list/row/reason presentation (ADR-0028) so a
/// person's linked Obligations look identical to how the Daily Brief shows them.
function renderRelationshipGroup(title: string, entries: RelationshipObligation[]) {
  if (entries.length === 0) return null;
  return (
    <>
      <p className="relationship-group-title">{title}</p>
      <ol className="daily-brief-list">
        {entries.map((entry) => (
          <li key={entry.obligation_id}>
            <div className="daily-brief-row">
              <code title={entry.obligation_id}>{entry.obligation_id.slice(0, 8)}…</code>
              <StatusBadge value={entry.status} />
            </div>
            <span className="daily-brief-reason">{entry.reason}</span>
          </li>
        ))}
      </ol>
    </>
  );
}

function isNodeNeighbor(neighbor: NodeNeighbor["neighbor"]): neighbor is { id: string; node_type: string; canonical_text: string } {
  return neighbor !== null && !("type" in neighbor);
}

// ADR-0033: renders the trust state ADR-0032 already carries per edge --
// historical (superseded) and/or confidence-bearing (suggested) -- as text,
// never by colour alone.
function trustSuffix(validTo: string | null, confidence: number | null): string {
  const parts: string[] = [];
  if (validTo !== null) parts.push(`historical, until ${new Date(validTo).toLocaleDateString()}`);
  if (confidence !== null) parts.push(`suggested, ${Math.round(confidence * 100)}% confidence`);
  return parts.length > 0 ? ` (${parts.join("; ")})` : "";
}

// ADR-0033: one step in the current exploration. viaEdgeType/direction are
// null only for the root (the first node selected from the list).
// viaValidTo/viaConfidence carry the traversed edge's own trust treatment
// (ADR-0032) into the trail, so a historical or suggested link never reads
// as an equally-certain, current fact.
type TrailStep = {
  nodeId: string;
  nodeType: string;
  canonicalText: string;
  viaEdgeType: string | null;
  direction: "outgoing" | "incoming" | null;
  viaValidTo: string | null;
  viaConfidence: number | null;
};

export default function GraphExplorer() {
  const [nodes, setNodes] = useState<GraphNode[]>([]);
  const [nodeTypeFilter, setNodeTypeFilter] = useState("all");
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);
  const [detail, setDetail] = useState<NodeDetail | null>(null);
  const [trail, setTrail] = useState<TrailStep[]>([]);
  const [error, setError] = useState<string | null>(null);

  const [newNodeType, setNewNodeType] = useState("");
  const [newCanonicalText, setNewCanonicalText] = useState("");
  const [newAttributesText, setNewAttributesText] = useState("");
  const [creating, setCreating] = useState(false);

  const [enrichAttributesText, setEnrichAttributesText] = useState("");
  const [enriching, setEnriching] = useState(false);

  const [edgeTargetId, setEdgeTargetId] = useState("");
  const [edgeType, setEdgeType] = useState("");
  const [supersede, setSupersede] = useState(false);
  const [linking, setLinking] = useState(false);

  async function loadNodes() {
    try {
      setNodes(await fetchNodes());
    } catch (cause) {
      setError((cause as Error).message);
    }
  }

  async function fetchDetail(id: string) {
    setSelectedNodeId(id);
    setEnrichAttributesText("");
    try {
      setDetail(await fetchNodeDetail(id));
    } catch (cause) {
      setError((cause as Error).message);
      setDetail(null);
    }
  }

  // Re-fetches the current focus without touching the trail (used after an
  // enrich/link on the already-focused node -- not a traversal step).
  async function refreshDetail() {
    if (!selectedNodeId) return;
    try {
      setDetail(await fetchNodeDetail(selectedNodeId));
    } catch (cause) {
      setError((cause as Error).message);
    }
  }

  // Starting from the node list (or just-created node) begins a new trail.
  async function selectRootNode(node: { id: string; node_type: string; canonical_text: string }) {
    setTrail([{ nodeId: node.id, nodeType: node.node_type, canonicalText: node.canonical_text, viaEdgeType: null, direction: null, viaValidTo: null, viaConfidence: null }]);
    await fetchDetail(node.id);
  }

  // Clicking a neighbor in the radial view appends one increment to the
  // trail (ADR-0033), or jumps back to it if it's already an earlier step,
  // rather than growing an endlessly repeating cycle.
  async function visitNeighbor(neighbor: NodeNeighbor) {
    if (!isNodeNeighbor(neighbor.neighbor)) return;
    const target = neighbor.neighbor;
    let nextFocusId = target.id;
    setTrail((current) => {
      const existingIndex = current.findIndex((step) => step.nodeId === target.id);
      if (existingIndex !== -1) return current.slice(0, existingIndex + 1);
      const direction: "outgoing" | "incoming" = neighbor.from_id === selectedNodeId ? "outgoing" : "incoming";
      return [
        ...current,
        {
          nodeId: target.id,
          nodeType: target.node_type,
          canonicalText: target.canonical_text,
          viaEdgeType: neighbor.edge_type,
          direction,
          viaValidTo: neighbor.valid_to,
          viaConfidence: neighbor.confidence,
        },
      ];
    });
    await fetchDetail(nextFocusId);
  }

  // Selecting an earlier trail step truncates later steps and restores it as
  // the current focus (also backs the dedicated Back control by one step).
  async function jumpToTrailStep(index: number) {
    const step = trail[index];
    if (!step) return;
    setTrail((current) => current.slice(0, index + 1));
    await fetchDetail(step.nodeId);
  }

  function goBack() {
    if (trail.length > 1) jumpToTrailStep(trail.length - 2);
  }

  useEffect(() => {
    loadNodes();
  }, []);

  const nodeTypes = Array.from(new Set(nodes.map((node) => node.node_type))).sort();
  const visibleNodes = nodeTypeFilter === "all" ? nodes : nodes.filter((node) => node.node_type === nodeTypeFilter);

  async function handleCreate(event: React.FormEvent) {
    event.preventDefault();
    setError(null);
    let attributes: Record<string, unknown> | undefined;
    try {
      attributes = parseAttributes(newAttributesText);
    } catch {
      setError("Attributes must be valid JSON (or left blank).");
      return;
    }
    setCreating(true);
    try {
      const node = await createNode(newNodeType, newCanonicalText, attributes);
      setNewNodeType("");
      setNewCanonicalText("");
      setNewAttributesText("");
      await loadNodes();
      await selectRootNode(node);
    } catch (cause) {
      setError((cause as Error).message);
    } finally {
      setCreating(false);
    }
  }

  async function handleEnrich(event: React.FormEvent) {
    event.preventDefault();
    if (!selectedNodeId) return;
    setError(null);
    let attributes: Record<string, unknown> | undefined;
    try {
      attributes = parseAttributes(enrichAttributesText);
    } catch {
      setError("Attributes must be valid JSON (or left blank).");
      return;
    }
    if (!attributes) return;
    setEnriching(true);
    try {
      await updateNode(selectedNodeId, { attributes });
      setEnrichAttributesText("");
      await loadNodes();
      await refreshDetail();
    } catch (cause) {
      setError((cause as Error).message);
    } finally {
      setEnriching(false);
    }
  }

  async function handleLink(event: React.FormEvent) {
    event.preventDefault();
    if (!selectedNodeId || !edgeTargetId || !edgeType.trim()) return;
    setError(null);
    setLinking(true);
    try {
      await createEdge(selectedNodeId, edgeTargetId, edgeType.trim(), undefined, supersede);
      setEdgeTargetId("");
      setEdgeType("");
      setSupersede(false);
      await refreshDetail();
    } catch (cause) {
      setError((cause as Error).message);
    } finally {
      setLinking(false);
    }
  }

  const neighbors = detail?.neighbors ?? [];

  return (
    <div className="graph-explorer">
      {error && <p className="error">{error}</p>}

      <div className="card">
        <div className="card-header">
          <h2>Add a node</h2>
        </div>
        <div className="card-body">
        <form className="toolbar" onSubmit={handleCreate}>
          <input placeholder="Node type (e.g. person, risk)" list="node-type-suggestions" value={newNodeType} onChange={(event) => setNewNodeType(event.target.value)} required />
          <datalist id="node-type-suggestions">
            <option value="person" />
            <option value="meeting" />
            <option value="risk" />
            <option value="decision" />
            <option value="expectation" />
            <option value="customer_problem" />
            <option value="outcome" />
            <option value="service" />
          </datalist>
          <input placeholder="Canonical text (e.g. a name)" value={newCanonicalText} onChange={(event) => setNewCanonicalText(event.target.value)} required />
          <div className="field">
            <span className="field-label">Attributes (JSON, optional)</span>
            <textarea
              className="field-textarea"
              placeholder='{"role": "manager"}'
              value={newAttributesText}
              onChange={(event) => setNewAttributesText(event.target.value)}
              rows={3}
            />
          </div>
          <button type="submit" disabled={creating || !newNodeType.trim() || !newCanonicalText.trim()}>
            {creating ? "Creating…" : "Create node"}
          </button>
        </form>
        </div>
      </div>

      <div className="graph-layout">
        <div className="card node-list">
          <div className="card-header">
            <h2>Nodes</h2>
            <span className="card-header-meta">
              {visibleNodes.length} of {nodes.length}
            </span>
          </div>
          <div className="card-body">
          <label className="filter-label">
            Filter by type
            <select value={nodeTypeFilter} onChange={(event) => setNodeTypeFilter(event.target.value)}>
              <option value="all">All</option>
              {nodeTypes.map((nodeType) => (
                <option key={nodeType} value={nodeType}>
                  {nodeType}
                </option>
              ))}
            </select>
          </label>
          {nodeTypeFilter !== "all" && (
            <button type="button" className="filter-chip" onClick={() => setNodeTypeFilter("all")}>
              {nodeTypeFilter}
              <span className="filter-chip-x" aria-hidden="true">
                ×
              </span>
            </button>
          )}
          {visibleNodes.length === 0 ? (
            <p className="empty-state">No nodes yet.</p>
          ) : (
            <ul className="node-list-items">
              {visibleNodes.map((node) => (
                <li key={node.id}>
                  <button className={node.id === selectedNodeId ? "node-list-button node-list-button-active" : "node-list-button"} onClick={() => selectRootNode(node)}>
                    <span className="node-type-tag" style={{ background: nodeTypeColors(node.node_type).bg, color: nodeTypeColors(node.node_type).fg }}>
                      <span aria-hidden="true">{typeIcon(node.node_type)}</span> {node.node_type}
                    </span>{" "}
                    {node.canonical_text}
                  </button>
                </li>
              ))}
            </ul>
          )}
          </div>
        </div>

        <div className="card node-detail">
          <div className="card-header">
            <h2>Node detail</h2>
          </div>
          <div className="card-body">
          {!detail ? (
            <p className="empty-state">Select a node to drill in.</p>
          ) : (
            <>
              <nav className="graph-trail" aria-label="Traversal trail">
                <button type="button" className="graph-trail-back" onClick={goBack} disabled={trail.length <= 1}>
                  ← Back
                </button>
                <ol className="graph-trail-path">
                  {trail.map((step, index) => (
                    <li key={`${step.nodeId}-${index}`} className="graph-trail-item">
                      {index > 0 && (
                        <>
                          <span className="graph-trail-chevron" aria-hidden="true">
                            ›
                          </span>
                          <span className={step.viaValidTo !== null || step.viaConfidence !== null ? "graph-trail-verb graph-trail-verb-untrusted" : "graph-trail-verb"}>
                            {step.viaEdgeType}
                            {trustSuffix(step.viaValidTo, step.viaConfidence)}
                          </span>
                          <span className="graph-trail-chevron" aria-hidden="true">
                            ›
                          </span>
                        </>
                      )}
                      <button
                        type="button"
                        className={index === trail.length - 1 ? "graph-trail-step graph-trail-step-current" : "graph-trail-step"}
                        onClick={() => jumpToTrailStep(index)}
                        disabled={index === trail.length - 1}
                      >
                        {typeIcon(step.nodeType)} {step.canonicalText}
                      </button>
                    </li>
                  ))}
                </ol>
              </nav>
              {trail.length > 1 && (
                <p className="why-here">
                  Why here: connected to {trail[trail.length - 2].canonicalText} via "{trail[trail.length - 1].viaEdgeType}"
                  {trustSuffix(trail[trail.length - 1].viaValidTo, trail[trail.length - 1].viaConfidence)}.
                </p>
              )}
              <h3>
                <span className="node-type-tag" style={{ background: nodeTypeColors(detail.node_type).bg, color: nodeTypeColors(detail.node_type).fg }}>
                  <span aria-hidden="true">{typeIcon(detail.node_type)}</span> {detail.node_type}
                </span>{" "}
                {detail.canonical_text}
              </h3>
              <p className="lifecycle-state">Lifecycle: {detail.lifecycle_state}</p>
              <dl className="attributes-list">
                {Object.entries(detail.attributes).map(([key, value]) => (
                  <div key={key}>
                    <dt>{key}</dt>
                    <dd>{JSON.stringify(value)}</dd>
                  </div>
                ))}
              </dl>

              {detail.node_type === "person" && detail.relationship && (
                <div className="relationship-obligations">
                  <h4>Relationship</h4>
                  {detail.relationship.at_risk.length === 0 && detail.relationship.open.length === 0 ? (
                    <p className="empty-state">No linked Obligations yet.</p>
                  ) : (
                    <>
                      {renderRelationshipGroup("At Risk", detail.relationship.at_risk)}
                      {renderRelationshipGroup("Open Commitments", detail.relationship.open)}
                    </>
                  )}
                </div>
              )}

              <svg width={CENTER * 2} height={CENTER * 2} className="relationship-view" role="img" aria-label={`Relationships for ${detail.canonical_text}`}>
                {neighbors.map((neighbor, index) => {
                  const angle = (2 * Math.PI * index) / Math.max(neighbors.length, 1) - Math.PI / 2;
                  const x = CENTER + RADIUS * Math.cos(angle);
                  const y = CENTER + RADIUS * Math.sin(angle);
                  const label = isNodeNeighbor(neighbor.neighbor) ? neighbor.neighbor.canonical_text : neighbor.neighbor ? "Obligation" : "Unknown";
                  const labelIcon = isNodeNeighbor(neighbor.neighbor) ? typeIcon(neighbor.neighbor.node_type) : neighbor.neighbor ? typeIcon("obligation") : "";
                  const midX = (CENTER + x) / 2;
                  const midY = (CENTER + y) / 2 - 4;
                  // ADR-0032: a closed validity window is superseded (dashed/muted); an open one with a known start is current (solid, dated).
                  const superseded = neighbor.valid_to !== null;
                  const pillLabel = superseded
                    ? `${neighbor.edge_type} · until ${new Date(neighbor.valid_to!).toLocaleDateString()}`
                    : neighbor.valid_from !== null
                    ? `${neighbor.edge_type} · since ${new Date(neighbor.valid_from).toLocaleDateString()}`
                    : neighbor.edge_type;
                  const pillWidth = estimateTextWidth(pillLabel, 10) + 12;
                  const neighborColor = isNodeNeighbor(neighbor.neighbor)
                    ? nodeTypeColors(neighbor.neighbor.node_type)
                    : neighbor.neighbor
                    ? { bg: "#fef9c3", fg: "#854d0e" }
                    : { bg: "#e9eaef", fg: "#9aa1ae" };
                  return (
                    <g key={neighbor.edge_id}>
                      <line
                        x1={CENTER}
                        y1={CENTER}
                        x2={x}
                        y2={y}
                        className={superseded ? "relationship-edge relationship-edge-superseded" : "relationship-edge"}
                      />
                      <rect x={midX - pillWidth / 2} y={midY - 9} width={pillWidth} height={14} rx={7} className="relationship-edge-pill" />
                      <text x={midX} y={midY + 1} className="relationship-edge-label" textAnchor="middle">
                        {pillLabel}
                      </text>
                      <circle
                        cx={x}
                        cy={y}
                        r={18}
                        className={isNodeNeighbor(neighbor.neighbor) ? "relationship-node relationship-node-clickable" : "relationship-node"}
                        style={{ fill: neighborColor.bg, stroke: neighborColor.fg }}
                        onClick={isNodeNeighbor(neighbor.neighbor) ? () => visitNeighbor(neighbor) : undefined}
                        role={isNodeNeighbor(neighbor.neighbor) ? "button" : undefined}
                        aria-label={isNodeNeighbor(neighbor.neighbor) ? `Visit ${neighbor.neighbor.canonical_text}` : undefined}
                      />
                      <text x={x} y={y + 32} textAnchor="middle" className="relationship-node-label">
                        {labelIcon} {label.length > 14 ? `${label.slice(0, 14)}…` : label}
                      </text>
                    </g>
                  );
                })}
                <circle cx={CENTER} cy={CENTER} r={30} className="relationship-node-halo" />
                <circle
                  cx={CENTER}
                  cy={CENTER}
                  r={22}
                  className="relationship-node relationship-node-center"
                  style={{ fill: nodeTypeColors(detail.node_type).bg, stroke: nodeTypeColors(detail.node_type).fg }}
                />
                <text x={CENTER} y={CENTER + 40} textAnchor="middle" className="relationship-node-label">
                  {typeIcon(detail.node_type)} {detail.canonical_text.length > 14 ? `${detail.canonical_text.slice(0, 14)}…` : detail.canonical_text}
                </text>
              </svg>
              <p className="relationship-count">
                {neighbors.length === 0 ? "No relationships yet." : `${neighbors.length} relationship${neighbors.length === 1 ? "" : "s"}`}
              </p>

              <form className="toolbar" onSubmit={handleEnrich}>
                <div className="field">
                  <span className="field-label">Enrich attributes (JSON)</span>
                  <textarea
                    className="field-textarea"
                    placeholder='{"team": "platform"}'
                    value={enrichAttributesText}
                    onChange={(event) => setEnrichAttributesText(event.target.value)}
                    rows={3}
                  />
                </div>
                <button type="submit" disabled={enriching || !enrichAttributesText.trim()}>
                  {enriching ? "Enriching…" : "Enrich"}
                </button>
              </form>

              <form className="toolbar" onSubmit={handleLink}>
                <select value={edgeTargetId} onChange={(event) => setEdgeTargetId(event.target.value)}>
                  <option value="">Link to…</option>
                  {nodes.filter((node) => node.id !== selectedNodeId).map((node) => (
                    <option key={node.id} value={node.id}>
                      {node.node_type}: {node.canonical_text}
                    </option>
                  ))}
                </select>
                <input placeholder="Relationship (e.g. made, owns)" value={edgeType} onChange={(event) => setEdgeType(event.target.value)} />
                <label className="supersede-checkbox">
                  <input type="checkbox" checked={supersede} onChange={(event) => setSupersede(event.target.checked)} />
                  Replace any current relationship of this type
                </label>
                <button type="submit" disabled={linking || !edgeTargetId || !edgeType.trim()}>
                  {linking ? "Linking…" : "Add relationship"}
                </button>
              </form>
            </>
          )}
          </div>
        </div>
      </div>
    </div>
  );
}
