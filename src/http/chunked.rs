use std::io::{self, Write};
use std::fmt;

/// Chunked transfer encoding state
#[derive(Debug, Clone, PartialEq)]
pub enum ChunkedState {
    /// Reading chunk size line
    ChunkSize,
    /// Reading chunk data
    ChunkData { size: usize, read: usize },
    /// Reading chunk trailer (CRLF after data)
    ChunkTrailer,
    /// Reading final trailer headers
    Trailer,
    /// Transfer complete
    Complete,
    /// Error state
    Error(String),
}

/// Chunked transfer decoder for incoming requests
#[derive(Debug)]
pub struct ChunkedDecoder {
    /// Current parsing state
    state: ChunkedState,
    /// Buffer for incomplete data
    buffer: Vec<u8>,
    /// Decoded body data
    body: Vec<u8>,
    /// Maximum chunk size allowed
    max_chunk_size: usize,
    /// Maximum total body size
    max_body_size: usize,
    /// Trailer headers
    trailer_headers: Vec<(String, String)>,
}

impl ChunkedDecoder {
    /// Create new chunked decoder
    pub fn new(max_chunk_size: usize, max_body_size: usize) -> Self {
        ChunkedDecoder {
            state: ChunkedState::ChunkSize,
            buffer: Vec::new(),
            body: Vec::new(),
            max_chunk_size,
            max_body_size,
            trailer_headers: Vec::new(),
        }
    }
    
    /// Process incoming data and return number of bytes consumed
    pub fn process(&mut self, data: &[u8]) -> io::Result<usize> {
        let mut consumed = 0;
        let mut pos = 0;
        
        while pos < data.len() {
            match &self.state {
                ChunkedState::ChunkSize => {
                    let (bytes_consumed, new_state) = self.parse_chunk_size(&data[pos..])?;
                    pos += bytes_consumed;
                    consumed += bytes_consumed;
                    self.state = new_state;
                }
                ChunkedState::ChunkData { size, read } => {
                    let remaining = size - read;
                    let available = data.len() - pos;
                    let to_read = remaining.min(available);
                    
                    // Check body size limit
                    if self.body.len() + to_read > self.max_body_size {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "Chunked body exceeds maximum size"
                        ));
                    }
                    
                    self.body.extend_from_slice(&data[pos..pos + to_read]);
                    pos += to_read;
                    consumed += to_read;
                    
                    let new_read = read + to_read;
                    if new_read == *size {
                        self.state = ChunkedState::ChunkTrailer;
                    } else {
                        self.state = ChunkedState::ChunkData { size: *size, read: new_read };
                    }
                }
                ChunkedState::ChunkTrailer => {
                    let (bytes_consumed, new_state) = self.parse_chunk_trailer(&data[pos..])?;
                    pos += bytes_consumed;
                    consumed += bytes_consumed;
                    self.state = new_state;
                }
                ChunkedState::Trailer => {
                    let (bytes_consumed, new_state) = self.parse_trailer(&data[pos..])?;
                    pos += bytes_consumed;
                    consumed += bytes_consumed;
                    self.state = new_state;
                }
                ChunkedState::Complete => {
                    break;
                }
                ChunkedState::Error(ref msg) => {
                    return Err(io::Error::new(io::ErrorKind::InvalidData, msg.clone()));
                }
            }
        }
        
        Ok(consumed)
    }
    
    /// Parse chunk size line
    fn parse_chunk_size(&mut self, data: &[u8]) -> io::Result<(usize, ChunkedState)> {
        self.buffer.extend_from_slice(data);
        
        // Look for CRLF
        if let Some(crlf_pos) = find_crlf(&self.buffer) {
            let line = &self.buffer[..crlf_pos];
            let line_str = std::str::from_utf8(line)
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid UTF-8 in chunk size"))?;
            
            // Parse hex chunk size (ignore extensions after semicolon)
            let size_str = line_str.split(';').next().unwrap_or(line_str).trim();
            let chunk_size = usize::from_str_radix(size_str, 16)
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid chunk size"))?;
            
            // Check chunk size limit
            if chunk_size > self.max_chunk_size {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Chunk size exceeds maximum allowed"
                ));
            }
            
            let consumed = crlf_pos + 2;
            self.buffer.drain(..consumed);
            
            if chunk_size == 0 {
                // Final chunk, move to trailer parsing
                Ok((consumed, ChunkedState::Trailer))
            } else {
                Ok((consumed, ChunkedState::ChunkData { size: chunk_size, read: 0 }))
            }
        } else {
            // Need more data
            Ok((data.len(), ChunkedState::ChunkSize))
        }
    }
    
    /// Parse chunk trailer (CRLF after chunk data)
    fn parse_chunk_trailer(&mut self, data: &[u8]) -> io::Result<(usize, ChunkedState)> {
        self.buffer.extend_from_slice(data);
        
        if self.buffer.len() >= 2 && &self.buffer[..2] == b"\r\n" {
            self.buffer.drain(..2);
            Ok((2, ChunkedState::ChunkSize))
        } else if self.buffer.len() >= 1 {
            if self.buffer[0] == b'\r' {
                // Need more data for LF
                Ok((data.len(), ChunkedState::ChunkTrailer))
            } else {
                Err(io::Error::new(io::ErrorKind::InvalidData, "Expected CRLF after chunk data"))
            }
        } else {
            // Need more data
            Ok((data.len(), ChunkedState::ChunkTrailer))
        }
    }
    
    /// Parse trailer headers
    fn parse_trailer(&mut self, data: &[u8]) -> io::Result<(usize, ChunkedState)> {
        self.buffer.extend_from_slice(data);
        
        // Look for end of headers (empty line)
        if let Some(empty_line_pos) = find_empty_line(&self.buffer) {
            let headers_data = &self.buffer[..empty_line_pos];
            
            // Parse trailer headers
            let headers_str = std::str::from_utf8(headers_data)
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid UTF-8 in trailer headers"))?;
            
            for line in headers_str.lines() {
                if line.trim().is_empty() {
                    continue;
                }
                
                if let Some(colon_pos) = line.find(':') {
                    let name = line[..colon_pos].trim().to_string();
                    let value = line[colon_pos + 1..].trim().to_string();
                    self.trailer_headers.push((name, value));
                }
            }
            
            let consumed = empty_line_pos + 2;
            self.buffer.drain(..consumed);
            Ok((consumed, ChunkedState::Complete))
        } else {
            // Check for immediate end (just CRLF)
            if self.buffer.len() >= 2 && &self.buffer[..2] == b"\r\n" {
                self.buffer.drain(..2);
                Ok((2, ChunkedState::Complete))
            } else {
                // Need more data
                Ok((data.len(), ChunkedState::Trailer))
            }
        }
    }
    
    /// Check if decoding is complete
    pub fn is_complete(&self) -> bool {
        matches!(self.state, ChunkedState::Complete)
    }
    
    /// Check if there's an error
    pub fn is_error(&self) -> bool {
        matches!(self.state, ChunkedState::Error(_))
    }
    
    /// Get decoded body data
    pub fn body(&self) -> &[u8] {
        &self.body
    }
    
    /// Get trailer headers
    pub fn trailer_headers(&self) -> &[(String, String)] {
        &self.trailer_headers
    }
    
    /// Get current state
    pub fn state(&self) -> &ChunkedState {
        &self.state
    }
    
    /// Reset decoder for reuse
    pub fn reset(&mut self) {
        self.state = ChunkedState::ChunkSize;
        self.buffer.clear();
        self.body.clear();
        self.trailer_headers.clear();
    }
    
    /// Get body size
    pub fn body_size(&self) -> usize {
        self.body.len()
    }
}

