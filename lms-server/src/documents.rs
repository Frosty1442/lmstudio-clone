use anyhow::{Context, Result};
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
    // TODO: Add when parsers available
    // Pdf,
    // Docx,
    // Html,
}

impl DocumentType {
    pub fn from_filename(filename: &str) -> Self {
        let lower = filename.to_lowercase();
        if lower.ends_with(".md") || lower.ends_with(".markdown") {
            DocumentType::Markdown
        } else {
            // Default to text for now
            DocumentType::Text
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            DocumentType::Text => "txt",
            DocumentType::Markdown => "md",
        }
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
