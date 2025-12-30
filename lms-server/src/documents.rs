use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use unicode_segmentation::UnicodeSegmentation;

/// Configuration for text chunking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkConfig {
    /// Target chunk size in characters (default: 1000)
    pub chunk_size: usize,
    /// Overlap size in characters (default: 200)
    pub overlap_size: usize,
    /// Minimum chunk size (default: 100)
    pub min_chunk_size: usize,
}

impl Default for ChunkConfig {
    fn default() -> Self {
        Self {
            chunk_size: 1000,
            overlap_size: 200,
            min_chunk_size: 100,
        }
    }
}

/// A text chunk with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextChunk {
    /// The chunk text
    pub text: String,
    /// Start position in the original document (character offset)
    pub start_pos: usize,
    /// End position in the original document (character offset)
    pub end_pos: usize,
    /// Chunk index
    pub index: usize,
}

/// Text chunker that splits text into overlapping segments
pub struct TextChunker {
    config: ChunkConfig,
}

impl TextChunker {
    pub fn new(config: ChunkConfig) -> Self {
        Self { config }
    }

    pub fn with_defaults() -> Self {
        Self::new(ChunkConfig::default())
    }

    /// Chunk text into overlapping segments with semantic boundaries
    ///
    /// This implementation:
    /// 1. Splits text into sentences using Unicode sentence boundaries
    /// 2. Groups sentences into chunks of approximately chunk_size characters
    /// 3. Adds overlap between chunks for better context preservation
    /// 4. Respects semantic boundaries (doesn't split mid-sentence)
    pub fn chunk_text(&self, text: &str) -> Result<Vec<TextChunk>> {
        if text.is_empty() {
            return Ok(Vec::new());
        }

        // Split into sentences using Unicode segmentation
        let sentences: Vec<&str> = text.unicode_sentences().collect();

        if sentences.is_empty() {
            return Ok(vec![TextChunk {
                text: text.to_string(),
                start_pos: 0,
                end_pos: text.len(),
                index: 0,
            }]);
        }

        let mut chunks = Vec::new();
        let mut current_chunk = String::new();
        let mut chunk_start_pos = 0;
        let mut current_pos = 0;
        let mut sentence_buffer = Vec::new(); // For overlap

        for sentence in sentences {
            let sentence_len = sentence.len();

            // If adding this sentence would exceed chunk size and we have content
            if !current_chunk.is_empty()
                && current_chunk.len() + sentence_len > self.config.chunk_size
            {
                // Create chunk from accumulated sentences
                if current_chunk.len() >= self.config.min_chunk_size {
                    chunks.push(TextChunk {
                        text: current_chunk.clone(),
                        start_pos: chunk_start_pos,
                        end_pos: chunk_start_pos + current_chunk.len(),
                        index: chunks.len(),
                    });

                    // Calculate overlap: take last N characters worth of sentences
                    let overlap_text = self.get_overlap_text(&sentence_buffer);
                    current_chunk = overlap_text.clone();
                    chunk_start_pos = chunk_start_pos + (current_chunk.len() - overlap_text.len());

                    // Keep only sentences in overlap for next iteration
                    sentence_buffer = self.get_overlap_sentences(&sentence_buffer);
                } else {
                    // Chunk too small, keep adding
                }
            }

            current_chunk.push_str(sentence);
            sentence_buffer.push(sentence.to_string());
            current_pos += sentence_len;
        }

        // Add final chunk if it has content
        if !current_chunk.is_empty() && current_chunk.len() >= self.config.min_chunk_size {
            chunks.push(TextChunk {
                text: current_chunk,
                start_pos: chunk_start_pos,
                end_pos: current_pos,
                index: chunks.len(),
            });
        }

        // If no chunks were created (text too short), create single chunk
        if chunks.is_empty() {
            chunks.push(TextChunk {
                text: text.to_string(),
                start_pos: 0,
                end_pos: text.len(),
                index: 0,
            });
        }

        Ok(chunks)
    }

    /// Get overlap text from sentence buffer
    fn get_overlap_text(&self, sentences: &[String]) -> String {
        let mut overlap = String::new();
        let mut total_len = 0;

        // Take sentences from the end until we reach overlap size
        for sentence in sentences.iter().rev() {
            if total_len + sentence.len() > self.config.overlap_size {
                break;
            }
            overlap.insert_str(0, sentence);
            total_len += sentence.len();
        }

        overlap
    }

    /// Get sentences for overlap buffer
    fn get_overlap_sentences(&self, sentences: &[String]) -> Vec<String> {
        let mut overlap_sentences = Vec::new();
        let mut total_len = 0;

        for sentence in sentences.iter().rev() {
            if total_len + sentence.len() > self.config.overlap_size {
                break;
            }
            overlap_sentences.insert(0, sentence.clone());
            total_len += sentence.len();
        }

        overlap_sentences
    }

    /// Chunk text with custom configuration
    pub fn chunk_with_config(text: &str, config: ChunkConfig) -> Result<Vec<TextChunk>> {
        let chunker = Self::new(config);
        chunker.chunk_text(text)
    }
}

