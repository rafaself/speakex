CREATE TABLE IF NOT EXISTS transcriptions (
  id TEXT PRIMARY KEY,
  text TEXT NOT NULL,
  provider TEXT NOT NULL,
  model TEXT,
  language TEXT,
  duration_ms INTEGER,
  audio_path TEXT,
  audio_deleted INTEGER NOT NULL DEFAULT 1,
  copied_to_clipboard INTEGER NOT NULL DEFAULT 0,
  error TEXT,
  created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_transcriptions_created_at
ON transcriptions(created_at DESC);

CREATE INDEX IF NOT EXISTS idx_transcriptions_provider
ON transcriptions(provider);
