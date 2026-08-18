use crate::graph;
use serde_json::{json, Value as Json};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use std::collections::HashSet;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct MeetingMetadata {
    pub title: String,
    pub occurred_at: Option<chrono::DateTime<chrono::Utc>>,
    pub organiser: Option<String>,
    pub participants: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscriptTurn {
    pub speaker: String,
    pub text: String,
}

/// Splits raw transcript text into speaker turns, not arbitrary character
/// count (docs/PRODUCT-SPEC.md SS6.2 step 2). Placeholder "Speaker: text"
/// line format pending a real provider integration (Teams/Scout, Epic E10).
pub fn parse_transcript(raw_text: &str) -> Vec<TranscriptTurn> {
    raw_text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| match line.split_once(':') {
            Some((speaker, text)) if !speaker.trim().is_empty() => TranscriptTurn {
                speaker: speaker.trim().to_string(),
                text: text.trim().to_string(),
            },
            _ => TranscriptTurn {
                speaker: "unknown".to_string(),
                text: line.to_string(),
            },
        })
        .collect()
}

fn sha256_hex(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// ADR-0069: resolves each unique participant/speaker name against an
/// existing Person node by exact, case-insensitive `canonical_text` match
/// (ADR-0060's own resolution, reused here at ingestion time). A match
/// creates a `participated_in` edge from the person to the source node; no
/// match creates nothing -- no Person node is ever fabricated.
async fn link_participants(
    tx: &mut sqlx::PgConnection,
    source_node_id: Uuid,
    names: &[String],
) -> Result<(), sqlx::Error> {
    let mut linked_person_ids = HashSet::new();
    let mut seen_names = HashSet::new();
    for name in names {
        let trimmed = name.trim();
        if trimmed.is_empty() || !seen_names.insert(trimmed.to_lowercase()) {
            continue;
        }
        let person_id: Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM nodes WHERE node_type = 'person' AND lower(canonical_text) = lower($1) LIMIT 1",
        )
        .bind(trimmed)
        .fetch_optional(&mut *tx)
        .await?;
        if let Some(person_id) = person_id {
            if linked_person_ids.insert(person_id) {
                graph::create_edge(
                    &mut *tx,
                    person_id,
                    source_node_id,
                    "participated_in",
                    Some(1.0),
                )
                .await?;
            }
        }
    }
    Ok(())
}

/// Splits raw text into blank-line-separated paragraphs (ADR-0040), for any
/// source that doesn't carry a transcript's speaker-turn shape (email,
/// note, Teams message, ...). A single-paragraph submission becomes
/// exactly one fragment; consecutive blank lines never produce an empty one.
pub fn split_paragraphs(raw_text: &str) -> Vec<String> {
    let mut paragraphs = Vec::new();
    let mut current: Vec<&str> = Vec::new();
    for line in raw_text.lines() {
        if line.trim().is_empty() {
            if !current.is_empty() {
                paragraphs.push(current.join(" ").trim().to_string());
                current.clear();
            }
        } else {
            current.push(line.trim());
        }
    }
    if !current.is_empty() {
        paragraphs.push(current.join(" ").trim().to_string());
    }
    paragraphs
}

#[derive(Debug, Clone)]
pub struct IngestedTranscript {
    pub meeting_id: Uuid,
    pub fragment_ids: Vec<Uuid>,
}

