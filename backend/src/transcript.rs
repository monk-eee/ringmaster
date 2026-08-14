use serde_json::{json, Value as Json};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct MeetingMetadata {
    pub title: String,
    pub date: Option<String>,
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
        "date": metadata.date,
        "organiser": metadata.organiser,
        "participants": metadata.participants,
        "raw_transcript_hash": sha256_hex(raw_text),
    });

    let mut tx = pool.begin().await?;

    let (meeting_id,): (Uuid,) = sqlx::query_as(
        "INSERT INTO nodes (node_type, canonical_text, attributes) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind("meeting")
    .bind(&metadata.title)
    .bind(&attributes)
    .fetch_one(&mut *tx)
    .await?;

    let mut fragment_ids = Vec::new();
    for (sequence, turn) in parse_transcript(raw_text).into_iter().enumerate() {
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

    tx.commit().await?;

    Ok(IngestedTranscript { meeting_id, fragment_ids })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::postgres::PgPoolOptions;

    async fn test_pool() -> PgPool {
        let database_url =
            std::env::var("DATABASE_URL").expect("DATABASE_URL must be set to run transcript tests");
        PgPoolOptions::new()
            .max_connections(2)
            .connect(&database_url)
            .await
            .expect("connect to test database")
    }

    #[test]
    fn parse_transcript_splits_by_speaker_turn_not_character_count() {
        let raw = "Roopa: We have a two-week transition.\nJohn: I need to follow up on the training.\n";
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
            date: Some("2026-08-14".to_string()),
            organiser: Some("Lyndon".to_string()),
            participants: vec!["Lyndon".to_string(), "Roopa".to_string()],
        };
        let raw = "Roopa: We have a two-week transition.\nJohn: I need to follow up on the training.";

        let ingested = ingest_transcript(&pool, &metadata, raw).await.expect("ingest transcript");
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
            date: None,
            organiser: None,
            participants: vec![],
        };
        let ingested = ingest_transcript(&pool, &metadata, "Alice: hello")
            .await
            .expect("ingest transcript");
        let fragment_id = ingested.fragment_ids[0];

        let update_result = sqlx::query("UPDATE source_fragments SET text = 'tampered' WHERE id = $1")
            .bind(fragment_id)
            .execute(&pool)
            .await;
        assert!(update_result.is_err(), "UPDATE must be rejected by the append-only trigger");

        let delete_result = sqlx::query("DELETE FROM source_fragments WHERE id = $1")
            .bind(fragment_id)
            .execute(&pool)
            .await;
        assert!(delete_result.is_err(), "DELETE must be rejected by the append-only trigger");
    }
}
