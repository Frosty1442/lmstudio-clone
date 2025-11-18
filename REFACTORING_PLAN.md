# Refactoring Plan: Making the Project More Cohesive and Concise

## 🎯 Goals
- Reduce code duplication
- Simplify architecture
- Improve consistency
- Make testing easier
- Reduce cognitive load

## 📊 Current Issues

### 1. **Manager Pattern Repetition**
Every manager (WorkspaceManager, DocumentManager, ChatSessionManager) repeats:
- `new(pool: SqlitePool)` constructor
- Similar CRUD patterns
- Similar error handling
- Similar test setup

### 2. **Large AppState**
```rust
pub struct AppState {
    pub model_manager: Arc<ModelManager>,
    pub workspace_manager: Arc<WorkspaceManager>,
    pub chat_session_manager: Arc<ChatSessionManager>,
    pub document_manager: Arc<DocumentManager>,
    pub rag_engine: Arc<RAGEngine>,
}
```
- 5 separate Arc<Manager> fields
- Getting unwieldy
- Hard to test

### 3. **Repetitive API Error Handling**
Every endpoint does:
```rust
.await
.map_err(|e| AppError::InvalidRequest(e.to_string()))?
```

### 4. **Test Boilerplate**
Each module recreates similar test helpers:
- Database creation
- Temporary directories
- Manager initialization

### 5. **Type Scattering**
- Some types in `types.rs`
- Some types in module files
- Request/Response types mixed with domain types

## 🔧 Refactoring Plan

### Phase 1: Consolidate Managers (Priority: HIGH)

**Problem:** 3+ managers with similar patterns, all holding `SqlitePool`

**Solution:** Create unified `DatabaseService` or use Repository pattern

```rust
// New: lms-server/src/repository.rs
pub struct Repository {
    pool: SqlitePool,
}

impl Repository {
    // Workspace operations
    pub async fn create_workspace(&self, req: CreateWorkspaceRequest) -> Result<Workspace>;
    pub async fn get_workspace(&self, id: &str) -> Result<Option<Workspace>>;
    pub async fn list_workspaces(&self) -> Result<Vec<Workspace>>;

    // Document operations
    pub async fn create_document(&self, doc: Document) -> Result<Document>;
    pub async fn get_document(&self, id: &str) -> Result<Option<Document>>;

    // Chat session operations
    pub async fn create_session(&self, req: CreateSessionRequest) -> Result<ChatSession>;
    pub async fn add_message(&self, msg: AddMessageRequest) -> Result<Message>;
}
```

**Benefits:**
- Single source of truth for database operations
- Easier to test (one mock point)
- Reduced code duplication
- Clearer separation: Repository (DB) vs Services (business logic)

**Impact:**
- Delete: `workspace.rs`, `chat_sessions.rs` manager code
- Consolidate into: `repository.rs`
- Keep domain types, move CRUD to repository

### Phase 2: Simplify AppState (Priority: HIGH)

**Problem:** Too many Arc wrappers, hard to initialize

**Solution:** Group by responsibility

```rust
// New structure
pub struct AppState {
    pub services: Arc<Services>,
    pub config: Arc<Config>,
}

pub struct Services {
    pub repository: Repository,
    pub inference: InferenceEngine,
    pub rag: RAGEngine,
    pub documents: DocumentProcessor,
}

pub struct Config {
    pub models_dir: PathBuf,
    pub storage_dir: PathBuf,
    pub vector_store_url: String,
}
```

**Benefits:**
- Clearer grouping
- Single Arc for all services
- Easier to pass around
- Config separated from services

**Impact:**
- Refactor `main.rs` initialization
- Update all API handlers (1 line change each)
- Update test helpers

### Phase 3: Extract Common API Patterns (Priority: MEDIUM)

**Problem:** Repetitive error handling in every endpoint

**Solution:** Custom extractor or macro

```rust
// New: lms-server/src/api/extractors.rs
use axum::extract::FromRequest;

// Custom extractor that handles common error mapping
pub struct DbResult<T>(pub T);

#[async_trait]
impl<S, T> FromRequest<S> for DbResult<T>
where
    T: Send,
{
    type Rejection = AppError;
    // ... auto-convert database errors to AppError
}

// Usage in endpoints becomes:
pub async fn create_workspace(
    State(state): State<AppState>,
    Json(req): Json<CreateWorkspaceRequest>,
) -> Result<Json<Workspace>, AppError> {
    let workspace = state.services.repository
        .create_workspace(req)
        .await?; // Cleaner!

    Ok(Json(workspace))
}
```

**Benefits:**
- Less boilerplate
- Consistent error handling
- Easier to add logging/tracing

**Impact:**
- Create `api/extractors.rs`
- Simplify all endpoints (remove .map_err)

### Phase 4: Unified Test Helpers (Priority: MEDIUM)

**Problem:** Every module creates its own test database/manager

**Solution:** Shared test utilities

```rust
// New: lms-server/src/test_utils.rs
#[cfg(test)]
pub mod test_utils {
    use super::*;

    pub struct TestContext {
        pub repository: Repository,
        pub workspace_id: String,
        pub temp_dir: TempDir,
    }

    impl TestContext {
        pub async fn new() -> Self {
            let temp_dir = TempDir::new().unwrap();
            let db_path = temp_dir.path().join("test.db");
            let db = Database::new(&db_path).await.unwrap();
            let repository = Repository::new(db.pool().clone());

            // Create default workspace
            let workspace = repository.create_workspace(/* ... */).await.unwrap();

            Self {
                repository,
                workspace_id: workspace.id,
                temp_dir,
            }
        }
    }
}

// Usage:
#[tokio::test]
async fn test_something() {
    let ctx = TestContext::new().await;
    // Use ctx.repository, ctx.workspace_id
}
```

