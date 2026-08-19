use crate::extraction::ALLOWED_CANDIDATE_TYPES;
use crate::model_adapter::{self, ModelAdapterError, ModelConfig};
use serde_json::Value as Json;
use sqlx::{FromRow, PgPool};
use std::collections::HashSet;
use uuid::Uuid;

/// ADR-0094: never panics on a malformed or unreachable model response;
/// callers see a typed error, matching extraction's own posture.
#[derive(Debug)]
pub enum SynthesisError {
    Database(sqlx::Error),
    Model(ModelAdapterError),
    Parse(String),
}

impl std::fmt::Display for SynthesisError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Database(error) => write!(f, "{error}"),
            Self::Model(error) => write!(f, "{error}"),
            Self::Parse(reason) => write!(f, "invalid synthesis response: {reason}"),
        }
    }
}

impl std::error::Error for SynthesisError {}

impl From<sqlx::Error> for SynthesisError {
    fn from(error: sqlx::Error) -> Self {
        Self::Database(error)
    }
}

#[derive(Debug, Clone, FromRow)]
struct AcceptedCandidate {
    candidate_id: Uuid,
    candidate_type: String,
    statement: String,
    confidence: Option<f32>,
}

const SYNTHESIS_PROMPT_PREAMBLE: &str = "You are given several short statements, each extracted \
independently from a different fragment of the SAME source document. Some describe the same \
underlying goal, commitment, decision, or topic and were only split apart by chunking -- merge \
those into ONE clear, synthesized statement. A statement with nothing else to merge with should \
still become its own one-member group; never omit one. Respond with a JSON array only, each item \
shaped exactly as: {\"synthesized_statement\": string, \"candidate_type\": one of \
[\"commitment\",\"request\",\"risk\",\"follow_up\",\"decision\",\"expectation\"], \
\"member_candidate_ids\": [string, ...]}.";

fn build_prompt(candidates: &[AcceptedCandidate]) -> String {
    let mut body = String::new();
    for candidate in candidates {
        body.push_str(&format!(
            "- id={} type={} confidence={:?}: {}\n",
            candidate.candidate_id, candidate.candidate_type, candidate.confidence, candidate.statement
        ));
    }
    format!("{SYNTHESIS_PROMPT_PREAMBLE}\n\nCandidates:\n{body}")
}

fn extract_json_array(raw: &str) -> Option<&str> {
    let start = raw.find('[')?;
    let end = raw.rfind(']')?;
    (end >= start).then(|| &raw[start..=end])
}

async fn insert_group<'e, E>(
    executor: E,
    source_id: Uuid,
    synthesized_statement: &str,
    candidate_type: &str,
    member_candidate_ids: &[Uuid],
    synthesis_model: &str,
) -> Result<Uuid, sqlx::Error>
where
    E: sqlx::PgExecutor<'e>,
{
    let (id,): (Uuid,) = sqlx::query_as(
        "INSERT INTO candidate_synthesis_groups \
            (source_id, synthesized_statement, candidate_type, member_candidate_ids, synthesis_model) \
         VALUES ($1, $2, $3, $4, $5) RETURNING id",
    )
    .bind(source_id)
    .bind(synthesized_statement)
    .bind(candidate_type)
    .bind(member_candidate_ids)
    .bind(synthesis_model)
    .fetch_one(executor)
    .await?;
    Ok(id)
}