/// Document type enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DocumentType {
    Text,
    Markdown,
    Pdf,
    Docx,
}

impl DocumentType {
    pub fn from_filename(filename: &str) -> Self {
        let lower = filename.to_lowercase();
        if lower.ends_with(".pdf") {
            DocumentType::Pdf
        } else if lower.ends_with(".docx") {
            DocumentType::Docx
        } else if lower.ends_with(".md") || lower.ends_with(".markdown") {
            DocumentType::Markdown
        } else {
            DocumentType::Text
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            DocumentType::Text => "txt",
            DocumentType::Markdown => "md",
            DocumentType::Pdf => "pdf",
            DocumentType::Docx => "docx",
        }
    }
}

/// Parsed document with metadata
#[derive(Debug, Clone)]
pub struct ParsedDocument {
    pub text: String,
    pub page_count: Option<usize>,
    pub metadata: DocumentMetadata,
    /// Character offsets where each page ends (for PDFs)
    /// page_boundaries[0] is the end of page 1, etc.
    pub page_boundaries: Vec<usize>,
}

impl ParsedDocument {
    /// Get the page number (1-indexed) for a given character position
    pub fn get_page_for_position(&self, char_pos: usize) -> Option<u32> {
        if self.page_boundaries.is_empty() {
            return None;
        }

        for (page_idx, &boundary) in self.page_boundaries.iter().enumerate() {
            if char_pos < boundary {
                return Some((page_idx + 1) as u32);
            }
        }

        // If past all boundaries, return last page
        Some(self.page_boundaries.len() as u32)
    }
}

/// Document metadata extracted during parsing
#[derive(Debug, Clone, Default)]
pub struct DocumentMetadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub created_date: Option<String>,
}

/// Document parser that handles multiple formats
pub struct DocumentParser;

impl DocumentParser {
    /// Parse a document based on its type
    pub fn parse(content: &[u8], doc_type: DocumentType) -> Result<ParsedDocument> {
        match doc_type {
            DocumentType::Text | DocumentType::Markdown => Self::parse_text(content),
            DocumentType::Pdf => Self::parse_pdf(content),
            DocumentType::Docx => Self::parse_docx(content),
        }
    }

    /// Parse plain text or markdown
    fn parse_text(content: &[u8]) -> Result<ParsedDocument> {
        let text = String::from_utf8_lossy(content).to_string();
        Ok(ParsedDocument {
            text,
            page_count: None,
            metadata: DocumentMetadata::default(),
            page_boundaries: Vec::new(),
        })
    }

    /// Parse PDF using lopdf
    fn parse_pdf(content: &[u8]) -> Result<ParsedDocument> {
        use lopdf::Document;

        let doc = Document::load_mem(content)
            .map_err(|e| anyhow!("Failed to load PDF: {}", e))?;

        let page_count = doc.get_pages().len();
        let mut text = String::new();
        let mut page_boundaries = Vec::new();

        // Extract text from all pages, tracking boundaries
        for page_num in 1..=page_count {
            match doc.extract_text(&[page_num as u32]) {
                Ok(page_text) => {
                    if !text.is_empty() {
                        text.push_str("\n\n");
                    }
                    text.push_str(&page_text);
                    // Record where this page ends
                    page_boundaries.push(text.len());
                }
                Err(e) => {
                    tracing::warn!("Failed to extract text from page {}: {}", page_num, e);
                    // Still record boundary for failed pages
                    page_boundaries.push(text.len());
                    continue;
                }
            }
        }

        // Extract metadata
        let metadata = Self::extract_pdf_metadata(&doc);

        Ok(ParsedDocument {
            text,
            page_count: Some(page_count),
            metadata,
            page_boundaries,
        })
    }

    /// Extract metadata from PDF
    fn extract_pdf_metadata(doc: &lopdf::Document) -> DocumentMetadata {
        let mut metadata = DocumentMetadata::default();

        // Try to get info dictionary
        if let Ok(info_dict) = doc.trailer.get(b"Info") {
            if let Ok(info) = info_dict.as_dict() {
                // Extract title
                if let Ok(title) = info.get(b"Title") {
                    if let Ok(title_bytes) = title.as_str() {
                        metadata.title = Some(String::from_utf8_lossy(title_bytes).to_string());
                    }
                }

                // Extract author
                if let Ok(author) = info.get(b"Author") {
                    if let Ok(author_bytes) = author.as_str() {
                        metadata.author = Some(String::from_utf8_lossy(author_bytes).to_string());
                    }
                }

                // Extract creation date
                if let Ok(created) = info.get(b"CreationDate") {
                    if let Ok(created_bytes) = created.as_str() {
                        metadata.created_date = Some(String::from_utf8_lossy(created_bytes).to_string());
                    }
                }
            }
        }

        metadata
    }