**Benefits:**
- DRY tests
- Consistent test setup
- Easy to extend

**Impact:**
- Create `test_utils.rs`
- Refactor all test modules to use it
- Delete duplicate test helpers

### Phase 5: Organize Types (Priority: MEDIUM)

**Problem:** Types scattered across files

**Solution:** Clear type organization

```rust
// lms-server/src/types/
// ├── mod.rs           - Re-exports
// ├── api.rs           - API request/response types
// ├── domain.rs        - Core domain types (Workspace, Document, etc.)
// ├── openai.rs        - OpenAI compatibility types
// └── error.rs         - Error types

// Clear separation:
// - API types: CreateWorkspaceRequest, WorkspaceResponse
// - Domain types: Workspace, Document, Message
// - OpenAI types: ChatCompletionRequest, etc.
```

**Benefits:**
- Clear boundaries
- Easy to find types
- Better documentation

**Impact:**
- Reorganize `types.rs` into folder
- Update imports across project

### Phase 6: Simplify Document Processing (Priority: LOW)

**Problem:** DocumentManager + TextChunker + separate documents.rs

**Solution:** Consolidate into single service

```rust
// Merge into: lms-server/src/services/documents.rs
pub struct DocumentService {
    repository: Repository,
    vector_store: Arc<VectorStore>,
    inference: Arc<InferenceEngine>,
    chunker: TextChunker,
    storage_path: PathBuf,
}

impl DocumentService {
    // All document operations in one place
    pub async fn upload(&self, ...) -> Result<Document>;
    pub async fn process(&self, ...) -> Result<()>;

    // Private helpers
    fn chunk_text(&self, text: &str) -> Result<Vec<Chunk>>;
    fn detect_file_type(&self, filename: &str) -> String;
}
```

**Benefits:**
- Single responsibility
- All document logic together
- Clearer API surface

**Impact:**
- Merge `document_manager.rs` + `documents.rs`
- Move to `services/documents.rs`

### Phase 7: Extract RAG Pipeline (Priority: LOW)

**Problem:** RAG logic spread across modules

**Solution:** Clear pipeline abstraction

```rust
// lms-server/src/services/rag.rs
pub struct RAGPipeline {
    stages: Vec<Box<dyn PipelineStage>>,
}

trait PipelineStage {
    async fn process(&self, context: &mut RAGContext) -> Result<()>;
}

// Stages:
// 1. QueryEmbedding - embed user query
// 2. VectorRetrieval - search vector store
// 3. ContextBuilding - build prompt with citations
// 4. Generation - call LLM
// 5. CitationTracking - extract citations

// Benefits: testable stages, composable, clear flow
```

**Benefits:**
- Testable stages
- Clear data flow
- Easy to add stages (reranking, etc.)

**Impact:**
- Refactor `rag.rs` to use pipeline
- Better testing (test each stage)

## 📋 Implementation Order

### Sprint 1: Core Consolidation (HIGH impact, 4-6 hours)
1. ✅ Create `repository.rs` - consolidate all database operations
2. ✅ Simplify `AppState` - group services
3. ✅ Update `main.rs` initialization
4. ✅ Update all API endpoints to use new structure
5. ✅ Run tests, fix breakages

### Sprint 2: Developer Experience (MEDIUM impact, 3-4 hours)
6. ✅ Create `test_utils.rs` - unified test helpers
7. ✅ Refactor all tests to use new helpers
8. ✅ Organize `types/` folder structure
9. ✅ Update imports
10. ✅ Run tests

### Sprint 3: API Improvements (LOW impact, 2-3 hours)
11. ✅ Create API extractors for common patterns
12. ✅ Simplify endpoint error handling
13. ✅ Add API response helpers

### Sprint 4: Service Refinement (OPTIONAL, 3-4 hours)
14. ⚠️ Merge document processing into single service
15. ⚠️ Refactor RAG to pipeline pattern
16. ⚠️ Add middleware for logging/tracing

## 🎯 Expected Outcomes

### Before Refactoring
```
Lines of Code: ~3,500
Files: 11 modules
Managers: 3 separate
AppState fields: 5
Test helpers: 6 duplicated
```

### After Refactoring
```
Lines of Code: ~2,800 (-20%)
Files: 8 modules
Repository: 1 unified
AppState fields: 2
Test helpers: 1 shared
```

### Quality Improvements
- ✅ Reduced duplication by ~20%
- ✅ Clearer separation of concerns
- ✅ Easier to test (single mock point)
- ✅ Better code organization
- ✅ Simpler API surface
- ✅ Faster onboarding for new developers

## ⚠️ Risks & Mitigation

### Risk 1: Breaking Changes
- **Mitigation:** Do refactoring in phases, run tests after each
- **Rollback:** Each sprint is a commit, easy to revert

### Risk 2: Test Failures
- **Mitigation:** Keep existing tests, update incrementally
- **Rollback:** Git bisect to find breaking change

### Risk 3: Regression Bugs
- **Mitigation:** Run full test suite after each change
- **Rollback:** Comprehensive test coverage (54 tests)

## 📝 Notes

- Keep all 54 tests passing throughout
- Each sprint should be a separate commit
- Document changes in CHANGELOG.md
- Update IMPLEMENTATION_STATUS.md after completion