/// Chunked transfer encoder for outgoing responses
#[derive(Debug)]
pub struct ChunkedEncoder {
    /// Buffer for encoded data
    buffer: Vec<u8>,
    /// Whether encoding is finalized
    finalized: bool,
}

impl ChunkedEncoder {
    /// Create new chunked encoder
    pub fn new() -> Self {
        ChunkedEncoder {
            buffer: Vec::new(),
            finalized: false,
        }
    }
    
    /// Encode a chunk of data
    pub fn encode_chunk(&mut self, data: &[u8]) -> io::Result<()> {
        if self.finalized {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "Encoder already finalized"));
        }
        
        if data.is_empty() {
            return Ok(());
        }
        
        // Write chunk size in hex
        write!(self.buffer, "{:X}\r\n", data.len())?;
        
        // Write chunk data
        self.buffer.extend_from_slice(data);
        
        // Write trailing CRLF
        self.buffer.extend_from_slice(b"\r\n");
        
        Ok(())
    }
    
    /// Finalize chunked encoding with optional trailer headers
    pub fn finalize(&mut self, trailer_headers: Option<&[(String, String)]>) -> io::Result<()> {
        if self.finalized {
            return Ok(());
        }
        
        // Write final chunk (size 0)
        self.buffer.extend_from_slice(b"0\r\n");
        
        // Write trailer headers if provided
        if let Some(headers) = trailer_headers {
            for (name, value) in headers {
                write!(self.buffer, "{}: {}\r\n", name, value)?;
            }
        }
        
        // Write final CRLF
        self.buffer.extend_from_slice(b"\r\n");
        
        self.finalized = true;
        Ok(())
    }
    
    /// Get encoded data and clear buffer
    pub fn take_data(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.buffer)
    }
    
    /// Get encoded data without clearing buffer
    pub fn data(&self) -> &[u8] {
        &self.buffer
    }
    
    /// Check if encoder is finalized
    pub fn is_finalized(&self) -> bool {
        self.finalized
    }
    
    /// Reset encoder for reuse
    pub fn reset(&mut self) {
        self.buffer.clear();
        self.finalized = false;
    }
    
    /// Encode complete data in one go
    pub fn encode_complete(data: &[u8], trailer_headers: Option<&[(String, String)]>) -> io::Result<Vec<u8>> {
        let mut encoder = ChunkedEncoder::new();
        encoder.encode_chunk(data)?;
        encoder.finalize(trailer_headers)?;
        Ok(encoder.take_data())
    }
    
    /// Encode data in chunks
    pub fn encode_chunks(chunks: &[&[u8]], trailer_headers: Option<&[(String, String)]>) -> io::Result<Vec<u8>> {
        let mut encoder = ChunkedEncoder::new();
        
        for chunk in chunks {
            encoder.encode_chunk(chunk)?;
        }
        
        encoder.finalize(trailer_headers)?;
        Ok(encoder.take_data())
    }
}