/// Ingests one raw transcript (docs/PRODUCT-SPEC.md SS6.1/SS6.2 steps 1-2):
/// creates a Meeting node carrying an immutable raw-transcript hash, then
/// chunks the text into per-speaker-turn, immutable source fragments
/// (ADR-0010). Does not deduplicate meetings/fragments, extract candidates,
/// or generate embeddings; see ADR-0010's scope. Atomic (ADR-0034): the
/// Meeting node and every fragment are written in one transaction, so a
/// storage failure partway through can never leave partial meeting memory.
pub async fn ingest_transcript(
    pool: &PgPool,
    metadata: &MeetingMetadata,
    raw_text: &str,
) -> Result<IngestedTranscript, sqlx::Error> {
    let attributes: Json = json!({
        "occurred_at": metadata.occurred_at.map(|value| value.to_rfc3339()),
        "organiser": metadata.organiser,
        "participants": metadata.participants,
        "raw_transcript_hash": sha256_hex(raw_text),
    });

    let mut tx = pool.begin().await?;

    let (meeting_id,): (Uuid,) = sqlx::query_as(
        "INSERT INTO nodes (node_type, canonical_text, attributes, occurred_at) VALUES ($1, $2, $3, $4) RETURNING id",
    )
    .bind("meeting")
    .bind(&metadata.title)
    .bind(&attributes)
    .bind(metadata.occurred_at)
    .fetch_one(&mut *tx)
    .await?;

    let turns = parse_transcript(raw_text);
    let mut fragment_ids = Vec::new();
    for (sequence, turn) in turns.iter().enumerate() {
        let hash = sha256_hex(&turn.text);
        let (fragment_id,): (Uuid,) = sqlx::query_as(
            "INSERT INTO source_fragments (source_id, text, speaker, hash, sequence) \
             VALUES ($1, $2, $3, $4, $5) RETURNING id",
        )
        .bind(meeting_id)
        .bind(&turn.text)
        .bind(&turn.speaker)
        .bind(&hash)
        .bind(sequence as i32)
        .fetch_one(&mut *tx)
        .await?;
        fragment_ids.push(fragment_id);
    }

    let names: Vec<String> = metadata
        .participants
        .iter()
        .cloned()
        .chain(turns.iter().map(|turn| turn.speaker.clone()))
        .collect();
    link_participants(&mut tx, meeting_id, &names).await?;

    tx.commit().await?;

    embed_fragments_best_effort(pool, &fragment_ids).await;
    Ok(IngestedTranscript {
        meeting_id,
        fragment_ids,
    })
}

