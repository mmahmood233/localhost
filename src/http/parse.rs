use crate::http::request::{HttpRequest, Method};
use std::io::{self, ErrorKind};
use std::str;

#[derive(Debug, Clone)]
pub enum ParseState {
    RequestLine,
    Headers,
    Body,
    Complete,
}

#[derive(Debug)]
pub struct HttpParser {
    state: ParseState,
    buffer: Vec<u8>,
    request: HttpRequest,
    body_bytes_read: usize,
    expected_body_length: Option<usize>,
}

impl HttpParser {
    pub fn new() -> Self {
        HttpParser {
            state: ParseState::RequestLine,
            buffer: Vec::new(),
            request: HttpRequest::new(),
            body_bytes_read: 0,
            expected_body_length: None,
        }
    }
    
    /// Parse incoming data incrementally. Returns Ok(Some(request)) when complete,
    /// Ok(None) when more data is needed, or Err for parse errors.
    pub fn parse(&mut self, data: &[u8]) -> io::Result<Option<HttpRequest>> {
        self.buffer.extend_from_slice(data);
        
        loop {
            match self.state {
                ParseState::RequestLine => {
                    if let Some(request) = self.parse_request_line()? {
                        self.request = request;
                        self.state = ParseState::Headers;
                    } else {
                        return Ok(None); // Need more data
                    }
                }
                ParseState::Headers => {
                    if self.parse_headers()? {
                        // Headers complete, determine if we need to read body
                        self.expected_body_length = self.request.content_length();
                        
                        if self.expected_body_length.is_some() || self.request.is_chunked() {
                            self.state = ParseState::Body;
                        } else {
                            self.state = ParseState::Complete;
                        }
                    } else {
                        return Ok(None); // Need more data
                    }
                }
                ParseState::Body => {
                    if self.parse_body()? {
                        self.state = ParseState::Complete;
                    } else {
                        return Ok(None); // Need more data
                    }
                }
                ParseState::Complete => {
                    return Ok(Some(self.request.clone()));
                }
            }
        }
    }
    
    fn parse_request_line(&mut self) -> io::Result<Option<HttpRequest>> {
        // Look for CRLF to end the request line
        if let Some(pos) = self.find_crlf() {
            let line_bytes = self.buffer.drain(..pos + 2).collect::<Vec<u8>>();
            let line = str::from_utf8(&line_bytes[..pos])
                .map_err(|_| io::Error::new(ErrorKind::InvalidData, "Invalid UTF-8 in request line"))?;
            
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() != 3 {
                return Err(io::Error::new(ErrorKind::InvalidData, "Invalid request line format"));
            }
            
            let method = Method::from_str(parts[0])
                .ok_or_else(|| io::Error::new(ErrorKind::InvalidData, "Invalid HTTP method"))?;
            
            let (path, query_string) = if let Some(query_pos) = parts[1].find('?') {
                let path = parts[1][..query_pos].to_string();
                let query = Some(parts[1][query_pos + 1..].to_string());
                (path, query)
            } else {
                (parts[1].to_string(), None)
            };
            
            let version = parts[2].to_string();
            
            // Validate HTTP version
            if !version.starts_with("HTTP/") {
                return Err(io::Error::new(ErrorKind::InvalidData, "Invalid HTTP version"));
            }
            
            let mut request = HttpRequest::new();
            request.method = method;
            request.path = path;
            request.version = version;
            request.query_string = query_string;
            
            Ok(Some(request))
        } else {
            // Check for oversized request line
            if self.buffer.len() > 8192 {
                return Err(io::Error::new(ErrorKind::InvalidData, "Request line too long"));
            }
            Ok(None)
        }
    }
    
    fn parse_headers(&mut self) -> io::Result<bool> {
        loop {
            if let Some(pos) = self.find_crlf() {
                if pos == 0 {
                    // Empty line indicates end of headers
                    self.buffer.drain(..2); // Remove the CRLF
                    return Ok(true);
                }
                
                let line_bytes = self.buffer.drain(..pos + 2).collect::<Vec<u8>>();
                let line = str::from_utf8(&line_bytes[..pos])
                    .map_err(|_| io::Error::new(ErrorKind::InvalidData, "Invalid UTF-8 in header"))?;
                
                if let Some(colon_pos) = line.find(':') {
                    let name = line[..colon_pos].trim().to_string();
                    let value = line[colon_pos + 1..].trim().to_string();
                    
                    if name.is_empty() {
                        return Err(io::Error::new(ErrorKind::InvalidData, "Empty header name"));
                    }
                    
                    self.request.headers.insert(name, value);
                } else {
                    return Err(io::Error::new(ErrorKind::InvalidData, "Invalid header format"));
                }
            } else {
                // Check for oversized headers
                if self.buffer.len() > 65536 {
                    return Err(io::Error::new(ErrorKind::InvalidData, "Headers too large"));
                }
                return Ok(false); // Need more data
            }
        }
    }
    
    fn parse_body(&mut self) -> io::Result<bool> {
        if self.request.is_chunked() {
            // TODO: Implement chunked transfer encoding in next iteration
            return Err(io::Error::new(ErrorKind::InvalidData, "Chunked encoding not yet supported"));
        }
        
        if let Some(expected_length) = self.expected_body_length {
            let available = self.buffer.len();
            let needed = expected_length - self.body_bytes_read;
            
            if available >= needed {
                // We have all the body data
                let body_data = self.buffer.drain(..needed).collect::<Vec<u8>>();
                self.request.body.extend_from_slice(&body_data);
                self.body_bytes_read += needed;
                return Ok(true);
            } else {
                // Take what we have and wait for more
                let body_data = self.buffer.drain(..).collect::<Vec<u8>>();
                self.request.body.extend_from_slice(&body_data);
                self.body_bytes_read += available;
                
                // Check for oversized body
                if self.body_bytes_read > 10 * 1024 * 1024 { // 10MB limit
                    return Err(io::Error::new(ErrorKind::InvalidData, "Request body too large"));
                }
                
                return Ok(false); // Need more data
            }
        }
        
        // No body expected
        Ok(true)
    }
    
    fn find_crlf(&self) -> Option<usize> {
        self.buffer.windows(2).position(|window| window == b"\r\n")
    }
    
    pub fn is_complete(&self) -> bool {
        matches!(self.state, ParseState::Complete)
    }
    
    pub fn reset(&mut self) {
        self.state = ParseState::RequestLine;
        self.buffer.clear();
        self.request = HttpRequest::new();
        self.body_bytes_read = 0;
        self.expected_body_length = None;
    }
    
    /// Check if the parser is currently reading request body
    pub fn is_reading_body(&self) -> bool {
        matches!(self.state, ParseState::Body)
    }
}
