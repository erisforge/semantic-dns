use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use sqlx_core::{Error as SqlxError, query::query, row::Row, types::Json};
use sqlx_postgres::{PgPool, PgPoolOptions};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuditEvent {
    pub event_type: String,
    pub payload: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuditEventRecord {
    pub id: i64,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub previous_hash: Option<String>,
    pub current_hash: String,
}

#[derive(Debug, Clone)]
pub struct AuditLedger {
    pool: PgPool,
    schema: String,
}

impl AuditLedger {
    pub async fn open(database_url: &str, schema: &str) -> Result<Self, SqlxError> {
        let schema = validate_schema_name(schema)?;
        let pool = PgPoolOptions::new()
            .max_connections(4)
            .connect(database_url)
            .await?;
        let ledger = Self { pool, schema };
        ledger.run_migrations().await?;
        Ok(ledger)
    }

    fn table(&self) -> String {
        format!("{}.audit_events", self.schema)
    }

    async fn run_migrations(&self) -> Result<(), SqlxError> {
        query(&format!("CREATE SCHEMA IF NOT EXISTS {}", self.schema))
            .execute(&self.pool)
            .await?;
        query(&format!(
            "CREATE TABLE IF NOT EXISTS {} (
                id BIGSERIAL PRIMARY KEY,
                event_type TEXT NOT NULL,
                payload_json JSONB NOT NULL,
                created_at TIMESTAMPTZ NOT NULL,
                previous_hash TEXT,
                current_hash TEXT NOT NULL
            )",
            self.table()
        ))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn append(&self, event: AuditEvent) -> Result<(), SqlxError> {
        let previous_hash = query(&format!(
            "SELECT current_hash FROM {} ORDER BY id DESC LIMIT 1",
            self.table()
        ))
        .fetch_optional(&self.pool)
        .await?
        .map(|row| row.get::<String, _>("current_hash"));

        let payload_json =
            serde_json::to_string(&event.payload).unwrap_or_else(|_| "{}".to_string());
        let current_hash = compute_hash(
            previous_hash.as_deref(),
            &event.event_type,
            &payload_json,
            &event.created_at.to_rfc3339(),
        );

        query(&format!(
            "INSERT INTO {} (event_type, payload_json, created_at, previous_hash, current_hash)
             VALUES ($1, $2, $3, $4, $5)",
            self.table()
        ))
        .bind(&event.event_type)
        .bind(Json(event.payload))
        .bind(event.created_at)
        .bind(previous_hash)
        .bind(current_hash)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn list_recent(&self, limit: i64) -> Result<Vec<AuditEventRecord>, SqlxError> {
        let limit = limit.clamp(1, 500);
        let rows = query(&format!(
            "SELECT id, event_type, payload_json, created_at, previous_hash, current_hash
             FROM {}
             ORDER BY id DESC
             LIMIT $1",
            self.table()
        ))
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                Ok(AuditEventRecord {
                    id: row.get("id"),
                    event_type: row.get("event_type"),
                    payload: row.get::<Json<serde_json::Value>, _>("payload_json").0,
                    created_at: row.get("created_at"),
                    previous_hash: row.get("previous_hash"),
                    current_hash: row.get("current_hash"),
                })
            })
            .collect()
    }
}

fn validate_schema_name(schema: &str) -> Result<String, SqlxError> {
    if schema.is_empty()
        || !schema
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_')
    {
        return Err(SqlxError::Protocol(format!(
            "invalid postgres schema `{schema}`; use lowercase letters, digits, or underscores"
        )));
    }

    Ok(schema.to_string())
}

fn compute_hash(
    previous_hash: Option<&str>,
    event_type: &str,
    payload_json: &str,
    created_at: &str,
) -> String {
    let mut hasher = Sha256::new();
    if let Some(previous_hash) = previous_hash {
        hasher.update(previous_hash.as_bytes());
    }
    hasher.update(event_type.as_bytes());
    hasher.update(payload_json.as_bytes());
    hasher.update(created_at.as_bytes());
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::compute_hash;

    #[test]
    fn hash_chain_changes_when_previous_hash_changes() {
        let first = compute_hash(None, "EVENT", r#"{"value":1}"#, "2026-03-24T00:00:00Z");
        let second = compute_hash(
            Some(&first),
            "EVENT",
            r#"{"value":1}"#,
            "2026-03-24T00:00:00Z",
        );

        assert_ne!(first, second);
    }
}