#[derive(Debug, Clone)]
pub struct SourceMetadata {
    pub source_type: String,
    pub title: String,
    pub occurred_at: chrono::DateTime<chrono::Utc>,
    pub participants: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct IngestedSource {
    pub node_id: Uuid,
    pub fragment_ids: Vec<Uuid>,
}

/// Ingests one dated source of any kind (meeting, email, note, Teams
/// message, ...) into one node plus its ordered, immutable, hashed
/// fragments (ADR-0040) -- the one function every ingestion surface (API,
/// CLI, MCP) calls, so none re-implements validation, splitting, or the
/// transaction. `source_type: "meeting"` keeps ingest_transcript's
/// per-speaker-turn split; anything else splits by paragraph, no speaker
/// field. Atomic, matching ADR-0034's posture exactly: one transaction,
/// never triggers extraction or embedding.
pub async fn ingest_source(
    pool: &PgPool,
    metadata: &SourceMetadata,
    raw_text: &str,
) -> Result<IngestedSource, sqlx::Error> {
    let attributes: Json = json!({
        "occurred_at": metadata.occurred_at.to_rfc3339(),
        "participants": metadata.participants,
        "raw_source_hash": sha256_hex(raw_text),
    });

    let mut tx = pool.begin().await?;

    let (node_id,): (Uuid,) = sqlx::query_as(
        "INSERT INTO nodes (node_type, canonical_text, attributes, occurred_at) VALUES ($1, $2, $3, $4) RETURNING id",
    )
    .bind(&metadata.source_type)
    .bind(&metadata.title)
    .bind(&attributes)
    .bind(metadata.occurred_at)
    .fetch_one(&mut *tx)
    .await?;

    let mut fragment_ids = Vec::new();
    let mut speaker_names: Vec<String> = Vec::new();

    if metadata.source_type == "meeting" {
        for (sequence, turn) in parse_transcript(raw_text).into_iter().enumerate() {
            let hash = sha256_hex(&turn.text);
            let (fragment_id,): (Uuid,) = sqlx::query_as(
                "INSERT INTO source_fragments (source_id, text, speaker, hash, sequence) \
                 VALUES ($1, $2, $3, $4, $5) RETURNING id",
            )
            .bind(node_id)
            .bind(&turn.text)
            .bind(&turn.speaker)
            .bind(&hash)
            .bind(sequence as i32)
            .fetch_one(&mut *tx)
            .await?;
            fragment_ids.push(fragment_id);
            speaker_names.push(turn.speaker);
        }
    } else {
        for (sequence, paragraph) in split_paragraphs(raw_text).into_iter().enumerate() {
            let hash = sha256_hex(&paragraph);
            let (fragment_id,): (Uuid,) = sqlx::query_as(
                "INSERT INTO source_fragments (source_id, text, speaker, hash, sequence) \
                 VALUES ($1, $2, NULL, $3, $4) RETURNING id",
            )
            .bind(node_id)
            .bind(&paragraph)
            .bind(&hash)
            .bind(sequence as i32)
            .fetch_one(&mut *tx)
            .await?;
            fragment_ids.push(fragment_id);
        }
    }

    let names: Vec<String> = metadata
        .participants
        .iter()
        .cloned()
        .chain(speaker_names)
        .collect();
    link_participants(&mut tx, node_id, &names).await?;

    tx.commit().await?;

    embed_fragments_best_effort(pool, &fragment_ids).await;
    Ok(IngestedSource {
        node_id,
        fragment_ids,
    })
}

/// ADR-0062: best-effort auto-embedding after an ingest commits. Never fails
/// the ingest -- when no embedding model is configured, or a call fails, the
/// fragment simply stays unembedded (ADR-0018's non-blocking guarantee). Runs
/// after the transaction so a slow or failing model call can neither hold the
/// ingest transaction open nor roll it back.
async fn embed_fragments_best_effort(pool: &PgPool, fragment_ids: &[Uuid]) {
    let Some(config) = crate::embedding_adapter::EmbeddingConfig::from_env() else {
        return;
    };
    for &fragment_id in fragment_ids {
        if let Err(error) = crate::graph::embed_source_fragment(pool, &config, fragment_id).await {
            eprintln!("auto-embed skipped for fragment {fragment_id}: {error}");
        }
    }
}

/// ADR-0063: backfills embeddings for every source fragment that has none yet
/// (fragments ingested before ADR-0062's auto-embed, or while no model was
/// configured). Best-effort per fragment: a failed embed is logged and
/// skipped. Only appends to the embeddings table; never touches an immutable
/// fragment (ADR-0010). Returns (candidates_found, embedded_ok).
pub async fn reindex_unembedded_fragments(
    pool: &PgPool,
    config: &crate::embedding_adapter::EmbeddingConfig,
) -> Result<(usize, usize), sqlx::Error> {
    let rows: Vec<(Uuid,)> = sqlx::query_as(
        "SELECT sf.id FROM source_fragments sf \
         LEFT JOIN embeddings e ON e.entity_id = sf.id AND e.entity_type = 'source_fragment' \
         WHERE e.id IS NULL ORDER BY sf.id",
    )
    .fetch_all(pool)
    .await?;
    let candidates = rows.len();
    let mut embedded = 0usize;
    for (fragment_id,) in rows {
        match crate::graph::embed_source_fragment(pool, config, fragment_id).await {
            Ok(_) => embedded += 1,
            Err(error) => eprintln!("reindex skipped fragment {fragment_id}: {error}"),
        }
    }
    Ok((candidates, embedded))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::postgres::PgPoolOptions;

    async fn test_pool() -> PgPool {
        let database_url = std::env::var("DATABASE_URL")
            .expect("DATABASE_URL must be set to run transcript tests");
        crate::guard_test_database(&database_url);
        PgPoolOptions::new()
            .max_connections(2)
            .connect(&database_url)
            .await
            .expect("connect to test database")
    }

    #[tokio::test]
    async fn ingest_auto_embeds_fragments_when_a_model_is_configured() {
        let pool = test_pool().await;
        let metadata = SourceMetadata {
            source_type: "note".to_string(),
            title: "Auto-embed on ingest".to_string(),
            occurred_at: chrono::Utc::now(),
            participants: vec![],
        };
        let marker = format!("auto-embed marker {}", Uuid::new_v4());
        let ingested = ingest_source(&pool, &metadata, &marker)
            .await
            .expect("ingest source");
        assert!(
            !ingested.fragment_ids.is_empty(),
            "ingest must create at least one fragment"
        );

        // ADR-0062: with a model configured, ingest auto-embeds; without one it
        // stays a no-op and ingest still succeeds (proven by reaching here).
        if crate::embedding_adapter::EmbeddingConfig::from_env().is_some() {
            let (count,): (i64,) =
                sqlx::query_as("SELECT COUNT(*) FROM embeddings WHERE entity_id = ANY($1)")
                    .bind(&ingested.fragment_ids)
                    .fetch_one(&pool)
                    .await
                    .expect("count embeddings for the ingested fragments");
            assert!(
                count > 0,
                "with an embedding model configured, ingest must auto-embed at least one fragment"
            );
        } else {
            eprintln!("skipped embedding assertion: RINGMASTER_EMBEDDING_URL is not set");
        }
    }

    #[tokio::test]
    async fn reindex_embeds_previously_unembedded_fragments_when_a_model_is_configured() {
        let pool = test_pool().await;
        // Created directly (not via ingest), so it starts with no embedding.
        let fragment_id = crate::graph::create_source_fragment(
            &pool,
            Uuid::new_v4(),
            &format!("reindex backfill marker {}", Uuid::new_v4()),
            &format!("reindex-hash-{}", Uuid::new_v4()),
        )
        .await
        .expect("create source fragment");

        let Some(config) = crate::embedding_adapter::EmbeddingConfig::from_env() else {
            eprintln!("skipped reindex assertion: RINGMASTER_EMBEDDING_URL is not set");
            return;
        };
        let (candidates, embedded) = reindex_unembedded_fragments(&pool, &config)
            .await
            .expect("reindex");
        assert!(
            candidates >= 1 && embedded >= 1,
            "reindex must find and embed at least our fragment"
        );

        let (count,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM embeddings WHERE entity_id = $1")
                .bind(fragment_id)
                .fetch_one(&pool)
                .await
                .expect("count embeddings for the fragment");
        assert!(
            count > 0,
            "the previously unembedded fragment must have an embedding after reindex"
        );
    }

    #[test]
    fn parse_transcript_splits_by_speaker_turn_not_character_count() {
        let raw =
            "Roopa: We have a two-week transition.\nJohn: I need to follow up on the training.\n";
        let turns = parse_transcript(raw);
        assert_eq!(turns.len(), 2);
        assert_eq!(turns[0].speaker, "Roopa");
        assert_eq!(turns[0].text, "We have a two-week transition.");
        assert_eq!(turns[1].speaker, "John");
        assert_eq!(turns[1].text, "I need to follow up on the training.");
    }

    #[tokio::test]
    async fn ingest_transcript_creates_a_meeting_node_and_hashed_source_fragments() {
        let pool = test_pool().await;
        let metadata = MeetingMetadata {
            title: "Weekly 1:1".to_string(),
            occurred_at: Some(
                chrono::DateTime::parse_from_rfc3339("2026-08-14T00:00:00Z")
                    .unwrap()
                    .with_timezone(&chrono::Utc),
            ),
            organiser: Some("Lyndon".to_string()),
            participants: vec!["Lyndon".to_string(), "Roopa".to_string()],
        };
        let raw =
            "Roopa: We have a two-week transition.\nJohn: I need to follow up on the training.";

        let ingested = ingest_transcript(&pool, &metadata, raw)
            .await
            .expect("ingest transcript");
        assert_eq!(ingested.fragment_ids.len(), 2);

        let meeting = crate::graph::get_node(&pool, ingested.meeting_id)
            .await
            .expect("read meeting node");
        assert_eq!(meeting.node_type, "meeting");
        assert_eq!(meeting.canonical_text, "Weekly 1:1");
        assert!(meeting
            .attributes
            .get("raw_transcript_hash")
            .and_then(|value| value.as_str())
            .is_some());
    }

    #[tokio::test]
    async fn source_fragments_cannot_be_mutated_or_deleted() {
        let pool = test_pool().await;
        let metadata = MeetingMetadata {
            title: "Test meeting".to_string(),
            occurred_at: None,
            organiser: None,
            participants: vec![],
        };
        let ingested = ingest_transcript(&pool, &metadata, "Alice: hello")
            .await
            .expect("ingest transcript");
        let fragment_id = ingested.fragment_ids[0];

        let update_result =
            sqlx::query("UPDATE source_fragments SET text = 'tampered' WHERE id = $1")
                .bind(fragment_id)
                .execute(&pool)
                .await;
        assert!(
            update_result.is_err(),
            "UPDATE must be rejected by the append-only trigger"
        );

        let delete_result = sqlx::query("DELETE FROM source_fragments WHERE id = $1")
            .bind(fragment_id)
            .execute(&pool)
            .await;
        assert!(
            delete_result.is_err(),
            "DELETE must be rejected by the append-only trigger"
        );
    }

    #[test]
    fn split_paragraphs_groups_lines_by_blank_line_separator() {
        let raw = "First paragraph,\nstill first.\n\nSecond paragraph.\n\n\nThird, after extra blank lines.";
        let paragraphs = split_paragraphs(raw);
        assert_eq!(
            paragraphs,
            vec![
                "First paragraph, still first.",
                "Second paragraph.",
                "Third, after extra blank lines."
            ]
        );
    }

    #[test]
    fn split_paragraphs_of_a_single_paragraph_is_exactly_one_fragment() {
        assert_eq!(
            split_paragraphs("Just one paragraph, no blank lines at all."),
            vec!["Just one paragraph, no blank lines at all."]
        );
    }

    #[tokio::test]
    async fn ingest_source_creates_a_node_with_occurred_at_and_paragraph_fragments_for_a_non_meeting_type(
    ) {
        let pool = test_pool().await;
        let occurred_at = chrono::DateTime::parse_from_rfc3339("2026-08-01T09:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        let metadata = SourceMetadata {
            source_type: "email".to_string(),
            title: "Re: transition plan".to_string(),
            occurred_at,
            participants: vec!["Roopa".to_string()],
        };
        let raw = "First paragraph of the email.\n\nSecond paragraph.";

        let ingested = ingest_source(&pool, &metadata, raw)
            .await
            .expect("ingest source");
        assert_eq!(ingested.fragment_ids.len(), 2);

        let node = crate::graph::get_node(&pool, ingested.node_id)
            .await
            .expect("read node");
        assert_eq!(node.node_type, "email");
        assert_eq!(node.canonical_text, "Re: transition plan");

        let (stored_occurred_at,): (Option<chrono::DateTime<chrono::Utc>>,) =
            sqlx::query_as("SELECT occurred_at FROM nodes WHERE id = $1")
                .bind(ingested.node_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(stored_occurred_at, Some(occurred_at));

        let (speaker,): (Option<String>,) =
            sqlx::query_as("SELECT speaker FROM source_fragments WHERE id = $1")
                .bind(ingested.fragment_ids[0])
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(speaker, None, "a non-meeting fragment carries no speaker");
    }

    #[tokio::test]
    async fn ingest_source_of_a_meeting_type_keeps_the_per_speaker_turn_split() {
        let pool = test_pool().await;
        let metadata = SourceMetadata {
            source_type: "meeting".to_string(),
            title: "Ingest source meeting test".to_string(),
            occurred_at: chrono::Utc::now(),
            participants: vec![],
        };
        let raw = "Roopa: We have a two-week transition.\nJohn: I need to follow up.";

        let ingested = ingest_source(&pool, &metadata, raw)
            .await
            .expect("ingest source");
        assert_eq!(ingested.fragment_ids.len(), 2);

        let (speaker,): (Option<String>,) =
            sqlx::query_as("SELECT speaker FROM source_fragments WHERE id = $1")
                .bind(ingested.fragment_ids[0])
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(speaker.as_deref(), Some("Roopa"));
    }

    /// ADR-0069: an exact, case-insensitive match against an existing Person
    /// node -- whether named in `participants` or as a fragment `speaker` --
    /// creates a `participated_in` edge in the same transaction as ingestion.
    #[tokio::test]
    async fn ingest_source_creates_participated_in_edge_on_exact_participant_match() {
        let pool = test_pool().await;
        let person_name = format!("Roopa Venkat {}", Uuid::new_v4());
        let person_id = crate::graph::create_node(&pool, "person", &person_name, json!({}))
            .await
            .expect("create person node");

        let metadata = SourceMetadata {
            source_type: "note".to_string(),
            title: "Participant linking test".to_string(),
            occurred_at: chrono::Utc::now(),
            // Deliberately different case than the stored canonical_text.
            participants: vec![person_name.to_uppercase()],
        };
        let ingested = ingest_source(&pool, &metadata, "A note mentioning the participant.")
            .await
            .expect("ingest source");

        let edges =
            crate::graph::list_edges_for_node(&pool, person_id, Some("participated_in"), false)
                .await
                .expect("list edges for person");
        assert_eq!(
            edges.len(),
            1,
            "an exact case-insensitive participant match must create exactly one edge"
        );
        assert_eq!(edges[0].to_id, ingested.node_id);
        assert_eq!(edges[0].from_id, person_id);
    }

    /// ADR-0069: a fragment `speaker` that exactly matches an existing Person
    /// node also creates a `participated_in` edge, via ingest_transcript's
    /// per-speaker-turn path.
    #[tokio::test]
    async fn ingest_transcript_creates_participated_in_edge_on_exact_speaker_match() {
        let pool = test_pool().await;
        let person_name = format!("Speaker Person {}", Uuid::new_v4());
        let person_id = crate::graph::create_node(&pool, "person", &person_name, json!({}))
            .await
            .expect("create person node");

        let metadata = MeetingMetadata {
            title: "Speaker linking test".to_string(),
            occurred_at: None,
            organiser: None,
            participants: vec![],
        };
        let raw = format!("{}: hello from the transcript.", person_name);
        let ingested = ingest_transcript(&pool, &metadata, &raw)
            .await
            .expect("ingest transcript");

        let edges =
            crate::graph::list_edges_for_node(&pool, person_id, Some("participated_in"), false)
                .await
                .expect("list edges for person");
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].to_id, ingested.meeting_id);
    }

    /// ADR-0069: a participant/speaker name with no exact Person match creates
    /// no edge and fabricates no Person node -- ingestion is otherwise
    /// unchanged, matching ADR-0060's own no-fabrication precedent.
    #[tokio::test]
    async fn ingest_source_creates_no_edge_or_person_node_without_an_exact_match() {
        let pool = test_pool().await;
        let unmatched_name = format!("Nobody Named {}", Uuid::new_v4());
        let metadata = SourceMetadata {
            source_type: "note".to_string(),
            title: "No participant match test".to_string(),
            occurred_at: chrono::Utc::now(),
            participants: vec![unmatched_name.clone()],
        };

        let ingested = ingest_source(&pool, &metadata, "A note with an unmatched participant.")
            .await
            .expect("ingest source");

        let (person_count,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM nodes WHERE node_type = 'person' AND canonical_text = $1",
        )
        .bind(&unmatched_name)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(
            person_count, 0,
            "an unmatched participant name must never fabricate a Person node"
        );

        let (edge_count,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM edges WHERE to_id = $1 AND edge_type = 'participated_in'",
        )
        .bind(ingested.node_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(
            edge_count, 0,
            "no participated_in edge is created without an exact match"
        );
    }
}
