# Changelog

All notable changes to the LMStudio Clone project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### [2024-11-25] - PDF and DOCX Parsing Implementation

#### ✨ Added
- **PDF Parsing Support**
  - Full text extraction using `lopdf` (pure Rust implementation)
  - Page count detection and storage in database
  - Metadata extraction (title, author, creation date)
  - Per-page text extraction with robust error handling
  - Proper UTF-8 conversion for all metadata fields

- **DOCX Parsing Support**
  - Complete text extraction using `docx-rs`
  - Paragraph-by-paragraph processing
  - Handles runs and text elements properly
  - Clean output with newline preservation

- **Document Processing Pipeline**
  - Created `DocumentParser` with pluggable format support
  - New `ParsedDocument` type containing text, page_count, and metadata
  - New `DocumentMetadata` type for extracted metadata
  - Automatic document type detection from filename extensions

#### 🔄 Changed
- **DocumentManager Updates**
  - `process_document` now parses before chunking
  - Returns `(chunk_count, page_count)` tuple instead of just count
  - Stores page_count in database after successful processing
  - Parse → Chunk → Embed → Store pipeline

- **DocumentType Enum Expansion**
  - Added `Pdf` variant
  - Added `Docx` variant
  - Updated `from_filename()` to detect PDF/DOCX
  - Updated `as_str()` to return correct extensions

#### 📦 Dependencies
- **Added**: `lopdf = "0.32"` (pure Rust PDF parser, no system deps)
- **Added**: `docx-rs = "0.4"` (DOCX parsing)
- **Added**: `encoding_rs = "0.8"` (text encoding detection)

#### ✅ Testing
- All 47 tests passing
- No regression in existing functionality
- Proper error handling throughout parsing pipeline

#### 🎯 Impact
Now supports real-world document formats! The system can process:
- PDF files with page count tracking
- DOCX files with proper text extraction
- Markdown and plain text (as before)

This completes the most critical missing feature.

---

### [2024-11-09] - Major Architecture Refactoring

#### 🎨 Refactored
- **Repository Pattern Implementation**
  - Consolidated `WorkspaceManager` and `ChatSessionManager` into unified `Repository`
  - Removed 1,470 lines of duplicate code
  - Single source of truth for all database operations
  - Added `Clone` trait to Repository for easy sharing across components

- **Simplified AppState Architecture**
  - Reduced from 5 separate `Arc<Manager>` fields to single `Arc<Services>` wrapper
  - Grouped all services (repository, model_manager, document_manager, rag_engine) into `Services` struct
  - Cleaner API surface and easier dependency injection
  - Better code organization and maintainability

#### ✨ Added
- **SQLite Vector Storage** (replacing Qdrant)
  - Embedded vector storage using SQLite BLOB columns
  - Binary serialization with `bincode` for efficient storage (f32 vectors → bytes)
  - In-memory cosine similarity search implemented in Rust
  - **No external services required** - true local-first architecture
  - New migration: `20241106000001_add_vector_embeddings.sql`
  - New `vector_embeddings` table with workspace/document foreign keys
  - Sufficient performance for <100k vectors (typical local use case)

#### 🔄 Changed
- **Component Updates**
  - `RAGEngine`: Now uses `Repository` instead of `WorkspaceManager`
  - `DocumentManager`: Uses `Repository` for document database operations
  - All API endpoints: Updated to use new `AppState.services` structure
  - All tests: Fixed to use new architecture patterns

#### 🗑️ Removed
- Deleted `lms-server/src/workspace.rs` (functionality moved to `repository.rs`)
- Deleted `lms-server/src/chat_sessions.rs` (functionality moved to `repository.rs`)
- Removed `qdrant-client` dependency from `Cargo.toml`
- Removed Docker requirement for vector storage

#### 📦 Dependencies
- **Added**: `bincode = "1.3"` for efficient binary serialization
- **Removed**: `qdrant-client` (no longer needed)

#### ✅ Tests
- All 47 tests passing
- Updated test helpers to use new `Repository` and `AppState` patterns
- Breakdown:
  - Database tests: 3/3
  - Repository tests: 3/3 (replaces previous workspace + chat session tests)
  - Document tests: 9/9
  - Document manager tests: 2/2
  - Model/API tests: 23/23
  - Vector store tests: 3/3
  - RAG engine tests: 3/3
  - Types tests: 1/1

#### 📈 Performance
- **Code Reduction**: Removed 1,470 lines through consolidation
- **Simplified Dependencies**: One less external service to run
- **Startup Time**: No waiting for Qdrant to initialize
- **Memory Efficiency**: Vectors stored directly in SQLite

#### 🎯 Impact
- **Development**: Faster iteration (no Docker setup required)
- **Testing**: Simpler test environment (single database file)
- **Deployment**: Single binary + SQLite file (no container orchestration)
- **Privacy**: Everything local, no network calls for vector operations

---

## Previous Work (Before Changelog)

### [2024-11-05] - RAG System Complete
- Implemented complete RAG (Retrieval Augmented Generation) pipeline
- Chat session management with message history
- Document upload and processing
- Text chunking with semantic boundaries
- Embedding generation via llama-server
- Citation tracking with sources and scores
- Workspace management with full CRUD API
- 54 tests passing (before refactoring)