/// ADR-0094: re-assembles one source's still-`accepted` candidates into a
/// smaller set of synthesized groups. Manual and synchronous, matching
/// extraction's own "never automatic on ingestion" posture -- run this on
/// explicit request for one source, not on every accept. Every accepted
/// candidate is accounted for: the model's own groups are inserted first,
/// then anything it left ungrouped (or an invalid/unparseable response)
/// becomes its own one-member group rather than being silently dropped.
pub async fn synthesize_candidates_for_source(
    pool: &PgPool,
    config: &ModelConfig,
    source_id: Uuid,
) -> Result<Vec<Uuid>, SynthesisError> {
    let candidates: Vec<AcceptedCandidate> = sqlx::query_as(
        "SELECT cp.candidate_id, cp.candidate_type, cp.statement, cp.confidence \
         FROM candidate_projection cp \
         JOIN source_fragments sf ON sf.id = cp.source_fragment_id \
         WHERE sf.source_id = $1 AND cp.validation_state = 'accepted' \
         ORDER BY sf.sequence NULLS LAST, cp.candidate_id",
    )
    .bind(source_id)
    .fetch_all(pool)
    .await?;

    if candidates.is_empty() {
        return Ok(Vec::new());
    }

    let valid_ids: HashSet<Uuid> = candidates.iter().map(|c| c.candidate_id).collect();
    let mut covered: HashSet<Uuid> = HashSet::new();
    let mut group_ids = Vec::new();

    let prompt = build_prompt(&candidates);
    let model_groups = match model_adapter::complete(config, &prompt).await {
        Ok(raw_response) => parse_model_groups(&raw_response),
        Err(error) => return Err(SynthesisError::Model(error)),
    };

    for (statement, candidate_type, member_ids) in model_groups {
        let member_ids: Vec<Uuid> = member_ids
            .into_iter()
            .filter(|id| valid_ids.contains(id) && !covered.contains(id))
            .collect();
        if member_ids.is_empty() {
            continue;
        }
        let id = insert_group(
            pool,
            source_id,
            &statement,
            &candidate_type,
            &member_ids,
            &config.model,
        )
        .await?;
        covered.extend(member_ids.iter().copied());
        group_ids.push(id);
    }

    // Never drop a candidate: anything the model didn't cover (including
    // every candidate, if the response was empty or unparseable) becomes
    // its own one-member group.
    for candidate in &candidates {
        if covered.contains(&candidate.candidate_id) {
            continue;
        }
        let id = insert_group(
            pool,
            source_id,
            &candidate.statement,
            &candidate.candidate_type,
            std::slice::from_ref(&candidate.candidate_id),
            &config.model,
        )
        .await?;
        group_ids.push(id);
    }

    Ok(group_ids)
}