impl Default for ChunkedEncoder {
    fn default() -> Self {
        Self::new()
    }
}

/// Chunked transfer utilities
pub struct ChunkedUtils;

impl ChunkedUtils {
    /// Check if request/response should use chunked encoding
    pub fn should_use_chunked(content_length: Option<usize>, http_version: &str) -> bool {
        // Only use chunked for HTTP/1.1 when content length is unknown
        http_version == "HTTP/1.1" && content_length.is_none()
    }
    
    /// Parse Transfer-Encoding header
    pub fn parse_transfer_encoding(header_value: &str) -> Vec<String> {
        header_value
            .split(',')
            .map(|s| s.trim().to_lowercase())
            .collect()
    }
    
    /// Check if Transfer-Encoding contains chunked
    pub fn is_chunked_encoding(transfer_encoding: &[String]) -> bool {
        transfer_encoding.iter().any(|encoding| encoding == "chunked")
    }
    
    /// Validate chunk size
    pub fn validate_chunk_size(size: usize, max_size: usize) -> io::Result<()> {
        if size > max_size {
            Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Chunk size {} exceeds maximum {}", size, max_size)
            ))
        } else {
            Ok(())
        }
    }
    
    /// Calculate optimal chunk size for streaming
    pub fn optimal_chunk_size(total_size: Option<usize>) -> usize {
        match total_size {
            Some(size) if size < 1024 => 512,      // Small data: 512 bytes
            Some(size) if size < 64 * 1024 => 4096, // Medium data: 4KB
            Some(size) if size < 1024 * 1024 => 8192, // Large data: 8KB
            _ => 16384, // Very large or unknown: 16KB
        }
    }
}

/// Find CRLF sequence in buffer
fn find_crlf(buffer: &[u8]) -> Option<usize> {
    buffer.windows(2).position(|window| window == b"\r\n")
}

/// Find empty line (CRLF CRLF) in buffer
fn find_empty_line(buffer: &[u8]) -> Option<usize> {
    buffer.windows(4).position(|window| window == b"\r\n\r\n")
}

/// Chunked transfer error types
#[derive(Debug, Clone)]
pub enum ChunkedError {
    /// Invalid chunk size
    InvalidChunkSize(String),
    /// Chunk too large
    ChunkTooLarge(usize, usize),
    /// Body too large
    BodyTooLarge(usize, usize),
    /// Invalid encoding
    InvalidEncoding(String),
    /// Incomplete data
    IncompleteData,
    /// IO error
    IoError(String),
}

impl fmt::Display for ChunkedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChunkedError::InvalidChunkSize(msg) => write!(f, "Invalid chunk size: {}", msg),
            ChunkedError::ChunkTooLarge(size, max) => write!(f, "Chunk size {} exceeds maximum {}", size, max),
            ChunkedError::BodyTooLarge(size, max) => write!(f, "Body size {} exceeds maximum {}", size, max),
            ChunkedError::InvalidEncoding(msg) => write!(f, "Invalid encoding: {}", msg),
            ChunkedError::IncompleteData => write!(f, "Incomplete chunked data"),
            ChunkedError::IoError(msg) => write!(f, "IO error: {}", msg),
        }
    }
}

impl std::error::Error for ChunkedError {}

