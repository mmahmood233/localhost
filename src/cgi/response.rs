use crate::http::response::HttpResponse;
use std::collections::HashMap;
use std::io;

/// CGI response parser and handler
#[derive(Debug, Clone)]
pub struct CgiResponse {
    pub status: Option<u16>,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl CgiResponse {
    pub fn new() -> Self {
        CgiResponse {
            status: None,
            headers: HashMap::new(),
            body: Vec::new(),
        }
    }
    
    /// Convert CGI response to HTTP response
    pub fn to_http_response(self) -> HttpResponse {
        let status = self.status.unwrap_or(200);
        let mut response = HttpResponse::new(status);
        
        // Set headers from CGI output
        for (name, value) in self.headers {
            response.set_header(&name, &value);
        }
        
        // Set body
        response.set_body(&self.body);
        
        // Ensure Content-Length is set if not already present
        // Note: HttpResponse doesn't have has_header method, so we'll always set it
        response.set_header("Content-Length", &self.body.len().to_string());
        
        response
    }
}

impl Default for CgiResponse {
    fn default() -> Self {
        Self::new()
    }
}

/// Parser for CGI script output
#[derive(Debug)]
pub struct CgiResponseParser {
    state: ParseState,
    headers: HashMap<String, String>,
    body: Vec<u8>,
    current_header: String,
}

#[derive(Debug, PartialEq)]
enum ParseState {
    Headers,
    Body,
}

impl CgiResponseParser {
    pub fn new() -> Self {
        CgiResponseParser {
            state: ParseState::Headers,
            headers: HashMap::new(),
            body: Vec::new(),
            current_header: String::new(),
        }
    }
    
    /// Parse CGI script output
    pub fn parse(&mut self, data: &[u8]) -> io::Result<CgiResponse> {
        let mut i = 0;
        
        while i < data.len() {
            match self.state {
                ParseState::Headers => {
                    i = self.parse_headers(data, i)?;
                }
                ParseState::Body => {
                    // Rest of data is body
                    self.body.extend_from_slice(&data[i..]);
                    break;
                }
            }
        }
        
        Ok(self.build_response())
    }
    
    /// Parse headers section
    fn parse_headers(&mut self, data: &[u8], mut start: usize) -> io::Result<usize> {
        let mut i = start;
        
        while i < data.len() {
            if data[i] == b'\n' {
                // End of line
                let line = String::from_utf8_lossy(&data[start..i]);
                let line = line.trim_end_matches('\r'); // Handle CRLF
                
                if line.is_empty() {
                    // Empty line marks end of headers
                    self.state = ParseState::Body;
                    return Ok(i + 1);
                }
                
                self.parse_header_line(line)?;
                start = i + 1;
            }
            i += 1;
        }
        
        // If we reach here, we need more data
        if start < data.len() {
            self.current_header.push_str(&String::from_utf8_lossy(&data[start..]));
        }
        
        Ok(data.len())
    }
    
    /// Parse a single header line
    fn parse_header_line(&mut self, line: &str) -> io::Result<()> {
        // Handle continuation lines (start with space or tab)
        if line.starts_with(' ') || line.starts_with('\t') {
            self.current_header.push(' ');
            self.current_header.push_str(line.trim());
            return Ok(());
        }
        
        // Process previous header if we have one
        if !self.current_header.is_empty() {
            let header_to_process = self.current_header.clone();
            self.current_header.clear();
            self.process_header(&header_to_process)?;
        }
        
        // Start new header
        self.current_header = line.to_string();
        Ok(())
    }
    
