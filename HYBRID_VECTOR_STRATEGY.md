# Hybrid Vector Storage Strategy

## Architecture: Pluggable Vector Backend

Instead of choosing SQLite OR Lance, design for easy swapping:

```rust
// lms-server/src/vector_store.rs

#[async_trait]
pub trait VectorBackend: Send + Sync {
    async fn insert(&self, vectors: Vec<VectorData>) -> Result<Vec<String>>;
    async fn search(&self, query: Vec<f32>, limit: usize, filters: Option<SearchFilters>) -> Result<Vec<SearchResult>>;
    async fn delete(&self, ids: Vec<String>) -> Result<()>;
}

// SQLite implementation (default)
pub struct SqliteVectorBackend {
    pool: SqlitePool,
}

#[async_trait]
impl VectorBackend for SqliteVectorBackend {
    // ... SQLite implementation
}

// Lance implementation (optional)
#[cfg(feature = "lance")]
pub struct LanceVectorBackend {
    db: Arc<lance::DB>,
}

#[cfg(feature = "lance")]
#[async_trait]
impl VectorBackend for LanceVectorBackend {
    // ... Lance implementation
}

// Factory
pub enum VectorBackendType {
    Sqlite,
    #[cfg(feature = "lance")]
    Lance,
}

pub fn create_vector_backend(
    backend_type: VectorBackendType,
    config: VectorConfig,
) -> Box<dyn VectorBackend> {
    match backend_type {
        VectorBackendType::Sqlite => Box::new(SqliteVectorBackend::new(config.pool)),
        #[cfg(feature = "lance")]
        VectorBackendType::Lance => Box::new(LanceVectorBackend::new(config.path)),
    }
}
```

## Usage

```rust
// lms-server/src/main.rs

// Default to SQLite
let vector_backend = create_vector_backend(
    VectorBackendType::Sqlite,
    VectorConfig { pool: db.pool() }
);

// Or use Lance (opt-in via feature flag)
// cargo build --features lance
#[cfg(feature = "lance")]
let vector_backend = create_vector_backend(
    VectorBackendType::Lance,
    VectorConfig { path: lance_path }
);

let vector_store = VectorStore::new(vector_backend);
```

## Benefits

1. **Start simple** - SQLite by default
2. **Future-proof** - Lance backend available when needed
3. **Easy migration** - swap backend without changing RAG code
4. **Testing** - can test both backends
5. **User choice** - power users can enable Lance

## Migration Tool

```rust
// tools/migrate_vectors.rs

async fn migrate_sqlite_to_lance(
    sqlite_pool: SqlitePool,
    lance_path: PathBuf,
) -> Result<()> {
    let sqlite_backend = SqliteVectorBackend::new(sqlite_pool);
    let lance_backend = LanceVectorBackend::new(lance_path);

    // Stream all vectors from SQLite
    let vectors = sqlite_backend.export_all().await?;

    // Batch insert to Lance
    for batch in vectors.chunks(1000) {
        lance_backend.insert(batch.to_vec()).await?;
    }

    println!("Migrated {} vectors", vectors.len());
    Ok(())
}
```

## Configuration

```toml
# config.toml

[vector_storage]
backend = "sqlite"  # or "lance"

[vector_storage.sqlite]
# Uses main database pool

[vector_storage.lance]
path = "~/.lmstudio-clone/lancedb"
cache_size_mb = 256
```

## Feature Flags

```toml
# Cargo.toml

[features]
default = ["sqlite-backend"]
sqlite-backend = []
lance-backend = ["lancedb", "arrow"]
all-backends = ["sqlite-backend", "lance-backend"]
```

## Decision Tree

```
Start with: SQLite
│
├─ Dataset <100k vectors → Keep SQLite ✅
│
├─ Dataset 100k-500k vectors → Evaluate performance
│  ├─ Search <200ms → Keep SQLite ✅
│  └─ Search >200ms → Consider Lance migration
│
└─ Dataset >500k vectors → Migrate to Lance
```

## Timeline

- **Week 1:** Implement SQLite backend (current plan)
- **Week 2-4:** Use in production, gather metrics
- **Month 2:** If needed, add Lance backend via feature flag
- **Month 3:** Provide migration tool for early users

## Best of Both Worlds

- Start simple (SQLite)
- Designed for growth (pluggable backend)
- Easy migration (trait-based abstraction)
- User choice (feature flags)