impl From<io::Error> for ChunkedError {
    fn from(err: io::Error) -> Self {
        ChunkedError::IoError(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_chunked_decoder_simple() {
        let mut decoder = ChunkedDecoder::new(1024, 4096);
        
        // Simple chunked data: "Hello" + "World" + end
        let data = b"5\r\nHello\r\n5\r\nWorld\r\n0\r\n\r\n";
        
        let consumed = decoder.process(data).unwrap();
        assert_eq!(consumed, data.len());
        assert!(decoder.is_complete());
        assert_eq!(decoder.body(), b"HelloWorld");
    }
    
    #[test]
    fn test_chunked_decoder_with_trailer() {
        let mut decoder = ChunkedDecoder::new(1024, 4096);
        
        // Chunked data with trailer headers
        let data = b"4\r\nTest\r\n0\r\nX-Custom: value\r\n\r\n";
        
        let consumed = decoder.process(data).unwrap();
        assert_eq!(consumed, data.len());
        assert!(decoder.is_complete());
        assert_eq!(decoder.body(), b"Test");
        assert_eq!(decoder.trailer_headers().len(), 1);
        assert_eq!(decoder.trailer_headers()[0], ("X-Custom".to_string(), "value".to_string()));
    }
    
    #[test]
    fn test_chunked_decoder_incremental() {
        let mut decoder = ChunkedDecoder::new(1024, 4096);
        
        // Process data incrementally
        let consumed1 = decoder.process(b"5\r\n").unwrap();
        assert_eq!(consumed1, 4);
        assert!(!decoder.is_complete());
        
        let consumed2 = decoder.process(b"Hello\r\n").unwrap();
        assert_eq!(consumed2, 7);
        assert!(!decoder.is_complete());
        
        let consumed3 = decoder.process(b"0\r\n\r\n").unwrap();
        assert_eq!(consumed3, 5);
        assert!(decoder.is_complete());
        assert_eq!(decoder.body(), b"Hello");
    }
    
    #[test]
    fn test_chunked_encoder_simple() {
        let mut encoder = ChunkedEncoder::new();
        
        encoder.encode_chunk(b"Hello").unwrap();
        encoder.encode_chunk(b"World").unwrap();
        encoder.finalize(None).unwrap();
        
        let expected = b"5\r\nHello\r\n5\r\nWorld\r\n0\r\n\r\n";
        assert_eq!(encoder.data(), expected);
    }
    
    #[test]
    fn test_chunked_encoder_with_trailer() {
        let mut encoder = ChunkedEncoder::new();
        
        encoder.encode_chunk(b"Test").unwrap();
        
        let trailer_headers = vec![("X-Custom".to_string(), "value".to_string())];
        encoder.finalize(Some(&trailer_headers)).unwrap();
        
        let expected = b"4\r\nTest\r\n0\r\nX-Custom: value\r\n\r\n";
        assert_eq!(encoder.data(), expected);
    }
    
    #[test]
    fn test_chunked_encoder_complete() {
        let data = b"Hello World";
        let encoded = ChunkedEncoder::encode_complete(data, None).unwrap();
        let expected = b"B\r\nHello World\r\n0\r\n\r\n";
        assert_eq!(encoded, expected);
    }
    
    #[test]
    fn test_chunked_utils() {
        assert!(ChunkedUtils::should_use_chunked(None, "HTTP/1.1"));
        assert!(!ChunkedUtils::should_use_chunked(Some(100), "HTTP/1.1"));
        assert!(!ChunkedUtils::should_use_chunked(None, "HTTP/1.0"));
        
        let encodings = ChunkedUtils::parse_transfer_encoding("chunked, gzip");
        assert_eq!(encodings, vec!["chunked", "gzip"]);
        assert!(ChunkedUtils::is_chunked_encoding(&encodings));
        
        assert_eq!(ChunkedUtils::optimal_chunk_size(Some(500)), 512);
        assert_eq!(ChunkedUtils::optimal_chunk_size(Some(5000)), 4096);
        assert_eq!(ChunkedUtils::optimal_chunk_size(None), 16384);
    }
    
    #[test]
    fn test_find_crlf() {
        assert_eq!(find_crlf(b"test\r\nmore"), Some(4));
        assert_eq!(find_crlf(b"no crlf here"), None);
        assert_eq!(find_crlf(b"\r\n"), Some(0));
    }
    
    #[test]
    fn test_find_empty_line() {
        assert_eq!(find_empty_line(b"header\r\n\r\nbody"), Some(6));
        assert_eq!(find_empty_line(b"no empty line"), None);
        assert_eq!(find_empty_line(b"\r\n\r\n"), Some(0));
    }
}
