-- Add vector embeddings table for SQLite-based vector storage
-- Stores embeddings as binary blobs with metadata for filtering

CREATE TABLE IF NOT EXISTS vector_embeddings (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL,
    document_id TEXT NOT NULL,
    chunk_index INTEGER NOT NULL,
    embedding BLOB NOT NULL,  -- Binary serialized vector (f32 array)
    created_at TEXT NOT NULL,
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE,
    FOREIGN KEY (document_id) REFERENCES documents(id) ON DELETE CASCADE
);

-- Indexes for efficient filtering
CREATE INDEX IF NOT EXISTS idx_vector_workspace ON vector_embeddings(workspace_id);
CREATE INDEX IF NOT EXISTS idx_vector_document ON vector_embeddings(document_id);
CREATE INDEX IF NOT EXISTS idx_vector_workspace_document ON vector_embeddings(workspace_id, document_id);

-- Note: Vector search will be done in-memory with cosine similarity
-- This is fast enough for local datasets (<100k vectors)
-- Metadata filtering uses SQL WHERE clauses before similarity calculation