/// Parses the model's JSON array response into `(statement, candidate_type,
/// member_candidate_ids)` tuples, silently skipping any item missing a
/// required field, an unrecognized `candidate_type`, or an unparseable
/// member id -- never erroring the whole call over one malformed entry. An
/// entirely unparseable response degrades to an empty list, which the
/// caller's "never drop a candidate" fallback then covers.
fn parse_model_groups(raw_response: &str) -> Vec<(String, String, Vec<Uuid>)> {
    let Some(json_text) = extract_json_array(raw_response) else {
        return Vec::new();
    };
    let Ok(parsed) = serde_json::from_str::<Vec<Json>>(json_text) else {
        return Vec::new();
    };

    parsed
        .into_iter()
        .filter_map(|item| {
            let statement = item
                .get("synthesized_statement")
                .and_then(|v| v.as_str())?
                .to_string();
            let candidate_type = item.get("candidate_type").and_then(|v| v.as_str())?;
            if !ALLOWED_CANDIDATE_TYPES.contains(&candidate_type) {
                return None;
            }
            let member_ids: Vec<Uuid> = item
                .get("member_candidate_ids")
                .and_then(|v| v.as_array())
                .into_iter()
                .flatten()
                .filter_map(|v| v.as_str())
                .filter_map(|s| Uuid::parse_str(s).ok())
                .collect();
            if member_ids.is_empty() {
                return None;
            }
            Some((statement, candidate_type.to_string(), member_ids))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{extraction, graph};
    use sqlx::postgres::PgPoolOptions;

    async fn test_pool() -> PgPool {
        let database_url = std::env::var("DATABASE_URL")
            .expect("DATABASE_URL must be set to run synthesis tests");
        crate::guard_test_database(&database_url);
        PgPoolOptions::new()
            .max_connections(2)
            .connect(&database_url)
            .await
            .expect("connect to test database")
    }

    #[test]
    fn parse_model_groups_skips_an_item_with_no_valid_members() {
        let raw = r#"[{"synthesized_statement": "x", "candidate_type": "commitment", "member_candidate_ids": ["not-a-uuid"]}]"#;
        assert!(parse_model_groups(raw).is_empty());
    }

    #[test]
    fn parse_model_groups_skips_an_unrecognized_candidate_type() {
        let id = Uuid::new_v4();
        let raw = format!(
            r#"[{{"synthesized_statement": "x", "candidate_type": "not_a_real_type", "member_candidate_ids": ["{id}"]}}]"#
        );
        assert!(parse_model_groups(&raw).is_empty());
    }

    #[test]
    fn parse_model_groups_extracts_a_well_formed_group_from_noisy_prose() {
        let id = Uuid::new_v4();
        let raw = format!(
            "Sure, here you go:\n[{{\"synthesized_statement\": \"Adopt tooling and measure it\", \"candidate_type\": \"commitment\", \"member_candidate_ids\": [\"{id}\"]}}]\nHope that helps!"
        );
        let groups = parse_model_groups(&raw);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].0, "Adopt tooling and measure it");
        assert_eq!(groups[0].1, "commitment");
        assert_eq!(groups[0].2, vec![id]);
    }

    #[tokio::test]
    async fn synthesize_returns_no_groups_for_a_source_with_no_accepted_candidates() {
        let pool = test_pool().await;
        let source_id = Uuid::new_v4();
        // Only reachable if a live model is configured -- otherwise the
        // early "no candidates" return happens before any model call, so
        // this assertion holds regardless (matches ADR-0011's own
        // graceful-skip posture for anything that *would* need a live
        // model beyond this point).
        let config = ModelConfig {
            url: "http://127.0.0.1:0".to_string(),
            model: "unused".to_string(),
            api_key: None,
        };
        let groups = synthesize_candidates_for_source(&pool, &config, source_id)
            .await
            .expect("no candidates means no model call, so this never errors");
        assert!(groups.is_empty());
    }

    #[tokio::test]
    async fn synthesis_table_rejects_update_and_delete() {
        let pool = test_pool().await;
        let source_id = Uuid::new_v4();
        let member_id = Uuid::new_v4();
        let id = insert_group(&pool, source_id, "x", "commitment", &[member_id], "test-model")
            .await
            .expect("insert a synthesis group");

        let update_result = sqlx::query("UPDATE candidate_synthesis_groups SET synthesized_statement = 'y' WHERE id = $1")
            .bind(id)
            .execute(&pool)
            .await;
        assert!(update_result.is_err(), "UPDATE must be rejected (ADR-0094)");

        let delete_result = sqlx::query("DELETE FROM candidate_synthesis_groups WHERE id = $1")
            .bind(id)
            .execute(&pool)
            .await;
        assert!(delete_result.is_err(), "DELETE must be rejected (ADR-0094)");
    }

    #[tokio::test]
    async fn synthesize_never_drops_a_candidate_when_no_live_model_is_configured() {
        if ModelConfig::from_env().is_some() {
            eprintln!("skipped: a live model IS configured, this test only covers the no-model degrade path");
            return;
        }
        let pool = test_pool().await;
        let source_id = Uuid::new_v4();
        let fragment_id = graph::create_source_fragment(&pool, source_id, "one sentence", "synthesis-test-hash")
            .await
            .expect("create source fragment");
        let candidate_id = Uuid::new_v4();
        extraction::extract_candidate(&pool, candidate_id, "commitment", "one sentence", fragment_id, None, None)
            .await
            .expect("extract a candidate");
        extraction::transition_candidate(&pool, candidate_id, "accepted", serde_json::json!({}))
            .await
            .expect("accept the candidate");
        extraction::rebuild_candidate_projection(&pool)
            .await
            .expect("rebuild candidate projection");

        let config = ModelConfig {
            url: "http://127.0.0.1:0".to_string(),
            model: "unused".to_string(),
            api_key: None,
        };
        let result = synthesize_candidates_for_source(&pool, &config, source_id).await;
        // An unreachable model endpoint is a Model error, not a dropped
        // candidate -- the caller learns synthesis didn't run, rather than
        // silently losing the candidate from view.
        assert!(matches!(result, Err(SynthesisError::Model(_))));
    }
}
