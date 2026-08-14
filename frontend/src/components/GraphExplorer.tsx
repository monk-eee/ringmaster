import { useEffect, useState } from "react";
import { createEdge, createNode, fetchNodeDetail, fetchNodes, updateNode, type GraphNode, type NodeDetail, type NodeNeighbor, type RelationshipObligation } from "../api";
import StatusBadge from "./StatusBadge";

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

export default function GraphExplorer() {
  const [nodes, setNodes] = useState<GraphNode[]>([]);
  const [nodeTypeFilter, setNodeTypeFilter] = useState("all");
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);
  const [detail, setDetail] = useState<NodeDetail | null>(null);
  const [error, setError] = useState<string | null>(null);

  const [newNodeType, setNewNodeType] = useState("");
  const [newCanonicalText, setNewCanonicalText] = useState("");
  const [newAttributesText, setNewAttributesText] = useState("");
  const [creating, setCreating] = useState(false);

  const [enrichAttributesText, setEnrichAttributesText] = useState("");
  const [enriching, setEnriching] = useState(false);

  const [edgeTargetId, setEdgeTargetId] = useState("");
  const [edgeType, setEdgeType] = useState("");
  const [linking, setLinking] = useState(false);

  async function loadNodes() {
    try {
      setNodes(await fetchNodes());
    } catch (cause) {
      setError((cause as Error).message);
    }
  }

  async function loadDetail(id: string) {
    setSelectedNodeId(id);
    setEnrichAttributesText("");
    try {
      setDetail(await fetchNodeDetail(id));
    } catch (cause) {
      setError((cause as Error).message);
      setDetail(null);
    }
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
      await loadDetail(node.id);
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
      await loadDetail(selectedNodeId);
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
      await createEdge(selectedNodeId, edgeTargetId, edgeType.trim());
      setEdgeTargetId("");
      setEdgeType("");
      await loadDetail(selectedNodeId);
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
                  <button className={node.id === selectedNodeId ? "node-list-button node-list-button-active" : "node-list-button"} onClick={() => loadDetail(node.id)}>
                    <span className="node-type-tag" style={{ background: nodeTypeColors(node.node_type).bg, color: nodeTypeColors(node.node_type).fg }}>
                      {node.node_type}
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
              <h3>
                <span className="node-type-tag" style={{ background: nodeTypeColors(detail.node_type).bg, color: nodeTypeColors(detail.node_type).fg }}>
                  {detail.node_type}
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
                  const midX = (CENTER + x) / 2;
                  const midY = (CENTER + y) / 2 - 4;
                  const pillWidth = estimateTextWidth(neighbor.edge_type, 10) + 12;
                  const neighborColor = isNodeNeighbor(neighbor.neighbor)
                    ? nodeTypeColors(neighbor.neighbor.node_type)
                    : neighbor.neighbor
                    ? { bg: "#fef9c3", fg: "#854d0e" }
                    : { bg: "#e9eaef", fg: "#9aa1ae" };
                  return (
                    <g key={neighbor.edge_id}>
                      <line x1={CENTER} y1={CENTER} x2={x} y2={y} className="relationship-edge" />
                      <rect x={midX - pillWidth / 2} y={midY - 9} width={pillWidth} height={14} rx={7} className="relationship-edge-pill" />
                      <text x={midX} y={midY + 1} className="relationship-edge-label" textAnchor="middle">
                        {neighbor.edge_type}
                      </text>
                      <circle
                        cx={x}
                        cy={y}
                        r={18}
                        className={isNodeNeighbor(neighbor.neighbor) ? "relationship-node relationship-node-clickable" : "relationship-node"}
                        style={{ fill: neighborColor.bg, stroke: neighborColor.fg }}
                        onClick={isNodeNeighbor(neighbor.neighbor) ? () => loadDetail(neighbor.neighbor!.id) : undefined}
                      />
                      <text x={x} y={y + 32} textAnchor="middle" className="relationship-node-label">
                        {label.length > 14 ? `${label.slice(0, 14)}…` : label}
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
                  {detail.canonical_text.length > 14 ? `${detail.canonical_text.slice(0, 14)}…` : detail.canonical_text}
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
