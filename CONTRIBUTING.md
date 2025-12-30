# Contributing to LMStudio Clone

Thank you for your interest in contributing to LMStudio Clone! This document provides guidelines and instructions for contributing.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Making Changes](#making-changes)
- [Pull Request Process](#pull-request-process)
- [Coding Standards](#coding-standards)
- [Testing](#testing)
- [Documentation](#documentation)

## Code of Conduct

Please be respectful and constructive in all interactions. We welcome contributors of all experience levels.

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/YOUR_USERNAME/lmstudio-clone.git`
3. Add upstream remote: `git remote add upstream https://github.com/Frosty1442/lmstudio-clone.git`
4. Create a feature branch: `git checkout -b feature/your-feature-name`

## Development Setup

### Prerequisites

- **Rust** (1.70+): Install via [rustup](https://rustup.rs/)
- **Node.js** (18+): For the UI
- **SQLite**: Usually pre-installed on most systems

### Backend (lms-server)

```bash
cd lms-server

# Build
cargo build

# Run tests
cargo test

# Run with logging
RUST_LOG=info cargo run

# Format code
cargo fmt

# Lint
cargo clippy
```

### Frontend (lms-ui)

```bash
cd lms-ui

# Install dependencies
npm install

# Development server
npm run dev

# Build
npm run build

# Type check
npx tsc --noEmit
```

### Full Stack Development

```bash
# Terminal 1: Start the server
cd lms-server && cargo run

# Terminal 2: Start the UI dev server
cd lms-ui && npm run dev
```

## Making Changes

### Branch Naming

- `feature/` - New features
- `fix/` - Bug fixes
- `docs/` - Documentation updates
- `refactor/` - Code refactoring
- `test/` - Test additions/changes

Examples:
- `feature/add-model-quantization`
- `fix/chat-streaming-issue`
- `docs/api-reference`

### Commit Messages

Use clear, descriptive commit messages:

```
<type>: <short description>

<optional body with more details>
```

Types:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation
- `refactor`: Code refactoring
- `test`: Tests
- `chore`: Maintenance tasks

Examples:
```
feat: Add PDF document parsing with page tracking

- Implement lopdf-based PDF text extraction
- Track page boundaries for accurate citations
- Add metadata extraction (title, author)
```

## Pull Request Process

1. **Update your branch** with the latest upstream changes:
   ```bash
   git fetch upstream
   git rebase upstream/main
   ```

2. **Run all checks** before submitting:
   ```bash
   # Backend
   cargo test
   cargo fmt --check
   cargo clippy

   # Frontend
   cd lms-ui && npm run build
   ```

3. **Create the pull request** with:
   - Clear title describing the change
   - Description of what and why
   - Reference to related issues (if any)
   - Screenshots for UI changes

4. **Address review feedback** promptly

5. **Squash commits** if requested

### PR Checklist

- [ ] Tests pass locally
- [ ] Code follows project style guidelines
- [ ] Documentation updated (if applicable)
- [ ] No new warnings from `cargo clippy`
- [ ] Commit messages follow conventions

## Coding Standards

### Rust

- Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `cargo fmt` for formatting
- Address all `cargo clippy` warnings
- Document public APIs with doc comments
- Use meaningful variable and function names
- Prefer `Result<T, E>` over panics
- Use `anyhow` for error handling in applications
- Use `thiserror` for library error types

```rust
/// Generates embeddings for the given texts.
///
/// # Arguments
/// * `model_id` - The ID of the embedding model to use
/// * `texts` - List of texts to embed
///
/// # Returns
/// A vector of embedding vectors (one per input text)
pub async fn generate_embeddings(
    &self,
    model_id: &str,
    texts: Vec<String>,
) -> Result<Vec<Vec<f32>>> {
    // Implementation
}
```

### TypeScript/React

- Use TypeScript strict mode
- Prefer functional components with hooks
- Use meaningful component and variable names
- Keep components focused and small
- Use proper TypeScript types (avoid `any`)

```typescript
interface ChatMessage {
  id: string;
  role: 'user' | 'assistant';
  content: string;
  timestamp: Date;
}

const Chat: React.FC<ChatProps> = ({ messages, onSend }) => {
  // Component implementation
};
```

## Testing

### Backend Tests

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run with output
cargo test -- --nocapture

# Run ignored tests
cargo test -- --ignored
```

### Writing Tests

- Place unit tests in the same file as the code
- Use descriptive test names
- Test both success and error cases
- Use `#[tokio::test]` for async tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_workspace_creation() {
        let repo = create_test_repository().await;
        let workspace = repo.create_workspace(request).await.unwrap();
        assert_eq!(workspace.name, "Test Workspace");
    }

    #[test]
    fn test_chunk_text_empty() {
        let chunker = TextChunker::with_defaults();
        let chunks = chunker.chunk_text("").unwrap();
        assert!(chunks.is_empty());
    }
}
```

## Documentation

- Update README.md for user-facing changes
- Update ARCHITECTURE.md for structural changes
- Add doc comments to public APIs
- Include examples in documentation

### Doc Comments

```rust
/// A document chunk with metadata for RAG retrieval.
///
/// # Example
///
/// ```rust
/// let chunk = TextChunk {
///     text: "Example text...".to_string(),
///     start_pos: 0,
///     end_pos: 15,
///     index: 0,
/// };
/// ```
pub struct TextChunk {
    /// The chunk text content
    pub text: String,
    /// Start position in the original document
    pub start_pos: usize,
    /// End position in the original document
    pub end_pos: usize,
    /// Sequential chunk index
    pub index: usize,
}
```

## Questions?

- Open an issue for bugs or feature requests
- Start a discussion for questions
- Check existing issues before creating new ones

Thank you for contributing!
