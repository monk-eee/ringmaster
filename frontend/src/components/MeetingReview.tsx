import { useEffect, useState } from "react";
import {
  acceptCandidate,
  extractSourceFragment,
  fetchMeetingCandidates,
  fetchMeetingDetail,
  fetchNodes,
  fetchSourceSynthesis,
  promoteCandidate,
  rejectCandidate,
  synthesizeSource,
  type GraphNode,
  type MeetingCandidates,
  type MeetingDetail,
  type SynthesisGroup,
} from "../api";
import StatusBadge from "./StatusBadge";
import { typeIcon } from "../icons";

export default function MeetingReview() {
  const [meetings, setMeetings] = useState<GraphNode[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [detail, setDetail] = useState<MeetingDetail | null>(null);
  const [candidates, setCandidates] = useState<MeetingCandidates | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [pendingId, setPendingId] = useState<string | null>(null);
  const [extractMessage, setExtractMessage] = useState<string | null>(null);
  const [synthesisGroups, setSynthesisGroups] = useState<SynthesisGroup[]>([]);
  const [synthesizing, setSynthesizing] = useState(false);
  const [synthesisMessage, setSynthesisMessage] = useState<string | null>(null);

  useEffect(() => {
    // ADR-0096: hasSourceFragments=true, not node_type="meeting" -- any
    // ingested source (1on1/note/comms/connect/perspective/...) belongs here.
    fetchNodes(undefined, false, undefined, undefined, true)
      .then(setMeetings)
      .catch((cause) => setError((cause as Error).message));
  }, []);

  async function loadMeeting(id: string) {
    setSelectedId(id);
    setError(null);
    setExtractMessage(null);
    setSynthesisMessage(null);
    try {
      const [nextDetail, nextCandidates, nextGroups] = await Promise.all([
        fetchMeetingDetail(id),
        fetchMeetingCandidates(id),
        fetchSourceSynthesis(id),
      ]);
      setDetail(nextDetail);
      setCandidates(nextCandidates);
      setSynthesisGroups(nextGroups);
    } catch (cause) {
      setError((cause as Error).message);
      setDetail(null);
      setCandidates(null);
      setSynthesisGroups([]);
    }
  }

  // ADR-0094: re-assembles this source's still-accepted candidates into
  // fewer, clearer synthesized statements -- additive, never hides the raw
  // per-fragment list below.
  async function handleSynthesize() {
    if (!selectedId) return;
    setSynthesizing(true);
    setSynthesisMessage(null);
    setError(null);
    try {
      await synthesizeSource(selectedId);
      setSynthesisGroups(await fetchSourceSynthesis(selectedId));
    } catch (cause) {
      setSynthesisMessage((cause as Error).message);
    } finally {
      setSynthesizing(false);
    }
  }

  async function refreshCandidates(id: string) {
    try {
      setCandidates(await fetchMeetingCandidates(id));
    } catch (cause) {
      setError((cause as Error).message);
    }
  }

  async function handleCandidateAction(action: (id: string) => Promise<unknown>, candidateId: string) {
    if (!selectedId) return;
    setPendingId(candidateId);
    setError(null);
    try {
      await action(candidateId);
      await refreshCandidates(selectedId);
    } catch (cause) {
      setError((cause as Error).message);
    } finally {
      setPendingId(null);
    }
  }

  async function handleExtract(fragmentId: string) {
    if (!selectedId) return;
    setPendingId(fragmentId);
    setExtractMessage(null);
    setError(null);
    try {
      const result = await extractSourceFragment(fragmentId);
      if (result.status === "empty") {
        setExtractMessage("Nothing worth extracting was found in this passage.");
      } else if (result.status === "unavailable") {
        setExtractMessage("No model configured — extraction is unavailable right now.");
      }
      await refreshCandidates(selectedId);
    } catch (cause) {
      setError((cause as Error).message);
    } finally {
      setPendingId(null);
    }
  }

  const attributes = detail?.attributes as { occurred_at?: string; date?: string } | undefined;
  const occurredAt = attributes?.occurred_at ?? attributes?.date;

  return (
    <div className="meeting-review">
      {error && <p className="error">{error}</p>}
      <div className="meeting-review-layout">
        <div className="card meeting-review-list">
          <div className="card-header">
            <h2>Sources</h2>
          </div>
          <div className="card-body">
            {meetings.length === 0 ? (
              <p className="empty-state">No sources ingested yet.</p>
            ) : (
              <ul className="node-list-items">
                {meetings.map((meeting) => (
                  <li key={meeting.id}>
                    <button
                      className={meeting.id === selectedId ? "node-list-button node-list-button-active" : "node-list-button"}
                      onClick={() => loadMeeting(meeting.id)}
                    >
                      <span aria-hidden="true">{typeIcon(meeting.node_type)}</span> {meeting.canonical_text}
                    </button>
                  </li>
                ))}
              </ul>
            )}
          </div>
        </div>

        <div className="card meeting-review-detail">
          <div className="card-header">
            <h2>Source review</h2>
          </div>
          <div className="card-body">
            {!detail || !candidates ? (
              <p className="empty-state">Select a source to review its transcript and candidates.</p>
            ) : (
              <>
                <h3>{detail.canonical_text}</h3>
                {occurredAt && <p className="meeting-review-meta">{new Date(occurredAt).toLocaleString()}</p>}
                <p className="meeting-review-progress">
                  {candidates.progress.extracted_fragment_count} of {candidates.progress.fragment_count} fragments extracted
                </p>
                {extractMessage && <p className="meeting-review-extract-message">{extractMessage}</p>}

                <div className="meeting-synthesis">
                  <div className="meeting-synthesis-header">
                    <h4>Synthesis</h4>
                    <button type="button" className="meeting-synthesis-button" disabled={synthesizing} onClick={handleSynthesize}>
                      {synthesizing ? "Synthesizing…" : synthesisGroups.length > 0 ? "Re-synthesize" : "Synthesize"}
                    </button>
                  </div>
                  {synthesisMessage && <p className="meeting-review-extract-message">{synthesisMessage}</p>}
                  {synthesisGroups.length === 0 ? (
                    <p className="empty-state">Not synthesized yet — the raw candidates below are unaffected.</p>
                  ) : (
                    <ul className="meeting-synthesis-groups">
                      {synthesisGroups.map((group) => (
                        <li key={group.id} className="meeting-synthesis-group">
                          <span className="type-icon" aria-hidden="true">
                            {typeIcon(group.candidate_type)}
                          </span>
                          <span className="meeting-synthesis-statement">{group.synthesized_statement}</span>
                          <span className="meeting-synthesis-member-count">
                            from {group.member_candidate_ids.length} candidate{group.member_candidate_ids.length === 1 ? "" : "s"}
                          </span>
                        </li>
                      ))}
                    </ul>
                  )}
                </div>

                <ol className="meeting-fragments">
                  {candidates.fragments.map((fragment) => (
                    <li key={fragment.fragment_id} className="meeting-fragment">
                      <div className="meeting-fragment-text">
                        {fragment.speaker && <strong className="meeting-fragment-speaker">{fragment.speaker}: </strong>}
                        {fragment.text}
                      </div>
                      {fragment.candidates.length === 0 ? (
                        <div className="meeting-fragment-empty">
                          <span className="empty-state">No candidates extracted from this passage yet.</span>
                          <button
                            type="button"
                            className="meeting-fragment-extract"
                            disabled={pendingId === fragment.fragment_id}
                            onClick={() => handleExtract(fragment.fragment_id)}
                          >
                            {pendingId === fragment.fragment_id ? "Extracting…" : "Extract"}
                          </button>
                        </div>
                      ) : (
                        <ul className="meeting-fragment-candidates">
                          {fragment.candidates.map((candidate) => (
                            <li key={candidate.candidate_id} className="meeting-candidate-row">
                              <span className="type-icon" aria-hidden="true">
                                {typeIcon(candidate.candidate_type)}
                              </span>
                              <span className="meeting-candidate-statement">{candidate.statement}</span>
                              <StatusBadge value={candidate.validation_state} />
                              <span className="meeting-candidate-confidence">{Math.round(candidate.confidence * 100)}%</span>
                              {candidate.validation_state === "candidate" && (
                                <span className="actions-cell">
                                  <button
                                    className="accept-button"
                                    disabled={pendingId === candidate.candidate_id}
                                    onClick={() => handleCandidateAction(acceptCandidate, candidate.candidate_id)}
                                  >
                                    {pendingId === candidate.candidate_id ? "…" : "Accept"}
                                  </button>
                                  <button
                                    className="reject-button"
                                    disabled={pendingId === candidate.candidate_id}
                                    onClick={() => handleCandidateAction(rejectCandidate, candidate.candidate_id)}
                                  >
                                    {pendingId === candidate.candidate_id ? "…" : "Reject"}
                                  </button>
                                </span>
                              )}
                              {candidate.validation_state === "accepted" && (
                                <span className="actions-cell">
                                  <button
                                    className="promote-button"
                                    disabled={pendingId === candidate.candidate_id}
                                    onClick={() => handleCandidateAction(promoteCandidate, candidate.candidate_id)}
                                  >
                                    {pendingId === candidate.candidate_id ? "…" : "Promote to Obligation"}
                                  </button>
                                </span>
                              )}
                            </li>
                          ))}
                        </ul>
                      )}
                    </li>
                  ))}
                </ol>
              </>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
