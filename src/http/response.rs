use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status_code: u16,
    pub status_text: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
    pub version: String,
}

impl HttpResponse {
    pub fn new(status_code: u16) -> Self {
        let status_text = match status_code {
            200 => "OK",
            204 => "No Content",
            301 => "Moved Permanently",
            302 => "Found",
            400 => "Bad Request",
            403 => "Forbidden",
            404 => "Not Found",
            405 => "Method Not Allowed",
            413 => "Payload Too Large",
            500 => "Internal Server Error",
            _ => "Unknown",
        }.to_string();
        
        let mut headers = HashMap::new();
        headers.insert("Server".to_string(), "Localhost".to_string());
        headers.insert("Date".to_string(), Self::current_date());
        
        HttpResponse {
            status_code,
            status_text,
            headers,
            body: Vec::new(),
            version: "HTTP/1.1".to_string(),
        }
    }
    
    pub fn ok() -> Self {
        Self::new(200)
    }
    
    pub fn not_found() -> Self {
        let mut response = Self::new(404);
        response.set_body(b"404 Not Found");
        response.set_header("Content-Type", "text/plain");
        response
    }
    
    pub fn bad_request() -> Self {
        let mut response = Self::new(400);
        response.set_body(b"400 Bad Request");
        response.set_header("Content-Type", "text/plain");
        response
    }
    
    pub fn method_not_allowed() -> Self {
        let mut response = Self::new(405);
        response.set_body(b"405 Method Not Allowed");
        response.set_header("Content-Type", "text/plain");
        response
    }
    
    pub fn internal_server_error() -> Self {
        let mut response = Self::new(500);
        response.set_body(b"500 Internal Server Error");
        response.set_header("Content-Type", "text/plain");
        response
    }
    
    pub fn set_header(&mut self, name: &str, value: &str) {
        self.headers.insert(name.to_string(), value.to_string());
    }
    
    pub fn set_body(&mut self, body: &[u8]) {
        self.body = body.to_vec();
        self.set_header("Content-Length", &self.body.len().to_string());
    }
    
    pub fn set_body_string(&mut self, body: &str) {
        self.set_body(body.as_bytes());
    }
    
    pub fn set_keep_alive(&mut self, keep_alive: bool) {
        if keep_alive {
            self.set_header("Connection", "keep-alive");
        } else {
            self.set_header("Connection", "close");
        }
    }
    
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut response = Vec::new();
        
        // Status line
        let status_line = format!("{} {} {}\r\n", self.version, self.status_code, self.status_text);
        response.extend_from_slice(status_line.as_bytes());
        
        // Headers
        for (name, value) in &self.headers {
            let header_line = format!("{}: {}\r\n", name, value);
            response.extend_from_slice(header_line.as_bytes());
        }
        
        // Empty line to separate headers from body
        response.extend_from_slice(b"\r\n");
        
        // Body
        response.extend_from_slice(&self.body);
        
        response
    }
    
    fn current_date() -> String {
        // Simple date format - in production would use proper HTTP date formatting
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        format!("timestamp-{}", timestamp)
    }
}

impl fmt::Display for HttpResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {} {}", self.version, self.status_code, self.status_text)
    }
}
