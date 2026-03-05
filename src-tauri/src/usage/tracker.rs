use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};

#[derive(Debug, Serialize, Deserialize)]
pub struct UsageRecord {
    pub timestamp: DateTime<Utc>,
    pub requested_model: String,
    pub mapped_model: String,
    pub model: String,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub total_tokens: i64,
    pub status_code: i64,
    pub latency_ms: i64,
    pub path: String,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LogEntry {
    pub id: i64,
    pub timestamp: String,
    pub method: String,
    pub path: String,
    pub requested_model: String,
    pub mapped_model: String,
    pub model: String,
    pub status_code: i64,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub latency_ms: i64,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UsageStats {
    pub date: String,
    pub request_count: i64,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub total_tokens: i64,
    pub model: String,
}

pub async fn run_migrations(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS app_config (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS request_logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp TEXT NOT NULL DEFAULT (datetime('now')),
            method TEXT NOT NULL DEFAULT 'POST',
            path TEXT NOT NULL,
            requested_model TEXT NOT NULL DEFAULT '',
            mapped_model TEXT NOT NULL DEFAULT '',
            model TEXT NOT NULL DEFAULT '',
            status_code INTEGER NOT NULL DEFAULT 200,
            prompt_tokens INTEGER NOT NULL DEFAULT 0,
            completion_tokens INTEGER NOT NULL DEFAULT 0,
            latency_ms INTEGER NOT NULL DEFAULT 0,
            error TEXT
        )",
    )
    .execute(pool)
    .await?;

    add_column_if_missing(
        pool,
        "ALTER TABLE request_logs ADD COLUMN requested_model TEXT NOT NULL DEFAULT ''",
    )
    .await?;

    add_column_if_missing(
        pool,
        "ALTER TABLE request_logs ADD COLUMN mapped_model TEXT NOT NULL DEFAULT ''",
    )
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_logs_timestamp ON request_logs(timestamp)")
        .execute(pool)
        .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_logs_model ON request_logs(model)")
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn insert_log(pool: &SqlitePool, record: &UsageRecord) -> Result<()> {
    let timestamp = record.timestamp.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();

    sqlx::query(
        "INSERT INTO request_logs
            (timestamp, method, path, requested_model, mapped_model, model, status_code, prompt_tokens, completion_tokens, latency_ms, error)
         VALUES (?, 'POST', ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&timestamp)
    .bind(&record.path)
    .bind(&record.requested_model)
    .bind(&record.mapped_model)
    .bind(&record.model)
    .bind(record.status_code)
    .bind(record.prompt_tokens)
    .bind(record.completion_tokens)
    .bind(record.latency_ms)
    .bind(&record.error)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_recent_logs(pool: &SqlitePool, limit: i64) -> Result<Vec<LogEntry>> {
    let rows = sqlx::query(
    "SELECT id, timestamp, method, path, requested_model, mapped_model, model, status_code,
                prompt_tokens, completion_tokens, latency_ms, error
         FROM request_logs
         ORDER BY id DESC
         LIMIT ?",
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;

    let result = rows
        .into_iter()
        .map(|row| LogEntry {
            id: row.get("id"),
            timestamp: row.get("timestamp"),
            method: row.get("method"),
            path: row.get("path"),
            requested_model: row.get::<Option<String>, _>("requested_model").unwrap_or_default(),
            mapped_model: row.get::<Option<String>, _>("mapped_model").unwrap_or_default(),
            model: row.get("model"),
            status_code: row.get("status_code"),
            prompt_tokens: row.get("prompt_tokens"),
            completion_tokens: row.get("completion_tokens"),
            latency_ms: row.get("latency_ms"),
            error: row.get("error"),
        })
        .collect();

    Ok(result)
}

pub async fn get_usage_stats(pool: &SqlitePool, days: i64) -> Result<Vec<UsageStats>> {
    let days_str = format!("-{} days", days);
    let rows = sqlx::query(
        "SELECT
            date(timestamp) as date,
            COUNT(*) as request_count,
            SUM(prompt_tokens) as prompt_tokens,
            SUM(completion_tokens) as completion_tokens,
            SUM(prompt_tokens + completion_tokens) as total_tokens,
            model
         FROM request_logs
         WHERE timestamp >= datetime('now', ?)
           AND status_code >= 200 AND status_code < 300
         GROUP BY date(timestamp), model
         ORDER BY date(timestamp) ASC",
    )
    .bind(&days_str)
    .fetch_all(pool)
    .await?;

    let result = rows
        .into_iter()
        .map(|row| UsageStats {
            date: row.get("date"),
            request_count: row.get("request_count"),
            prompt_tokens: row.get::<Option<i64>, _>("prompt_tokens").unwrap_or(0),
            completion_tokens: row.get::<Option<i64>, _>("completion_tokens").unwrap_or(0),
            total_tokens: row.get::<Option<i64>, _>("total_tokens").unwrap_or(0),
            model: row.get("model"),
        })
        .collect();

    Ok(result)
}

pub async fn get_today_request_count(pool: &SqlitePool) -> Result<i64> {
    let row = sqlx::query(
        "SELECT COUNT(*) as count FROM request_logs WHERE date(timestamp) = date('now')",
    )
    .fetch_one(pool)
    .await?;

    Ok(row.get("count"))
}

pub async fn get_total_request_count(pool: &SqlitePool) -> Result<i64> {
    let row = sqlx::query("SELECT COUNT(*) as count FROM request_logs")
        .fetch_one(pool)
        .await?;

    Ok(row.get("count"))
}

pub async fn clear_logs(pool: &SqlitePool) -> Result<()> {
    sqlx::query("DELETE FROM request_logs")
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn load_config_from_db(pool: &SqlitePool) -> Result<Option<String>> {
    let row = sqlx::query("SELECT value FROM app_config WHERE key = 'config'")
        .fetch_optional(pool)
        .await?;

    Ok(row.map(|r| r.get("value")))
}

pub async fn save_config_to_db_raw(pool: &SqlitePool, value: &str) -> Result<()> {
    sqlx::query("INSERT OR REPLACE INTO app_config (key, value) VALUES ('config', ?)")
        .bind(value)
        .execute(pool)
        .await?;
    Ok(())
}

async fn add_column_if_missing(pool: &SqlitePool, sql: &str) -> Result<()> {
    match sqlx::query(sql).execute(pool).await {
        Ok(_) => Ok(()),
        Err(sqlx::Error::Database(db_err)) if db_err.message().contains("duplicate column name") => {
            Ok(())
        }
        Err(e) => Err(e.into()),
    }
}