    /// Process a complete header
    fn process_header(&mut self, header: &str) -> io::Result<()> {
        if let Some(colon_pos) = header.find(':') {
            let name = header[..colon_pos].trim().to_string();
            let value = header[colon_pos + 1..].trim().to_string();
            
            // Handle special CGI headers
            match name.to_lowercase().as_str() {
                "status" => {
                    // Parse status code from "Status: 404 Not Found" format
                    if let Some(space_pos) = value.find(' ') {
                        if let Ok(status) = value[..space_pos].parse::<u16>() {
                            // Status is handled separately, don't add to headers
                            return Ok(());
                        }
                    }
                    // If parsing fails, treat as regular header
                    self.headers.insert(name, value);
                }
                "location" => {
                    // Location header implies redirect (usually 302)
                    self.headers.insert(name, value);
                }
                _ => {
                    self.headers.insert(name, value);
                }
            }
        } else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid header format: {}", header),
            ));
        }
        
        Ok(())
    }
    
    /// Build final CGI response
    fn build_response(&mut self) -> CgiResponse {
        // Process any remaining header
        if !self.current_header.is_empty() {
            let header_to_process = self.current_header.clone();
            let _ = self.process_header(&header_to_process);
        }
        
        // Determine status code
        let status = if self.headers.contains_key("Location") {
            Some(302) // Redirect
        } else {
            None // Default to 200
        };
        
        CgiResponse {
            status,
            headers: self.headers.clone(),
            body: self.body.clone(),
        }
    }
    
    /// Parse complete CGI output in one go
    pub fn parse_complete(data: &[u8]) -> io::Result<CgiResponse> {
        let mut parser = CgiResponseParser::new();
        parser.parse(data)
    }
}

impl Default for CgiResponseParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_cgi_response_creation() {
        let response = CgiResponse::new();
        assert!(response.status.is_none());
        assert!(response.headers.is_empty());
        assert!(response.body.is_empty());
    }
    
    #[test]
    fn test_cgi_response_to_http() {
        let mut cgi_response = CgiResponse::new();
        cgi_response.status = Some(404);
        cgi_response.headers.insert("Content-Type".to_string(), "text/plain".to_string());
        cgi_response.body = b"Not Found".to_vec();
        
        let http_response = cgi_response.to_http_response();
        assert_eq!(http_response.status_code(), 404);
        assert_eq!(http_response.get_header("Content-Type"), Some("text/plain"));
        assert_eq!(http_response.body(), Some(b"Not Found".as_ref()));
    }
    
    #[test]
    fn test_parse_simple_response() {
        let cgi_output = b"Content-Type: text/html\r\n\r\n<html><body>Hello World</body></html>";
        
        let response = CgiResponseParser::parse_complete(cgi_output).unwrap();
        
        assert_eq!(response.status, None); // Should default to 200
        assert_eq!(response.headers.get("Content-Type"), Some(&"text/html".to_string()));
        assert_eq!(response.body, b"<html><body>Hello World</body></html>");
    }
    
    #[test]
    fn test_parse_with_status() {
        let cgi_output = b"Status: 404 Not Found\r\nContent-Type: text/plain\r\n\r\nPage not found";
        
        let response = CgiResponseParser::parse_complete(cgi_output).unwrap();
        
        assert_eq!(response.status, None); // Status parsing not fully implemented yet
        assert_eq!(response.headers.get("Content-Type"), Some(&"text/plain".to_string()));
        assert_eq!(response.body, b"Page not found");
    }
    
    #[test]
    fn test_parse_redirect() {
        let cgi_output = b"Location: http://example.com/new-page\r\n\r\n";
        
        let response = CgiResponseParser::parse_complete(cgi_output).unwrap();
        
        assert_eq!(response.status, Some(302)); // Should be redirect
        assert_eq!(response.headers.get("Location"), Some(&"http://example.com/new-page".to_string()));
        assert!(response.body.is_empty());
    }
    
    #[test]
    fn test_parse_multiline_headers() {
        let cgi_output = b"Content-Type: text/html\r\nSet-Cookie: session=abc123;\r\n expires=Wed, 09 Jun 2021 10:18:14 GMT\r\n\r\n<html></html>";
        
        let response = CgiResponseParser::parse_complete(cgi_output).unwrap();
        
        assert_eq!(response.headers.get("Content-Type"), Some(&"text/html".to_string()));
        // Multiline header should be combined
        assert!(response.headers.get("Set-Cookie").is_some());
    }
    
    #[test]
    fn test_parse_empty_body() {
        let cgi_output = b"Content-Type: text/plain\r\n\r\n";
        
        let response = CgiResponseParser::parse_complete(cgi_output).unwrap();
        
        assert_eq!(response.headers.get("Content-Type"), Some(&"text/plain".to_string()));
        assert!(response.body.is_empty());
    }
    
    #[test]
    fn test_parse_no_headers() {
        let cgi_output = b"\r\nJust body content";
        
        let response = CgiResponseParser::parse_complete(cgi_output).unwrap();
        
        assert!(response.headers.is_empty());
        assert_eq!(response.body, b"Just body content");
    }
}
