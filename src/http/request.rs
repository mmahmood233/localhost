use std::collections::HashMap;
use std::str;

#[derive(Debug, Clone)]
pub enum Method {
    GET,
    POST,
    DELETE,
    HEAD,
    PUT,
    OPTIONS,
}

impl Method {
    pub fn from_str(s: &str) -> Option<Method> {
        match s {
            "GET" => Some(Method::GET),
            "POST" => Some(Method::POST),
            "DELETE" => Some(Method::DELETE),
            "HEAD" => Some(Method::HEAD),
            "PUT" => Some(Method::PUT),
            "OPTIONS" => Some(Method::OPTIONS),
            _ => None,
        }
    }
    
    pub fn as_str(&self) -> &'static str {
        match self {
            Method::GET => "GET",
            Method::POST => "POST",
            Method::DELETE => "DELETE",
            Method::HEAD => "HEAD",
            Method::PUT => "PUT",
            Method::OPTIONS => "OPTIONS",
        }
    }
}

#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: Method,
    pub path: String,
    pub version: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
    pub query_string: Option<String>,
}

impl HttpRequest {
    pub fn new() -> Self {
        HttpRequest {
            method: Method::GET,
            path: String::from("/"),
            version: String::from("HTTP/1.1"),
            headers: HashMap::new(),
            body: Vec::new(),
            query_string: None,
        }
    }
    
    pub fn get_header(&self, name: &str) -> Option<&String> {
        // Case-insensitive header lookup
        let name_lower = name.to_lowercase();
        for (key, value) in &self.headers {
            if key.to_lowercase() == name_lower {
                return Some(value);
            }
        }
        None
    }
    
    pub fn content_length(&self) -> Option<usize> {
        self.get_header("content-length")
            .and_then(|v| v.parse().ok())
    }
    
    pub fn is_chunked(&self) -> bool {
        self.get_header("transfer-encoding")
            .map(|v| v.to_lowercase().contains("chunked"))
            .unwrap_or(false)
    }
    
    pub fn connection_keep_alive(&self) -> bool {
        match self.get_header("connection") {
            Some(conn) => {
                let conn_lower = conn.to_lowercase();
                // HTTP/1.1 defaults to keep-alive unless explicitly closed
                if self.version == "HTTP/1.1" {
                    !conn_lower.contains("close")
                } else {
                    // HTTP/1.0 defaults to close unless explicitly keep-alive
                    conn_lower.contains("keep-alive")
                }
            }
            None => {
                // HTTP/1.1 defaults to keep-alive, HTTP/1.0 defaults to close
                self.version == "HTTP/1.1"
            }
        }
    }
    
    pub fn host(&self) -> Option<&String> {
        self.get_header("host")
    }
}
