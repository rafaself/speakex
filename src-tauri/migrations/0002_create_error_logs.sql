CREATE TABLE IF NOT EXISTS error_logs (
  id TEXT PRIMARY KEY,
  scope TEXT NOT NULL,
  source TEXT NOT NULL,
  summary TEXT NOT NULL,
  detail TEXT NOT NULL,
  created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_error_logs_created_at
ON error_logs(created_at DESC);

CREATE INDEX IF NOT EXISTS idx_error_logs_scope
ON error_logs(scope);