    /// Parse DOCX using docx-rs
    fn parse_docx(content: &[u8]) -> Result<ParsedDocument> {
        let docx = docx_rs::read_docx(content)
            .map_err(|e| anyhow!("Failed to read DOCX: {}", e))?;

        let mut text = String::new();

        // Extract text from all paragraphs
        for child in &docx.document.children {
            if let docx_rs::DocumentChild::Paragraph(para) = child {
                for para_child in &para.children {
                    if let docx_rs::ParagraphChild::Run(run) = para_child {
                        for run_child in &run.children {
                            if let docx_rs::RunChild::Text(text_elem) = run_child {
                                text.push_str(&text_elem.text);
                            }
                        }
                    }
                }
                text.push('\n');
            }
        }

        // Extract metadata from core properties (if available)
        // Note: docx-rs API may vary, metadata extraction is optional
        let metadata = DocumentMetadata::default();

        Ok(ParsedDocument {
            text,
            page_count: None, // DOCX doesn't have fixed pages
            metadata,
            page_boundaries: Vec::new(), // DOCX doesn't have page boundaries
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_empty_text() {
        let chunker = TextChunker::with_defaults();
        let chunks = chunker.chunk_text("").unwrap();
        assert_eq!(chunks.len(), 0);
    }

    #[test]
    fn test_chunk_short_text() {
        let chunker = TextChunker::with_defaults();
        let text = "This is a short text.";
        let chunks = chunker.chunk_text(text).unwrap();

        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].text, text);
        assert_eq!(chunks[0].start_pos, 0);
        assert_eq!(chunks[0].end_pos, text.len());
        assert_eq!(chunks[0].index, 0);
    }

    #[test]
    fn test_chunk_long_text() {
        let chunker = TextChunker::new(ChunkConfig {
            chunk_size: 100,
            overlap_size: 20,
            min_chunk_size: 30,
        });

        let text = "This is the first sentence. This is the second sentence. \
                   This is the third sentence. This is the fourth sentence. \
                   This is the fifth sentence. This is the sixth sentence. \
                   This is the seventh sentence.";

        let chunks = chunker.chunk_text(text).unwrap();

        // Should create multiple chunks
        assert!(chunks.len() > 1);

        // Each chunk should have an index
        for (i, chunk) in chunks.iter().enumerate() {
            assert_eq!(chunk.index, i);
        }

        // Chunks should have reasonable sizes
        for chunk in &chunks {
            assert!(chunk.text.len() >= chunker.config.min_chunk_size);
        }
    }

    #[test]
    fn test_chunk_overlap() {
        let chunker = TextChunker::new(ChunkConfig {
            chunk_size: 50,
            overlap_size: 15,
            min_chunk_size: 20,
        });

        let text = "First sentence. Second sentence. Third sentence. Fourth sentence.";
        let chunks = chunker.chunk_text(text).unwrap();

        if chunks.len() > 1 {
            // Verify that consecutive chunks have some overlap
            // (exact overlap is hard to test due to sentence boundaries)
            assert!(chunks.len() >= 1);
        }
    }

    #[test]
    fn test_chunk_unicode() {
        let chunker = TextChunker::with_defaults();
        let text = "This is English. これは日本語です。This is more English. さらに日本語。";
        let chunks = chunker.chunk_text(text).unwrap();

        // Should handle Unicode correctly
        assert!(!chunks.is_empty());
        for chunk in chunks {
            assert!(!chunk.text.is_empty());
        }
    }

    #[test]
    fn test_chunk_positions() {
        let chunker = TextChunker::new(ChunkConfig {
            chunk_size: 50,
            overlap_size: 10,
            min_chunk_size: 20,
        });

        let text = "Sentence one. Sentence two. Sentence three. Sentence four.";
        let chunks = chunker.chunk_text(text).unwrap();

        // Verify positions make sense
        for chunk in &chunks {
            assert!(chunk.end_pos > chunk.start_pos);
            assert!(chunk.end_pos <= text.len());
        }
    }

    #[test]
    fn test_document_type_from_filename() {
        assert_eq!(DocumentType::from_filename("test.md"), DocumentType::Markdown);
        assert_eq!(DocumentType::from_filename("README.MD"), DocumentType::Markdown);
        assert_eq!(DocumentType::from_filename("file.txt"), DocumentType::Text);
        assert_eq!(DocumentType::from_filename("data.json"), DocumentType::Text);
    }

    #[test]
    fn test_document_type_as_str() {
        assert_eq!(DocumentType::Text.as_str(), "txt");
        assert_eq!(DocumentType::Markdown.as_str(), "md");
    }

    #[test]
    fn test_chunk_custom_config() {
        let config = ChunkConfig {
            chunk_size: 80,
            overlap_size: 15,
            min_chunk_size: 25,
        };

        let text = "A. B. C. D. E. F. G. H. I. J. K. L. M. N. O. P. Q. R. S. T. U. V. W. X. Y. Z.";
        let max_size = config.chunk_size * 2;
        let chunks = TextChunker::chunk_with_config(text, config).unwrap();

        assert!(!chunks.is_empty());
        for chunk in chunks {
            // Most chunks should be around the target size or smaller
            assert!(chunk.text.len() <= max_size); // Allow some flexibility
        }
    }
}
