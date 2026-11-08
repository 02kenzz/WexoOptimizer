//! SQLite-backed transaction log for optimization history and rollback.

use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use parking_lot::Mutex;
use rusqlite::{params, Connection, OptionalExtension, Result as SqlResult};
use serde::{Deserialize, Serialize};
use serde_json;
use thiserror::Error;
use tracing::{debug, info};

use crate::snapshot::Snapshot;

/// Errors for transaction log operations.
#[derive(Debug, Error)]
pub enum TransactionError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Snapshot error: {0}")]
    Snapshot(#[from] crate::snapshot::SnapshotError),
    #[error("Transaction not found: {0}")]
    NotFound(String),
}

/// A transaction record stored in the log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionRecord {
    pub id: String,
    pub timestamp_ms: u64,
    pub optimization_ids: Vec<String>,
    pub status: TransactionStatus,
    pub rollback_available: bool,
    pub message: Option<String>,
    pub snapshot_id: Option<String>,
}

/// Status of a transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransactionStatus {
    Applied,
    RolledBack,
    Failed,
}

/// SQLite-backed transaction log.
pub struct TransactionLog {
    conn: Arc<Mutex<Connection>>,
    snapshot_dir: std::path::PathBuf,
}

impl TransactionLog {
    /// Opens or creates the transaction log at the given path.
    pub fn new<P: AsRef<Path>>(data_dir: P) -> Result<Self, TransactionError> {
        let data_dir = data_dir.as_ref();
        fs::create_dir_all(data_dir)?;

        let db_path = data_dir.join("transactions.db");
        let snapshot_dir = data_dir.join("snapshots");
        fs::create_dir_all(&snapshot_dir)?;

        let conn = Connection::open(&db_path)?;
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS transactions (
                id TEXT PRIMARY KEY,
                timestamp_ms INTEGER NOT NULL,
                optimization_ids TEXT NOT NULL,
                status TEXT NOT NULL,
                rollback_available INTEGER NOT NULL,
                message TEXT,
                snapshot_id TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_transactions_timestamp ON transactions(timestamp_ms DESC);
            "#,
        )?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            snapshot_dir,
        })
    }

    /// Records a new transaction.
    pub fn record(
        &self,
        optimization_ids: &[String],
        status: TransactionStatus,
        rollback_available: bool,
        message: Option<String>,
        snapshot_id: Option<String>,
    ) -> Result<TransactionRecord, TransactionError> {
        let id = format!("txn-{}", now_ms());
        let timestamp_ms = now_ms();
        let opt_ids_json = serde_json::to_string(optimization_ids)?;

        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO transactions (id, timestamp_ms, optimization_ids, status, rollback_available, message, snapshot_id) VALUES (?, ?, ?, ?, ?, ?, ?)",
            params![id, timestamp_ms, opt_ids_json, status_str(status), rollback_available as i32, message, snapshot_id],
        )?;

        let record = TransactionRecord {
            id,
            timestamp_ms,
            optimization_ids: optimization_ids.to_vec(),
            status,
            rollback_available,
            message,
            snapshot_id,
        };
        info!(txn_id = %record.id, ?status, "transaction recorded");
        Ok(record)
    }

    /// Updates a transaction's status (e.g., after rollback).
    pub fn update_status(&self, id: &str, status: TransactionStatus) -> Result<(), TransactionError> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE transactions SET status = ?, rollback_available = ? WHERE id = ?",
            params![status_str(status), (status != TransactionStatus::Applied) as i32, id],
        )?;
        debug!(txn_id = %id, ?status, "transaction status updated");
        Ok(())
    }

    /// Gets a transaction by ID.
    pub fn get(&self, id: &str) -> Result<Option<TransactionRecord>, TransactionError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare("SELECT id, timestamp_ms, optimization_ids, status, rollback_available, message, snapshot_id FROM transactions WHERE id = ?")?;
        let record = stmt
            .query_row(params![id], |row| row_to_record(row))
            .optional()?;
        Ok(record)
    }

    /// Lists all transactions, newest first.
    pub fn list(&self, limit: usize) -> Result<Vec<TransactionRecord>, TransactionError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, timestamp_ms, optimization_ids, status, rollback_available, message, snapshot_id FROM transactions ORDER BY timestamp_ms DESC LIMIT ?",
        )?;
        let rows = stmt.query_map(params![limit as i64], |row| row_to_record(row))?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// Gets the snapshot directory path.
    pub fn snapshot_dir(&self) -> &Path {
        &self.snapshot_dir
    }

    /// Captures and stores a snapshot for the given optimizations.
    pub fn capture_snapshot(
        &self,
        spec: &wexo_core::SnapshotSpec,
        optimization_ids: &[String],
    ) -> Result<Snapshot, TransactionError> {
        let snapshot = crate::snapshot::capture_snapshot(&self.snapshot_dir, spec, optimization_ids)?;
        info!(snapshot_id = %snapshot.id, "snapshot stored");
        Ok(snapshot)
    }

    /// Restores a snapshot by ID.
    pub fn restore_snapshot(&self, snapshot_id: &str) -> Result<(), TransactionError> {
        let snapshot = crate::snapshot::load_snapshot(&self.snapshot_dir, snapshot_id)?;
        crate::snapshot::restore_snapshot(&snapshot)?;
        info!(snapshot_id = %snapshot_id, "snapshot restored");
        Ok(())
    }
}

fn row_to_record(row: &rusqlite::Row) -> SqlResult<TransactionRecord> {
    let opt_ids_json: String = row.get(2)?;
    let optimization_ids = serde_json::from_str(&opt_ids_json).unwrap_or_default();
    let status_str: String = row.get(3)?;
    let status = match status_str.as_str() {
        "applied" => TransactionStatus::Applied,
        "rolled_back" => TransactionStatus::RolledBack,
        _ => TransactionStatus::Failed,
    };
    Ok(TransactionRecord {
        id: row.get(0)?,
        timestamp_ms: row.get(1)?,
        optimization_ids,
        status,
        rollback_available: row.get(4)?,
        message: row.get(5)?,
        snapshot_id: row.get(6)?,
    })
}

fn status_str(status: TransactionStatus) -> &'static str {
    match status {
        TransactionStatus::Applied => "applied",
        TransactionStatus::RolledBack => "rolled_back",
        TransactionStatus::Failed => "failed",
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn log_records_and_lists_transactions() {
        let dir = tempdir().unwrap();
        let log = TransactionLog::new(dir.path()).unwrap();

        let record = log.record(
            &["opt1".to_string(), "opt2".to_string()],
            TransactionStatus::Applied,
            true,
            Some("Test transaction".to_string()),
            Some("snap-123".to_string()),
        ).unwrap();

        assert_eq!(record.optimization_ids.len(), 2);
        assert!(record.rollback_available);

        let list = log.list(10).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, record.id);
    }

    #[test]
    fn log_updates_status_on_rollback() {
        let dir = tempdir().unwrap();
        let log = TransactionLog::new(dir.path()).unwrap();

        let record = log.record(
            &["opt1".to_string()],
            TransactionStatus::Applied,
            true,
            None,
            Some("snap-123".to_string()),
        ).unwrap();

        log.update_status(&record.id, TransactionStatus::RolledBack).unwrap();

        let updated = log.get(&record.id).unwrap().unwrap();
        assert_eq!(updated.status, TransactionStatus::RolledBack);
    }
}